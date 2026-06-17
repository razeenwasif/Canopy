//! Sandboxed LaTeX compilation.
//!
//! Security model (carried over from the original design — LaTeX is
//! Turing-complete and must never run on the host): each compile runs in a
//! fresh, ephemeral TeX Live container with
//!   * `NetworkMode: "none"`      — no network, no data exfiltration
//!   * a hard memory limit         — default 512 MiB, swap disabled
//!   * a hard wall-clock timeout   — default 40s, enforced by killing the container
//!   * all Linux capabilities dropped and a read-only root filesystem
//! The project directory is bind-mounted read-write as the working dir; the
//! container writes `main.pdf` there, which we read back out, then destroy it.
//!
//! Phase 2 ships the contract + a Docker reachability probe. Phase 3 fills in
//! the container lifecycle (`compile`) using `bollard`.

use std::path::PathBuf;

use anyhow::Result;
use bollard::Docker;

use crate::config::Config;

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
}

/// Probe whether a Docker daemon is reachable. Used to warn the user up front
/// rather than failing only when they first hit compile.
pub async fn docker_available() -> bool {
    match Docker::connect_with_local_defaults() {
        Ok(docker) => docker.ping().await.is_ok(),
        Err(_) => false,
    }
}

/// Compile a project to PDF inside a sandboxed container.
///
/// TODO(phase-3): implement with bollard —
///   1. `Docker::connect_with_local_defaults()`
///   2. create a container from `config.texlive_image` with HostConfig:
///        network_mode = "none", memory = config.memory_bytes,
///        memory_swap = config.memory_bytes (disable swap),
///        cap_drop = ["ALL"], readonly_rootfs = true,
///        binds = ["{work_dir}:/work"], and Cmd
///        ["{engine}", "-interaction=nonstopmode", "-halt-on-error", "{main_tex}"]
///        with WorkingDir "/work".
///   3. start it, `wait` with `tokio::time::timeout(config.timeout, …)`; on
///      elapse, `kill` the container → CompileStatus::Timeout.
///   4. collect logs, read `/work/<stem>.pdf`, then `remove_container(force)`.
pub async fn compile(_config: &Config, _req: CompileRequest) -> Result<CompileOutcome> {
    anyhow::bail!("sandboxed Docker compilation lands in Phase 3")
}
