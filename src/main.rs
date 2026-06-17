//! Canopy — a local terminal LaTeX editor.
//!
//! A single self-contained binary: edit `.tex` files in your terminal, compile
//! them inside an ephemeral, network-isolated Docker container, and preview the
//! resulting PDF inline. No server, no database, no network — just you, your
//! files, and a sandboxed TeX Live container.
//!
//! Phase 2 delivers the editor itself: a real text buffer (ropey), cursor
//! movement, editing, scrolling, save, and a file browser. Sandboxed Docker
//! compilation (`compile`) and inline PDF preview (`pdf`) are scaffolded with
//! `TODO(phase-N)` markers and filled in next.

mod app;
mod compile;
mod config;
mod editor;
mod fs;
mod pdf;
mod ui;

use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

use crate::app::App;
use crate::config::Config;

#[derive(Debug, Parser)]
#[command(name = "canopy", version, about = "Local terminal LaTeX editor")]
struct Cli {
    /// File to open, or a directory to browse. Defaults to the current directory.
    path: Option<PathBuf>,

    /// TeX Live Docker image used for compilation.
    #[arg(long, env = "CANOPY_TEXLIVE_IMAGE", default_value = "texlive/texlive:latest")]
    texlive_image: String,

    /// TeX engine to run.
    #[arg(long, env = "CANOPY_ENGINE", default_value = "pdflatex")]
    engine: String,

    /// Hard compile timeout in seconds.
    #[arg(long, env = "CANOPY_COMPILE_TIMEOUT", default_value_t = 40)]
    timeout_secs: u64,

    /// Container memory limit (bytes). Defaults to 512 MiB.
    #[arg(long, env = "CANOPY_COMPILE_MEMORY", default_value_t = 512 * 1024 * 1024)]
    memory_bytes: i64,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = Config::from_cli(
        cli.texlive_image,
        cli.engine,
        cli.timeout_secs,
        cli.memory_bytes,
    );

    let start_path = match cli.path {
        Some(p) => p,
        None => std::env::current_dir()?,
    };

    // Enter raw mode + alternate screen. `ratatui::init` installs a panic hook
    // that restores the terminal on crash.
    let terminal = ratatui::init();

    let mut app = App::new(config, start_path)?;
    let result = app.run(terminal).await;

    ratatui::restore();
    result
}
