//! PDF preview support.
//!
//! The compiler writes a PDF to the project directory; we rasterize the
//! requested page to an RGBA image with `pdfium-render` and hand it to
//! `ratatui-image`, which picks the best terminal graphics protocol available
//! (Kitty → iTerm2 → Sixel → halfblocks fallback).

use std::path::Path;

use anyhow::Result;
use image::DynamicImage;

/// Rasterize one page (0-indexed) of a PDF file to an image at the given target
/// width in pixels (height follows the page aspect ratio).
///
/// TODO(phase-4): implement with `pdfium_render::prelude::*`:
///   Pdfium::new(Pdfium::bind_to_system_library()?)
///     .load_pdf_from_file(path, None)?
///     .pages().get(page)?
///     .render_with_config(&PdfRenderConfig::new().set_target_width(width))?
///     .as_image()
pub fn rasterize_page(_path: &Path, _page: u16, _target_width: u16) -> Result<DynamicImage> {
    anyhow::bail!("PDF rasterization lands in Phase 4")
}
