//! The words of each page of a PDF, with where they sit (specification, "Get the text with positions").
//!
//! A PDF stores glyphs placed at positions, not text. The glyphs are read with the `pdf-extract` crate through its
//! `OutputDev` hook and grouped into words here, each with its box on the page. Positions are in points (1/72 inch)
//! with the origin at the top left of the page, the way a reader sees it, so the reading order, the confirm screen's
//! outlines and the OCR fallback all use one system. The stream cleaning and glyph handling are adapted from PrismQDA's
//! PDF importer (same author and licence).
//!
//! Reading runs on its own thread with a large stack and every page inside `catch_unwind`, so a damaged file is reported
//! as a problem, never a crash.

use pdf_extract::content::{Content, Operation};
use pdf_extract::{
    Dictionary, Document, MediaBox, Object, ObjectId, OutputDev, OutputError, Stream, Transform,
};
use serde::{Deserialize, Serialize};
use std::panic::{catch_unwind, AssertUnwindSafe};

const READER_STACK_BYTES: usize = 64 * 1024 * 1024;

/// One word on a page.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Word {
    pub text: String,
    /// Left, top, right, bottom in points from the page's top left.
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
    /// The type size in points.
    pub size: f64,
    /// 1.0 for the text layer; the recogniser's confidence for OCR.
    pub confidence: f64,
}

