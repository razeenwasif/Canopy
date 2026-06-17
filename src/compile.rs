//! Sandboxed LaTeX compilation (Phase 3).
//!
//! Security model (LaTeX is Turing-complete and must never run on the host):
//! each compile runs in a fresh, ephemeral TeX Live container with
//!   * `NetworkMode: "none"`      — no network, no data exfiltration
//!   * a hard memory limit         — default 512 MiB, swap disabled
//!   * a hard wall-clock timeout   — default 40s (across all passes), enforced
//!     by killing the container
//!   * all Linux capabilities dropped, a read-only root filesystem, a PID cap,
//!     and execution as the host uid:gid (so outputs aren't root-owned)
//!
//! The engine is re-run until references/TOC stabilize (the log stops asking to
//! "rerun"), up to `MAX_PASSES`, so cross-references resolve in one compile.
//! Afterwards, auxiliary files are cleaned up, leaving just the PDF and sources.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use bollard::container::{
    Config, KillContainerOptions, LogOutput, LogsOptions, RemoveContainerOptions,
    WaitContainerOptions,
};
use bollard::models::HostConfig;
use bollard::Docker;
use futures_util::StreamExt;

use crate::config::Config as AppConfig;

/// Upper bound on engine passes (cross-references usually settle within 3).
const MAX_PASSES: u32 = 4;

/// Filename suffixes considered LaTeX build artifacts (removed on cleanup).
// Note: `.xmpdata` is intentionally NOT here — it's a pdfx/hyperxmp *source*
// file (PDF metadata) the user authors, not a build artifact.
const AUX_SUFFIXES: &[&str] = &[
    ".abs", ".aux", ".bbl", ".blg", ".brf", ".bcf", ".fdb_latexmk", ".fls", ".idx", ".ilg",
    ".ind", ".lof", ".log", ".lot", ".lol", ".loa", ".nav", ".out", ".run.xml", ".snm",
    ".synctex.gz", ".synctex", ".toc", ".vrb", ".xdy", ".glo", ".gls", ".glg", ".acn", ".acr",
    ".alg", ".ist", ".thm", ".spl", ".fff", ".ttt", ".los", ".soc", ".maf", ".mtc",
];

