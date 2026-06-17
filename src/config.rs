//! Runtime configuration for compilation, resolved from CLI args / env / defaults.

use std::time::Duration;

#[derive(Debug, Clone)]
pub struct Config {
    /// TeX Live image to run compilations in.
    pub texlive_image: String,
    /// TeX engine binary (pdflatex / xelatex / lualatex).
    pub engine: String,
    /// Hard wall-clock timeout for a compile.
    pub timeout: Duration,
    /// Container memory limit, in bytes.
    pub memory_bytes: i64,
}

impl Config {
    pub fn from_cli(
        texlive_image: String,
        engine: String,
        timeout_secs: u64,
        memory_bytes: i64,
    ) -> Self {
        Self {
            texlive_image,
            engine,
            timeout: Duration::from_secs(timeout_secs),
            memory_bytes,
        }
    }
}
