//! PDF preview support.
//!
//! Rasterizes a PDF page to an image by shelling out to poppler's `pdftoppm`
//! (commonly installed; no extra shared library needed). `ratatui-image` then
//! displays the image inline via the best terminal graphics protocol available.

use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result};
use image::DynamicImage;

/// Rasterize one page (1-indexed) of a PDF to an image at the given DPI.
/// `ratatui-image` downscales it to the preview pane on render, so a moderate
/// DPI is plenty.
pub fn rasterize(pdf: &Path, page: u32, dpi: u32) -> Result<DynamicImage> {
    let out_prefix = std::env::temp_dir().join("canopy-preview");
    let out_png = out_prefix.with_extension("png");

    let status = Command::new("pdftoppm")
        .args(["-png", "-singlefile", "-r", &dpi.to_string()])
        .args(["-f", &page.to_string(), "-l", &page.to_string()])
        .arg(pdf)
        .arg(&out_prefix) // pdftoppm appends ".png" with -singlefile
        .status()
        .context("running pdftoppm — is poppler-utils installed?")?;

    if !status.success() {
        anyhow::bail!("pdftoppm failed (exit {:?})", status.code());
    }

    let img = image::open(&out_png)
        .with_context(|| format!("reading rasterized page {}", out_png.display()))?;
    let _ = std::fs::remove_file(&out_png);
    Ok(img)
}
