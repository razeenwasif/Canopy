//! Sandboxed LaTeX compilation (Phase 3).
//!
//! Security model (LaTeX is Turing-complete and must never run on the host):
//! each compile runs in a fresh, ephemeral TeX Live container with
//!   * `NetworkMode: "none"`      — no network, no data exfiltration
//!   * a hard memory limit         — default 512 MiB, swap disabled
//!   * a hard wall-clock timeout   — default 40s, enforced by killing the container
//!   * all Linux capabilities dropped, a read-only root filesystem, a PID cap,
//!     and execution as the host uid:gid (so outputs aren't root-owned)
//! The project directory is bind-mounted read-write as `/work`; the engine
//! writes `<stem>.pdf` there, which the host reads back before we destroy the
//! container.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;

use anyhow::{Context, Result};
use bollard::container::{
    Config, KillContainerOptions, LogOutput, LogsOptions, RemoveContainerOptions,
    WaitContainerOptions,
};
use bollard::models::HostConfig;
use bollard::Docker;
use futures_util::StreamExt;

use crate::config::Config as AppConfig;

#[derive(Debug)]
pub struct CompileRequest {
    /// Host directory containing the LaTeX sources; bind-mounted into /work.
    pub work_dir: PathBuf,
    /// Entry-point file relative to `work_dir`, e.g. "main.tex".
    pub main_tex: String,
}

#[derive(Debug, PartialEq, Eq)]
pub enum CompileStatus {
    Success,
    Failed,
    Timeout,
}

#[derive(Debug)]
pub struct CompileOutcome {
    pub status: CompileStatus,
    /// Captured compiler log (stdout + stderr).
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

/// Compile a project to PDF inside a sandboxed container.
pub async fn compile(config: &AppConfig, req: CompileRequest) -> Result<CompileOutcome> {
    let started = Instant::now();
    let docker = Docker::connect_with_local_defaults()
        .context("connecting to Docker — is the daemon running?")?;

    let work_dir = req
        .work_dir
        .canonicalize()
        .unwrap_or(req.work_dir.clone());
    let bind = format!("{}:/work", work_dir.display());

    // Run the engine, capturing whatever PDF it can even on warnings.
    let cmd = vec![
        config.engine.clone(),
        "-interaction=nonstopmode".to_string(),
        "-halt-on-error".to_string(),
        "-no-shell-escape".to_string(), // belt-and-braces: forbid \write18
        req.main_tex.clone(),
    ];

    let mut tmpfs = HashMap::new();
    tmpfs.insert("/tmp".to_string(), "rw,size=64m".to_string());

    let host_config = HostConfig {
        binds: Some(vec![bind]),
        network_mode: Some("none".to_string()),
        memory: Some(config.memory_bytes),
        memory_swap: Some(config.memory_bytes), // == memory ⇒ swap disabled
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

    // Ensure the container is always cleaned up, even on early return.
    let outcome = run_container(&docker, &id, config, &req, &work_dir, started).await;
    let _ = docker
        .remove_container(
            &id,
            Some(RemoveContainerOptions {
                force: true,
                ..Default::default()
            }),
        )
        .await;
    outcome
}

async fn run_container(
    docker: &Docker,
    id: &str,
    config: &AppConfig,
    req: &CompileRequest,
    work_dir: &PathBuf,
    started: Instant,
) -> Result<CompileOutcome> {
    docker
        .start_container::<String>(id, None)
        .await
        .context("starting compile container")?;

    // Wait for exit, bounded by the hard timeout.
    let mut wait = docker.wait_container(id, None::<WaitContainerOptions<String>>);
    let status = match tokio::time::timeout(config.timeout, wait.next()).await {
        Err(_) => {
            // Timed out: kill the container.
            let _ = docker
                .kill_container(id, None::<KillContainerOptions<String>>)
                .await;
            CompileStatus::Timeout
        }
        Ok(None) => CompileStatus::Failed,
        Ok(Some(Ok(resp))) if resp.status_code == 0 => CompileStatus::Success,
        Ok(Some(_)) => CompileStatus::Failed,
    };

    let log = collect_logs(docker, id).await;

    let pdf_path = if status == CompileStatus::Success {
        let stem = req.main_tex.rsplit_once('.').map(|(s, _)| s).unwrap_or(&req.main_tex);
        let pdf = work_dir.join(format!("{stem}.pdf"));
        pdf.exists().then_some(pdf)
    } else {
        None
    };

    Ok(CompileOutcome {
        status,
        log,
        pdf_path,
        duration_ms: started.elapsed().as_millis(),
    })
}

async fn collect_logs(docker: &Docker, id: &str) -> String {
    let mut stream = docker.logs(
        id,
        Some(LogsOptions::<String> {
            stdout: true,
            stderr: true,
            ..Default::default()
        }),
    );
    let mut out = String::new();
    while let Some(Ok(chunk)) = stream.next().await {
        match chunk {
            LogOutput::StdOut { message }
            | LogOutput::StdErr { message }
            | LogOutput::Console { message } => {
                out.push_str(&String::from_utf8_lossy(&message));
            }
            _ => {}
        }
        if out.len() > 64 * 1024 {
            out.push_str("\n…(log truncated)…");
            break;
        }
    }
    out
}

/// `uid:gid` of the current process, for the container `User` field.
fn host_uid_gid() -> String {
    // Safe: getuid/getgid never fail and have no side effects.
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