#[derive(Debug)]
pub struct CompileRequest {
    /// Host directory containing the LaTeX sources; bind-mounted into /work.
    pub work_dir: PathBuf,
    /// Entry-point file relative to `work_dir`, e.g. "main.tex".
    pub main_tex: String,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum CompileStatus {
    Success,
    Failed,
    Timeout,
}

#[derive(Debug)]
pub struct CompileOutcome {
    pub status: CompileStatus,
    /// Captured compiler log (stdout + stderr) from the final pass.
    pub log: String,
    /// Host path to the produced PDF, present on success.
    pub pdf_path: Option<PathBuf>,
    /// Wall-clock duration in milliseconds.
    pub duration_ms: u128,
}

/// Probe whether a Docker daemon is reachable.
pub async fn docker_available() -> bool {
    match Docker::connect_with_local_defaults() {
        Ok(docker) => docker.ping().await.is_ok(),
        Err(_) => false,
    }
}

/// Compile a project to PDF inside a sandboxed container, re-running the engine
/// until references stabilize, then cleaning auxiliary files.
pub async fn compile(config: &AppConfig, req: CompileRequest) -> Result<CompileOutcome> {
    let started = Instant::now();
    let docker = Docker::connect_with_local_defaults()
        .context("connecting to Docker — is the daemon running?")?;

    let work_dir = req.work_dir.canonicalize().unwrap_or(req.work_dir.clone());
    let bind = format!("{}:/work", work_dir.display());

    let mut status = CompileStatus::Failed;
    let mut log = String::new();
    for _ in 0..MAX_PASSES {
        // The whole compile shares one hard timeout.
        let remaining = config.timeout.saturating_sub(started.elapsed());
        if remaining.is_zero() {
            status = CompileStatus::Timeout;
            break;
        }
        let (pass_status, pass_log) = run_once(&docker, config, &req, &bind, remaining).await?;
        status = pass_status;
        log = pass_log;
        // Only a clean run is worth re-running; stop otherwise.
        if status != CompileStatus::Success || !needs_rerun(&work_dir, &req.main_tex) {
            break;
        }
    }

    let pdf_path = if status == CompileStatus::Success {
        let stem = req.main_tex.rsplit_once('.').map_or(req.main_tex.as_str(), |(s, _)| s);
        let pdf = work_dir.join(format!("{stem}.pdf"));
        pdf.exists().then_some(pdf)
    } else {
        None
    };

    // Tidy up auxiliary files on success (keep them on failure for debugging).
    if config.clean_artifacts && status == CompileStatus::Success {
        clean_artifacts(&work_dir);
    }

    Ok(CompileOutcome {
        status,
        log,
        pdf_path,
        duration_ms: started.elapsed().as_millis(),
    })
}

/// One engine pass: create → start → wait (bounded) → collect log → remove.
async fn run_once(
    docker: &Docker,
    config: &AppConfig,
    req: &CompileRequest,
    bind: &str,
    timeout: Duration,
) -> Result<(CompileStatus, String)> {
    let cmd = vec![
        config.engine.clone(),
        "-interaction=nonstopmode".to_string(),
        "-halt-on-error".to_string(),
        "-no-shell-escape".to_string(), // forbid \write18
        req.main_tex.clone(),
    ];

    let mut tmpfs = HashMap::new();
    tmpfs.insert("/tmp".to_string(), "rw,size=64m".to_string());

    let host_config = HostConfig {
        binds: Some(vec![bind.to_string()]),
        network_mode: Some("none".to_string()),
        memory: Some(config.memory_bytes),
        memory_swap: Some(config.memory_bytes),
        cap_drop: Some(vec!["ALL".to_string()]),
        readonly_rootfs: Some(true),
        tmpfs: Some(tmpfs),
        pids_limit: Some(256),
        auto_remove: Some(false),
        ..Default::default()
    };

    let container_config = Config {
        image: Some(config.texlive_image.clone()),
        cmd: Some(cmd),
        working_dir: Some("/work".to_string()),
        env: Some(vec!["HOME=/work".to_string(), "TEXMFVAR=/tmp/texmf".to_string()]),
        user: Some(host_uid_gid()),
        host_config: Some(host_config),
        ..Default::default()
    };

    let create = docker
        .create_container::<String, String>(None, container_config)
        .await
        .map_err(|e| anyhow::anyhow!(image_hint(&config.texlive_image, e)))?;
    let id = create.id;

    let result = drive_container(docker, &id, timeout).await;
    let _ = docker
        .remove_container(&id, Some(RemoveContainerOptions { force: true, ..Default::default() }))
        .await;
    result
}

async fn drive_container(
    docker: &Docker,
    id: &str,
    timeout: Duration,
) -> Result<(CompileStatus, String)> {
    docker
        .start_container::<String>(id, None)
        .await
        .context("starting compile container")?;

    let mut wait = docker.wait_container(id, None::<WaitContainerOptions<String>>);
    let status = match tokio::time::timeout(timeout, wait.next()).await {
        Err(_) => {
            let _ = docker.kill_container(id, None::<KillContainerOptions<String>>).await;
            CompileStatus::Timeout
        }
        Ok(None) => CompileStatus::Failed,
        Ok(Some(Ok(resp))) if resp.status_code == 0 => CompileStatus::Success,
        Ok(Some(_)) => CompileStatus::Failed,
    };

    Ok((status, collect_logs(docker, id).await))
}

async fn collect_logs(docker: &Docker, id: &str) -> String {
    let mut stream = docker.logs(
        id,
        Some(LogsOptions::<String> { stdout: true, stderr: true, ..Default::default() }),
    );
    let mut out = String::new();
    while let Some(Ok(chunk)) = stream.next().await {
        match chunk {
            LogOutput::StdOut { message }
            | LogOutput::StdErr { message }
            | LogOutput::Console { message } => out.push_str(&String::from_utf8_lossy(&message)),
            _ => {}
        }
        if out.len() > 64 * 1024 {
            out.push_str("\n…(log truncated)…");
            break;
        }
    }
    out
}

/// Does the `.log` ask for another pass (unresolved refs / changed labels)?
fn needs_rerun(work_dir: &Path, main_tex: &str) -> bool {
    let stem = main_tex.rsplit_once('.').map_or(main_tex, |(s, _)| s);
    let log_path = work_dir.join(format!("{stem}.log"));
    match std::fs::read_to_string(&log_path) {
        Ok(text) => text.to_lowercase().contains("rerun"),
        Err(_) => false,
    }
}

/// Remove LaTeX auxiliary files from the project directory, keeping the PDF and
/// all source files. Only the top level is scanned.
pub fn clean_artifacts(work_dir: &Path) -> usize {
    let mut removed = 0;
    let Ok(entries) = std::fs::read_dir(work_dir) else { return 0 };
    for entry in entries.flatten() {
        if !entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_lowercase();
        if AUX_SUFFIXES.iter().any(|suf| name.ends_with(suf)) {
            if std::fs::remove_file(entry.path()).is_ok() {
                removed += 1;
            }
        }
    }
    removed
}

/// `uid:gid` of the current process, for the container `User` field.
fn host_uid_gid() -> String {
    let (uid, gid) = unsafe { (libc::getuid(), libc::getgid()) };
    format!("{uid}:{gid}")
}

/// Turn a "no such image" create error into an actionable hint.
fn image_hint(image: &str, err: bollard::errors::Error) -> String {
    let msg = err.to_string();
    if msg.contains("404") || msg.to_lowercase().contains("no such image") {
        format!("image \"{image}\" not found — run `docker pull {image}` first")
    } else {
        format!("creating compile container: {msg}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleans_aux_keeps_sources_and_pdf() {
        let dir = std::env::temp_dir().join(format!("canopy-clean-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let make = |n: &str| std::fs::write(dir.join(n), b"x").unwrap();

        // Sources + output to keep (note: .xmpdata is a pdfx source file).
        for keep in ["main.tex", "main.pdf", "refs.bib", "fig.png", "style.sty", "main.xmpdata"] {
            make(keep);
        }
        // Artifacts to remove (including .abs and compound suffixes).
        for junk in ["main.aux", "main.log", "main.out", "main.toc", "main.synctex.gz", "main.run.xml", "main.fdb_latexmk", "main.abs"] {
            make(junk);
        }

        let removed = clean_artifacts(&dir);
        assert_eq!(removed, 8);

        let remaining: Vec<String> = std::fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        assert!(remaining.contains(&"main.tex".to_string()));
        assert!(remaining.contains(&"main.pdf".to_string()));
        assert!(remaining.contains(&"refs.bib".to_string()));
        assert!(remaining.contains(&"main.xmpdata".to_string())); // source, kept
        assert!(!remaining.iter().any(|n| n.ends_with(".abs")));
        assert!(!remaining.iter().any(|n| n.ends_with(".aux")));
        assert!(!remaining.iter().any(|n| n.ends_with(".synctex.gz")));
        assert_eq!(remaining.len(), 6);

        std::fs::remove_dir_all(&dir).ok();
    }
}