impl Word {
    pub fn cx(&self) -> f64 {
        (self.x0 + self.x1) / 2.0
    }
    pub fn cy(&self) -> f64 {
        (self.y0 + self.y1) / 2.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TextSource {
    /// The PDF's own text layer.
    TextLayer,
    /// Recognised from a picture of the page.
    Ocr,
    /// Neither gave usable text.
    None,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Page {
    /// Counting from 1.
    pub number: u32,
    pub width: f64,
    pub height: f64,
    pub words: Vec<Word>,
    pub source: TextSource,
    /// The text layer looked garbled (wrong character maps), so its words should not be trusted.
    pub garbled: bool,
    /// Reading the page failed part way.
    pub failed: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PdfError(pub String);

impl std::fmt::Display for PdfError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

fn damaged() -> PdfError {
    PdfError(
        "this PDF could not be read; it may be damaged, cut short, or saved in an unusual way"
            .into(),
    )
}

fn on_reader_thread<T: Send>(work: impl FnOnce() -> T + Send) -> Result<T, PdfError> {
    std::thread::scope(|scope| {
        let handle = std::thread::Builder::new()
            .name("pdf-reader".into())
            .stack_size(READER_STACK_BYTES)
            .spawn_scoped(scope, work)
            .map_err(|_| {
                PdfError("the computer did not have enough memory to read this PDF".into())
            })?;
        handle.join().map_err(|_| damaged())
    })
}

/// Reads every page's words from the text layer. Pages with no text layer come back empty with `source: None`; the
/// OCR step fills them.
pub fn read_text_layer(bytes: &[u8]) -> Result<Vec<Page>, PdfError> {
    on_reader_thread(|| read_pages(bytes))?
}

fn read_pages(bytes: &[u8]) -> Result<Vec<Page>, PdfError> {
    let mut doc = Document::load_mem(bytes).map_err(|_| damaged())?;
    if doc.is_encrypted() && doc.decrypt("").is_err() {
        return Err(PdfError(
            "this PDF is protected by a password; save an unprotected copy and use that".into(),
        ));
    }
    let numbers: Vec<u32> = doc.get_pages().keys().copied().collect();
    if numbers.is_empty() {
        return Err(PdfError(
            "this PDF has no pages, or its page list is damaged".into(),
        ));
    }
    clean_streams(&mut doc);
    let mut pages = Vec::with_capacity(numbers.len());
    for number in numbers {
        let mut c = Collector::default();
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            pdf_extract::output_doc_page(&doc, &mut c, number)
        }));
        let failed = !matches!(outcome, Ok(Ok(())));
        c.finish_word();
        let (width, height) = (c.width.max(1.0), c.height.max(1.0));
        let words = c.words;
        let garbled = looks_garbled(&words);
        let source = if words.is_empty() {
            TextSource::None
        } else {
            TextSource::TextLayer
        };
        pages.push(Page {
            number,
            width,
            height,
            words,
            source,
            garbled,
            failed,
        });
    }
    Ok(pages)
}

/// Makes the document safe for `pdf-extract` (adapted from PrismQDA): the `'` and `"` operators become `T*` and `Tj`,
/// operators that do not place or show text are dropped, and images are emptied.
fn clean_streams(doc: &mut Document) {
    let pages: Vec<ObjectId> = doc.get_pages().into_values().collect();
    for id in pages {
        let Ok(raw) = doc.get_page_content(id) else {
            continue;
        };
        let Some(clean) = clean_content(&raw) else {
            continue;
        };
        let stream_id = doc.add_object(Stream::new(Dictionary::new(), clean));
        if let Ok(Object::Dictionary(page)) = doc.get_object_mut(id) {
            page.set("Contents", Object::Reference(stream_id));
        }
    }
    let ids: Vec<ObjectId> = doc.objects.keys().copied().collect();
    for id in ids {
        let Some(Object::Stream(stream)) = doc.objects.get_mut(&id) else {
            continue;
        };
        let subtype = stream
            .dict
            .get(b"Subtype")
            .and_then(Object::as_name)
            .ok()
            .map(<[u8]>::to_vec);
        match subtype.as_deref() {
            Some(b"Image") => stream.set_plain_content(Vec::new()),
            Some(b"Form") => {
                if let Some(clean) = stream
                    .decompressed_content()
                    .ok()
                    .and_then(|raw| clean_content(&raw))
                {
                    stream.set_plain_content(clean);
                }
            }
            _ => {}
        }
    }
}

fn is_number(o: &Object) -> bool {
    matches!(o, Object::Integer(_) | Object::Real(_))
}

fn clean_content(raw: &[u8]) -> Option<Vec<u8>> {
    let content = Content::decode(raw).ok()?;
    let mut kept: Vec<Operation> = Vec::with_capacity(content.operations.len());
    for op in content.operations {
        let numbers = op.operands.iter().all(is_number);
        let n = op.operands.len();
        match op.operator.as_str() {
            "'" if matches!(op.operands.as_slice(), [Object::String(..)]) => {
                kept.push(Operation::new("T*", vec![]));
                kept.push(Operation::new("Tj", op.operands));
            }
            "\"" if n == 3
                && is_number(&op.operands[0])
                && is_number(&op.operands[1])
                && matches!(op.operands[2], Object::String(..)) =>
            {
                let mut operands = op.operands;
                let text = operands.pop().expect("three operands");
                let char_spacing = operands.pop().expect("three operands");
                let word_spacing = operands.pop().expect("three operands");
                kept.push(Operation::new("Tw", vec![word_spacing]));
                kept.push(Operation::new("Tc", vec![char_spacing]));
                kept.push(Operation::new("T*", vec![]));
                kept.push(Operation::new("Tj", vec![text]));
            }
            "Tj" if matches!(op.operands.as_slice(), [Object::String(..)]) => kept.push(op),
            "TJ" if matches!(op.operands.as_slice(), [Object::Array(_)]) => kept.push(op),
            "Tf" if matches!(op.operands.as_slice(), [Object::Name(_), size] if is_number(size)) => {
                kept.push(op)
            }
            "Tm" | "cm" if n == 6 && numbers => kept.push(op),
            "Td" | "TD" if n == 2 && numbers => kept.push(op),
            "Tc" | "Tw" | "Tz" | "TL" | "Ts" if n == 1 && numbers => kept.push(op),
            "Do" if matches!(op.operands.as_slice(), [Object::Name(_)]) => kept.push(op),
            "BT" | "ET" | "T*" | "q" | "Q" if n == 0 => kept.push(op),
            _ => {}
        }
    }
    Content { operations: kept }.encode().ok()
}

/// Ligatures become their letters, odd spaces plain spaces, control characters are dropped (from PrismQDA).
fn push_clean(out: &mut String, text: &str) {
    for c in text.chars() {
        match c {
            '\u{FB00}' => out.push_str("ff"),
            '\u{FB01}' => out.push_str("fi"),
            '\u{FB02}' => out.push_str("fl"),
            '\u{FB03}' => out.push_str("ffi"),
            '\u{FB04}' => out.push_str("ffl"),
            '\u{FB05}' | '\u{FB06}' => out.push_str("st"),
            '\u{00A0}'
            | '\u{2002}'..='\u{200A}'
            | '\u{202F}'
            | '\u{205F}'
            | '\u{3000}'
            | '\t'
            | '\n'
            | '\r' => out.push(' '),
            '\u{200B}' | '\u{FEFF}' | '\u{2060}' => {}
            c if c.is_control() => {}
            c => out.push(c),
        }
    }
}

#[derive(Default)]
struct Collector {
    /// The page's visible area: left, bottom (PDF coordinates) and size.
    left: f64,
    top: f64,
    width: f64,
    height: f64,
    words: Vec<Word>,
    current: Option<Word>,
    /// Where the next glyph would start if nothing came between, and the last glyph's baseline.
    advance: f64,
    baseline: f64,
    last_text: String,
    last_x: f64,
}

impl Collector {
    fn finish_word(&mut self) {
        if let Some(w) = self.current.take() {
            let t = w.text.trim();
            if !t.is_empty() {
                let drawn_again = self.words.last().is_some_and(|p| {
                    p.text == t
                        && (p.x0 - w.x0).abs() < 0.15 * w.size
                        && (p.y0 - w.y0).abs() < 0.15 * w.size
                });
                if !drawn_again {
                    self.words.push(Word {
                        text: t.to_string(),
                        ..w
                    });
                }
            }
        }
    }
}

impl OutputDev for Collector {
    fn begin_page(
        &mut self,
        _page: u32,
        media_box: &MediaBox,
        art: Option<(f64, f64, f64, f64)>,
    ) -> Result<(), OutputError> {
        self.finish_word();
        let _ = art;
        let (llx, lly, urx, ury) = (media_box.llx, media_box.lly, media_box.urx, media_box.ury);
        self.left = llx.min(urx);
        self.top = lly.max(ury);
        self.width = (urx - llx).abs();
        self.height = (ury - lly).abs();
        Ok(())
    }

