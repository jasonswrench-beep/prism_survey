//! Text recognition for pages with no usable text layer (specification, "run OCR locally on a 300 dpi render of the
//! page"). On Windows the recogniser built into Windows 10 and 11 is used: it runs on this computer, needs nothing
//! downloaded, and returns each word with its box. The recogniser sits behind `recognise`, so another engine can take
//! its place (a Mac build will need one).

use crate::pdf::Word;
use crate::render::Picture;

#[derive(Debug, Clone)]
pub struct OcrError(pub String);

impl std::fmt::Display for OcrError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// The words in a picture, with boxes in points (using the picture's scale) and a confidence.
pub fn recognise(pic: &Picture) -> Result<Vec<Word>, OcrError> {
    imp::recognise(pic)
}

/// Whether text recognition is available on this computer.
pub fn available() -> bool {
    imp::available()
}

#[cfg(windows)]
mod imp {
    use super::*;
    use windows::Graphics::Imaging::{BitmapPixelFormat, SoftwareBitmap};
    use windows::Media::Ocr::OcrEngine;
    use windows::Storage::Streams::DataWriter;

    fn err(e: windows::core::Error) -> OcrError {
        OcrError(format!("text recognition failed: {}", e.message()))
    }

    pub fn available() -> bool {
        OcrEngine::TryCreateFromUserProfileLanguages().is_ok()
    }

    pub fn recognise(pic: &Picture) -> Result<Vec<Word>, OcrError> {
        let engine = OcrEngine::TryCreateFromUserProfileLanguages()
            .map_err(|_| OcrError("Windows text recognition is not available; add an OCR language in Windows settings (Time and language)".into()))?;
        // Windows wants blue, green, red, alpha.
        let mut bgra = pic.rgba.clone();
        for px in bgra.as_chunks_mut::<4>().0 {
            px.swap(0, 2);
        }
        let writer = DataWriter::new().map_err(err)?;
        writer.WriteBytes(&bgra).map_err(err)?;
        let buffer = writer.DetachBuffer().map_err(err)?;
        let bitmap = SoftwareBitmap::CreateCopyFromBuffer(
            &buffer,
            BitmapPixelFormat::Bgra8,
            pic.width as i32,
            pic.height as i32,
        )
        .map_err(err)?;
        let result = engine
            .RecognizeAsync(&bitmap)
            .map_err(err)?
            .join()
            .map_err(err)?;
        let mut words = Vec::new();
        for line in result.Lines().map_err(err)? {
            for w in line.Words().map_err(err)? {
                let text = w.Text().map_err(err)?.to_string();
                let r = w.BoundingRect().map_err(err)?;
                let s = pic.scale;
                let (x0, y0) = (f64::from(r.X) / s, f64::from(r.Y) / s);
                let (x1, y1) = (x0 + f64::from(r.Width) / s, y0 + f64::from(r.Height) / s);
                words.push(Word {
                    text,
                    x0,
                    y0,
                    x1,
                    y1,
                    size: (y1 - y0).max(1.0),
                    confidence: 0.9,
                });
            }
        }
        Ok(words)
    }
}

#[cfg(not(windows))]
mod imp {
    use super::*;

    pub fn available() -> bool {
        false
    }

    pub fn recognise(_pic: &Picture) -> Result<Vec<Word>, OcrError> {
        Err(OcrError(
            "text recognition is not available on this system yet".into(),
        ))
    }
}
