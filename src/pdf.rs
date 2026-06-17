//! PDF preview support.
//!
//! Rasterizes PDF pages with poppler's `pdftoppm` (no extra shared library
//! needed) and exposes a `PdfView` that supports page navigation, zoom, and
//! panning. `ratatui-image` displays the (cropped, zoomed) page inline.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};
use image::DynamicImage;
use ratatui::layout::Rect;
use ratatui_image::picker::Picker;
use ratatui_image::protocol::StatefulProtocol;

/// DPI the page is rasterized at. Zoom crops into this raster, so a moderately
/// high DPI keeps zoomed-in views reasonably sharp.
const RASTER_DPI: u32 = 150;
/// Terminal cell aspect (width:height in px) used to keep the crop undistorted.
const CELL_ASPECT: (u32, u32) = (1, 2);

/// Rasterize one page (1-indexed) of a PDF to an image at the given DPI.
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

/// Page count via `pdfinfo`; falls back to 1 if unavailable.
pub fn page_count(pdf: &Path) -> u32 {
    Command::new("pdfinfo")
        .arg(pdf)
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| {
            s.lines()
                .find_map(|l| l.strip_prefix("Pages:").and_then(|n| n.trim().parse::<u32>().ok()))
        })
        .unwrap_or(1)
        .max(1)
}

/// A navigable, zoomable view of one compiled PDF.
pub struct PdfView {
    path: PathBuf,
    page: u32,
    page_count: u32,
    image: DynamicImage,
    iw: u32,
    ih: u32,
    /// 1.0 = fit page width to the pane; higher = zoomed in.
    zoom: f32,
    scroll_x: u32,
    scroll_y: u32,
    protocol: Option<StatefulProtocol>,
    /// (page, zoom×1000, scroll_x, scroll_y, cols, rows) the cached protocol is for.
    cache_key: Option<(u32, u32, u32, u32, u16, u16)>,
}

impl PdfView {
    pub fn open(path: &Path) -> Result<Self> {
        let page_count = page_count(path);
        let image = rasterize(path, 1, RASTER_DPI)?;
        let (iw, ih) = (image.width(), image.height());
        Ok(Self {
            path: path.to_path_buf(),
            page: 1,
            page_count,
            image,
            iw,
            ih,
            zoom: 1.0,
            scroll_x: 0,
            scroll_y: 0,
            protocol: None,
            cache_key: None,
        })
    }

    pub fn status(&self) -> String {
        format!("p{}/{}  {:.0}%", self.page, self.page_count, self.zoom * 100.0)
    }

    fn load_page(&mut self) {
        if let Ok(img) = rasterize(&self.path, self.page, RASTER_DPI) {
            self.iw = img.width();
            self.ih = img.height();
            self.image = img;
            self.scroll_x = 0;
            self.scroll_y = 0;
            self.cache_key = None;
        }
    }

    pub fn next_page(&mut self) {
        if self.page < self.page_count {
            self.page += 1;
            self.load_page();
        }
    }
    pub fn prev_page(&mut self) {
        if self.page > 1 {
            self.page -= 1;
            self.load_page();
        }
    }
    pub fn first_page(&mut self) {
        if self.page != 1 {
            self.page = 1;
            self.load_page();
        }
    }
    pub fn last_page(&mut self) {
        if self.page != self.page_count {
            self.page = self.page_count;
            self.load_page();
        }
    }

    pub fn zoom_in(&mut self) {
        self.zoom = (self.zoom * 1.25).min(6.0);
        self.cache_key = None;
    }
    pub fn zoom_out(&mut self) {
        self.zoom = (self.zoom / 1.25).max(1.0);
        self.cache_key = None;
    }
    pub fn reset_view(&mut self) {
        self.zoom = 1.0;
        self.scroll_x = 0;
        self.scroll_y = 0;
        self.cache_key = None;
    }

    /// Pan by step units (positive = right/down). Clamped at render time.
    pub fn scroll(&mut self, dx: i32, dy: i32) {
        let hstep = (self.iw / 20).max(8) as i32;
        let vstep = (self.ih / 20).max(8) as i32;
        self.scroll_x = (self.scroll_x as i32 + dx * hstep).max(0) as u32;
        self.scroll_y = (self.scroll_y as i32 + dy * vstep).max(0) as u32;
        self.cache_key = None;
    }

    /// Build (or reuse) the protocol showing the current viewport in `area`.
    pub fn protocol_for(
        &mut self,
        picker: &mut Picker,
        area: Rect,
    ) -> Option<&mut StatefulProtocol> {
        if area.width == 0 || area.height == 0 {
            return None;
        }
        let pane_px_w = area.width as u32 * CELL_ASPECT.0;
        let pane_px_h = area.height as u32 * CELL_ASPECT.1;

        // Crop size in source pixels: zoom is relative to fit-width.
        let crop_w = ((self.iw as f32 / self.zoom).round() as u32).clamp(1, self.iw);
        let crop_h = (((pane_px_h as f32 / pane_px_w as f32) * crop_w as f32).round() as u32)
            .clamp(1, self.ih);

        // Clamp scroll to keep the crop inside the page.
        self.scroll_x = self.scroll_x.min(self.iw - crop_w);
        self.scroll_y = self.scroll_y.min(self.ih - crop_h);

        let key = (
            self.page,
            (self.zoom * 1000.0) as u32,
            self.scroll_x,
            self.scroll_y,
            area.width,
            area.height,
        );
        if self.cache_key != Some(key) {
            let sub = self.image.crop_imm(self.scroll_x, self.scroll_y, crop_w, crop_h);
            self.protocol = Some(picker.new_resize_protocol(sub));
            self.cache_key = Some(key);
        }
        self.protocol.as_mut()
    }
}
