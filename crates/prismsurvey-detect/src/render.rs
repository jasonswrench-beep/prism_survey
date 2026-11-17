//! Pictures of PDF pages, drawn with `hayro` (a PDF renderer written in safe Rust), for the confirm screen and for OCR
//! of pages with no usable text layer. Drawing runs on its own thread inside `catch_unwind`, so a damaged file is an
//! error, never a crash (the same approach as PrismQDA's page viewer).

use hayro::hayro_interpret::InterpreterSettings;
use hayro::hayro_syntax::Pdf;
use hayro::vello_cpu::color::palette::css::WHITE;
use hayro::{render, PixmapSettings, RenderCache, RenderSettings};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::Arc;

const STACK_BYTES: usize = 64 * 1024 * 1024;
/// The resolution pages are read at for OCR (specification: "a 300 dpi render of the page").
pub const OCR_DPI: f32 = 300.0;

/// A drawn page: RGBA pixels, row by row, on white.
pub struct Picture {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    /// Pixels per point, so word boxes can be turned back into points.
    pub scale: f64,
}

#[derive(Debug, Clone)]
pub struct RenderError(pub String);

impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

fn guarded<T: Send>(
    work: impl FnOnce() -> Result<T, RenderError> + Send,
) -> Result<T, RenderError> {
    std::thread::scope(|scope| {
        let handle = std::thread::Builder::new()
            .name("pdf-render".into())
            .stack_size(STACK_BYTES)
            .spawn_scoped(scope, move || catch_unwind(AssertUnwindSafe(work)))
            .map_err(|_| {
                RenderError("the computer did not have enough memory to draw this PDF".into())
            })?;
        match handle.join() {
            Ok(Ok(r)) => r,
            _ => Err(RenderError("this page could not be drawn".into())),
        }
    })
}

/// Draws page `page` (from 1) at `dpi` dots per inch.
pub fn render_page(bytes: &[u8], page: u32, dpi: f32) -> Result<Picture, RenderError> {
    guarded(|| {
        let pdf = Pdf::new(Arc::new(bytes.to_vec()))
            .map_err(|_| RenderError("this PDF could not be opened for drawing".into()))?;
        let pages = pdf.pages();
        let p = pages.get(page.saturating_sub(1) as usize).ok_or_else(|| {
            RenderError(format!(
                "this PDF has {} pages, so there is no page {page}",
                pages.len()
            ))
        })?;
        let (w, h) = p.render_dimensions();
        if !(w.is_finite() && h.is_finite() && w >= 1.0 && h >= 1.0) {
            return Err(RenderError(format!("page {page} has no usable size")));
        }
        let mut scale = dpi / 72.0;
        // Keep within what a picture can be (and what the recogniser accepts).
        let longest = w.max(h);
        if longest * scale > 9000.0 {
            scale = 9000.0 / longest;
        }
        let cache = RenderCache::new();
        let pixmap = render(
            p,
            &cache,
            &InterpreterSettings::default(),
            &RenderSettings::default(),
            &PixmapSettings {
                x_scale: scale,
                y_scale: scale,
                bg_color: WHITE,
            },
        );
        let (pw, ph) = (u32::from(pixmap.width()), u32::from(pixmap.height()));
        // The background is opaque white, so premultiplied and straight colour are the same.
        let rgba = pixmap.data_as_u8_slice().to_vec();
        Ok(Picture {
            width: pw,
            height: ph,
            rgba,
            scale: f64::from(scale),
        })
    })
}

/// A PNG of page `page`, `width_px` pixels wide, for the confirm screen.
pub fn render_page_png(bytes: &[u8], page: u32, width_px: u32) -> Result<Vec<u8>, RenderError> {
    let width_px = width_px.clamp(200, 3000);
    let sizes = page_sizes(bytes)?;
    let (w, _) = *sizes
        .get(page.saturating_sub(1) as usize)
        .ok_or_else(|| RenderError(format!("there is no page {page}")))?;
    let pic = render_page(bytes, page, width_px as f32 / w as f32 * 72.0)?;
    let mut out = Vec::new();
    let img = image::RgbaImage::from_raw(pic.width, pic.height, pic.rgba)
        .ok_or_else(|| RenderError("the picture could not be made".into()))?;
    img.write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
        .map_err(|e| RenderError(e.to_string()))?;
    Ok(out)
}

/// Each page's size as drawn, in points.
pub fn page_sizes(bytes: &[u8]) -> Result<Vec<(f64, f64)>, RenderError> {
    guarded(|| {
        let pdf = Pdf::new(Arc::new(bytes.to_vec()))
            .map_err(|_| RenderError("this PDF could not be opened for drawing".into()))?;
        Ok(pdf
            .pages()
            .iter()
            .map(|p| {
                let (w, h) = p.render_dimensions();
                (f64::from(w), f64::from(h))
            })
            .collect())
    })
}