    fn end_page(&mut self) -> Result<(), OutputError> {
        self.finish_word();
        Ok(())
    }

    fn output_character(
        &mut self,
        trm: &Transform,
        width: f64,
        spacing: f64,
        font_size: f64,
        text: &str,
    ) -> Result<(), OutputError> {
        let scale_x = trm.m11.hypot(trm.m12);
        let (cos, sin) = if scale_x > 1e-9 {
            (trm.m11 / scale_x, trm.m12 / scale_x)
        } else {
            (1.0, 0.0)
        };
        // Turned text (a sideways journal name in the margin) is left out of the reading.
        if sin.abs() > 0.2 || cos < 0.0 {
            self.finish_word();
            return Ok(());
        }
        let nominal = if font_size.is_finite() && font_size.abs() > 1e-9 {
            font_size.abs()
        } else {
            1.0
        };
        let mut size = nominal * trm.m21.hypot(trm.m22);
        if !size.is_finite() || size < 0.01 {
            size = nominal;
        }
        let (x, y) = (trm.m31, trm.m32);
        if !x.is_finite() || !y.is_finite() {
            return Ok(());
        }
        let mut glyph = String::new();
        push_clean(&mut glyph, text);
        if glyph.is_empty() {
            return Ok(());
        }
        let em = if width.is_finite() && width > 0.01 {
            width
        } else {
            0.5
        };
        let step = (em * nominal + spacing) * scale_x;
        let gx0 = x - self.left;
        let gx1 = gx0
            + if step.is_finite() && step > 0.0 {
                step
            } else {
                0.5 * size
            };
        let baseline = self.top - y;
        // Some generators draw each glyph twice, a hair apart, to fake bold. Keep one.
        if self.last_text == glyph
            && (gx0 - self.last_x).abs() < 0.1 * size
            && (baseline - self.baseline).abs() < 0.1 * size
            && !glyph.trim().is_empty()
        {
            return Ok(());
        }
        let is_space = glyph.chars().all(char::is_whitespace);
        let continues = self.current.as_ref().is_some_and(|w| {
            let gap = gx0 - self.advance;
            (baseline - self.baseline).abs() < 0.5 * size.max(w.size)
                && gap < 0.2 * size.max(w.size)
                && gap > -0.5 * size
        });
        if is_space || !continues {
            self.finish_word();
        }
        if !is_space {
            let w = self.current.get_or_insert_with(|| Word {
                text: String::new(),
                x0: gx0,
                y0: baseline - 0.8 * size,
                x1: gx1,
                y1: baseline + 0.2 * size,
                size,
                confidence: 1.0,
            });
            w.text.push_str(&glyph);
            w.x1 = w.x1.max(gx1);
            w.y0 = w.y0.min(baseline - 0.8 * size);
            w.y1 = w.y1.max(baseline + 0.2 * size);
            w.size = w.size.max(size);
        }
        self.advance = gx1;
        self.baseline = baseline;
        self.last_text = glyph;
        self.last_x = gx0;
        Ok(())
    }

