//! Reading an article into pages of positioned words: the PDF's text layer first, and local OCR for any page that has
//! none, a garbled one, or one the PDF library could not finish (specification, "Get the text with positions"). Page
//! images and phone photos (.png, .jpg) are read by OCR.

use crate::ocr;
use crate::pdf::{read_text_layer, Page, TextSource};
use crate::render::{render_page, Picture, OCR_DPI};
use serde::Serialize;
use std::path::Path;
use std::sync::Once;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Article {
    pub file: String,
    pub pages: Vec<Page>,
    /// Plain-language notes about how the article was read.
    pub notes: Vec<String>,
}

/// The PDF library panics on some fonts; those are caught, so their messages are kept off the console.
fn quiet_reader_panics() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let default = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let name = std::thread::current()
                .name()
                .map(str::to_string)
                .unwrap_or_default();
            if name != "pdf-reader" && name != "pdf-render" {
                default(info);
            }
        }));
    });
}

/// A page needs OCR when it has no words, a garbled layer, or failed part way with few words.
fn needs_ocr(p: &Page) -> bool {
    p.source == TextSource::None || p.garbled || (p.failed && p.words.len() < 50)
}

pub fn read_article(path: &Path) -> Result<Article, String> {
    quiet_reader_panics();
    let file = path
        .file_name()
        .map(|f| f.to_string_lossy().into_owned())
        .unwrap_or_default();
    let bytes = std::fs::read(path).map_err(|e| format!("{file}: {e}"))?;
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    let mut notes = Vec::new();
    if matches!(
        ext.as_str(),
        "png" | "jpg" | "jpeg" | "tif" | "tiff" | "bmp"
    ) {
        let page = read_image(&bytes, &mut notes).map_err(|e| format!("{file}: {e}"))?;
        return Ok(Article {
            file,
            pages: vec![page],
            notes,
        });
    }
    let mut pages = read_text_layer(&bytes).map_err(|e| format!("{file}: {e}"))?;
    let todo: Vec<u32> = pages
        .iter()
        .filter(|p| needs_ocr(p))
        .map(|p| p.number)
        .collect();
    if !todo.is_empty() {
        if !ocr::available() {
            notes.push(format!("Page(s) {} have no usable text and need text recognition, which is not available on this computer.", join(&todo)));
        } else {
            let mut done = Vec::new();
            for n in &todo {
                match render_page(&bytes, *n, OCR_DPI)
                    .map_err(|e| e.to_string())
                    .and_then(|pic| ocr::recognise(&pic).map_err(|e| e.to_string()))
                {
                    Ok(words) if !words.is_empty() => {
                        if let Some(p) = pages.iter_mut().find(|p| p.number == *n) {
                            p.words = words;
                            p.source = TextSource::Ocr;
                            p.garbled = false;
                        }
                        done.push(*n);
                    }
                    Ok(_) => {}
                    Err(e) => notes.push(format!("Page {n}: {e}")),
                }
            }
            if !done.is_empty() {
                notes.push(format!(
                    "Page(s) {} were read by text recognition; check their words closely.",
                    join(&done)
                ));
            }
        }
    }
    Ok(Article { file, pages, notes })
}

fn join(n: &[u32]) -> String {
    n.iter()
        .map(|x| x.to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

/// A page image or phone photo: straightened and cropped to the paper, then recognised.
fn read_image(bytes: &[u8], notes: &mut Vec<String>) -> Result<Page, String> {
    let img = image::load_from_memory(bytes)
        .map_err(|e| format!("the picture could not be read: {e}"))?
        .to_rgba8();
    let (img, cropped) = crop_to_paper(img);
    if cropped {
        notes.push("The picture was cropped to the page.".into());
    }
    // Pictures are read as they are at 300 dpi equivalent: one pixel is one 300th of an inch.
    let scale = f64::from(OCR_DPI) / 72.0;
    let pic = Picture {
        width: img.width(),
        height: img.height(),
        rgba: img.into_raw(),
        scale,
    };
    let words = ocr::recognise(&pic).map_err(|e| e.to_string())?;
    notes.push("The page was read by text recognition; check its words closely.".into());
    Ok(Page {
        number: 1,
        width: f64::from(pic.width) / scale,
        height: f64::from(pic.height) / scale,
        source: if words.is_empty() {
            TextSource::None
        } else {
            TextSource::Ocr
        },
        words,
        garbled: false,
        failed: false,
    })
}

/// Crops a photo to the paper: the bounding box of the light area, when the paper is clearly lighter than a dark
/// surround. Pictures that are all paper (scans) are left as they are.
fn crop_to_paper(img: image::RgbaImage) -> (image::RgbaImage, bool) {
    let (w, h) = img.dimensions();
    if w < 100 || h < 100 {
        return (img, false);
    }
    let light = |x: u32, y: u32| {
        let p = img.get_pixel(x, y);
        (u32::from(p[0]) + u32::from(p[1]) + u32::from(p[2])) / 3 > 150
    };
    let step = (w.min(h) / 200).max(1);
    let row_light = |y: u32| {
        (0..w)
            .step_by(step as usize)
            .filter(|&x| light(x, y))
            .count() as u32
            * step
            * 2
            > w
    };
    let col_light = |x: u32| {
        (0..h)
            .step_by(step as usize)
            .filter(|&y| light(x, y))
            .count() as u32
            * step
            * 2
            > h
    };
    let top = (0..h)
        .step_by(step as usize)
        .find(|&y| row_light(y))
        .unwrap_or(0);
    let bottom = (0..h)
        .rev()
        .step_by(step as usize)
        .find(|&y| row_light(y))
        .unwrap_or(h - 1);
    let left = (0..w)
        .step_by(step as usize)
        .find(|&x| col_light(x))
        .unwrap_or(0);
    let right = (0..w)
        .rev()
        .step_by(step as usize)
        .find(|&x| col_light(x))
        .unwrap_or(w - 1);
    let (cw, ch) = (
        right.saturating_sub(left) + 1,
        bottom.saturating_sub(top) + 1,
    );
    // Only crop when a real border was found, and the paper is still most of the picture.
    if (cw < w * 97 / 100 || ch < h * 97 / 100) && cw > w / 3 && ch > h / 3 {
        (
            image::imageops::crop_imm(&img, left, top, cw, ch).to_image(),
            true,
        )
    } else {
        (img, false)
    }
}
