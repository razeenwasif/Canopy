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
    /// Ollama model for the AI assistant.
    pub ai_model: String,
    /// Ollama host (loopback HTTP).
    pub ollama_host: String,
    /// Remove LaTeX auxiliary files after a successful compile (keep the PDF).
    pub clean_artifacts: bool,
}

impl Config {
    #[allow(clippy::too_many_arguments)]
    pub fn from_cli(
        texlive_image: String,
        engine: String,
        timeout_secs: u64,
        memory_bytes: i64,
        ai_model: String,
        ollama_host: String,
        clean_artifacts: bool,
    ) -> Self {
        Self {
            texlive_image,
            engine,
            timeout: Duration::from_secs(timeout_secs),
            memory_bytes,
            ai_model,
            ollama_host,
            clean_artifacts,
        }
    }
}