    fn begin_word(&mut self) -> Result<(), OutputError> {
        Ok(())
    }
    fn end_word(&mut self) -> Result<(), OutputError> {
        Ok(())
    }
    fn end_line(&mut self) -> Result<(), OutputError> {
        Ok(())
    }
}

/// A text layer with broken character maps comes out as symbols and stray letters. Too few ordinary words means the
/// page should be read from its picture instead.
pub fn looks_garbled(words: &[Word]) -> bool {
    if words.len() < 20 {
        return false;
    }
    let letters_only = |w: &&Word| {
        w.text.chars().filter(|c| c.is_alphabetic()).count() * 2 >= w.text.chars().count()
    };
    let wordlike = words.iter().filter(letters_only).count();
    let common = [
        "the", "and", "of", "to", "in", "a", "is", "for", "that", "with",
    ];
    let has_common = words
        .iter()
        .filter(|w| {
            common.contains(
                &w.text
                    .to_lowercase()
                    .trim_matches(|c: char| !c.is_alphabetic()),
            )
        })
        .count();
    // Words broken into fragments ("ith w", "nd a"): too many one- and two-letter pieces that are not real words.
    let short_ok = [
        "a", "i", "of", "to", "in", "is", "on", "at", "by", "or", "an", "as", "be", "we", "it",
        "us", "if", "no", "so", "do", "my", "me", "he", "up", "vs", "et", "al", "e.g.", "i.e.",
        // Statistics tables are full of short tokens that are not broken words.
        "sd", "df", "se", "ns", "ci", "m", "n", "t", "p", "r", "f", "h", "k", "b", "x", "y", "z",
        "ii", "iii", "iv", "vi",
    ];
    let fragments = words
        .iter()
        .filter(|w| {
            let t = w.text.to_lowercase();
            let letters: String = t.chars().filter(|c| c.is_alphabetic()).collect();
            !letters.is_empty()
                && letters.len() <= 2
                && t.chars().all(char::is_alphabetic)
                && !short_ok.contains(&t.as_str())
        })
        .count();
    wordlike * 100 < words.len() * 60
        || has_common * 100 < words.len()
        || fragments * 100 > words.len() * 8
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w(t: &str) -> Word {
        Word {
            text: t.into(),
            x0: 0.0,
            y0: 0.0,
            x1: 1.0,
            y1: 1.0,
            size: 10.0,
            confidence: 1.0,
        }
    }

    #[test]
    fn ordinary_text_is_not_garbled() {
        let words: Vec<Word> = "the scale of the measure is in the appendix and the items are listed with the anchors for each".split(' ').map(w).collect();
        assert!(!looks_garbled(&words));
    }

    #[test]
    fn symbol_soup_is_garbled() {
        let words: Vec<Word> = (0..40)
            .map(|i| w(if i % 2 == 0 { "#$%" } else { "x7" }))
            .collect();
        assert!(looks_garbled(&words));
    }
}
