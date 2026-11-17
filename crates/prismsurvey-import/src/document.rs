//! Reading a stored scale file into a simple document: paragraphs (with which words are bold, where the format keeps
//! that) and tables. The three readers are `.txt`, `.docx` and Word 97-2003 `.doc`.

use std::io::Read;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Paragraph {
    pub text: String,
    /// True when every visible character of the paragraph is bold. `None` when the file format does not say.
    pub bold: Option<bool>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Cell {
    pub text: String,
    pub bold: Option<bool>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Block {
    Para(Paragraph),
    Table(Vec<Vec<Cell>>),
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Document {
    pub blocks: Vec<Block>,
}

#[derive(Debug, thiserror::Error)]
pub enum ReadError {
    #[error("could not read the file: {0}")]
    Io(#[from] std::io::Error),
    #[error("the Word file could not be opened: {0}")]
    Docx(String),
    #[error("the Word 97-2003 file could not be opened: {0}")]
    Doc(String),
    #[error("this kind of file is not read here: .{0}")]
    Unsupported(String),
}

pub fn read_path(path: &Path) -> Result<Document, ReadError> {
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    let bytes = std::fs::read(path)?;
    match ext.as_str() {
        "txt" | "text" => Ok(read_txt(&bytes)),
        "docx" => read_docx(&bytes),
        "doc" => {
            // Some ".doc" files are really .docx, RTF or plain text saved under the old name.
            if bytes.starts_with(b"PK") {
                read_docx(&bytes)
            } else if bytes.starts_with(&[0xD0, 0xCF, 0x11, 0xE0]) {
                read_doc(&bytes)
            } else if bytes.starts_with(b"{\\rtf") {
                Err(ReadError::Unsupported("rtf (saved as .doc)".into()))
            } else {
                Ok(read_txt(&bytes))
            }
        }
        other => Err(ReadError::Unsupported(other.to_string())),
    }
}

// ---------------------------------------------------------------- plain text

/// Plain text: UTF-8 (with or without a byte order mark), UTF-16 with a mark, or Windows-1252. Lines in the pipe-table
/// style that text copies of Word files use ("|cell |cell |") are read back as tables.
pub fn read_txt(bytes: &[u8]) -> Document {
    let text = decode_text(bytes);
    let mut blocks = Vec::new();
    let mut table: Vec<Vec<Cell>> = Vec::new();
    for raw in text.split('\n') {
        let line = raw.trim_end_matches('\r');
        let t = line.trim();
        if t.starts_with('|') && t.ends_with('|') && t.len() > 1 {
            let cells = t[1..t.len() - 1]
                .split('|')
                .map(|c| Cell {
                    text: c.trim().to_string(),
                    bold: None,
                })
                .collect();
            table.push(cells);
            continue;
        }
        if !table.is_empty() {
            blocks.push(Block::Table(std::mem::take(&mut table)));
        }
        blocks.push(Block::Para(Paragraph {
            text: line.to_string(),
            bold: None,
        }));
    }
    if !table.is_empty() {
        blocks.push(Block::Table(table));
    }
    Document { blocks }
}

fn decode_text(bytes: &[u8]) -> String {
    if let Some(rest) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        return String::from_utf8_lossy(rest).into_owned();
    }
    if bytes.starts_with(&[0xFF, 0xFE]) {
        return encoding_rs::UTF_16LE.decode(bytes).0.into_owned();
    }
    if bytes.starts_with(&[0xFE, 0xFF]) {
        return encoding_rs::UTF_16BE.decode(bytes).0.into_owned();
    }
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => encoding_rs::WINDOWS_1252.decode(bytes).0.into_owned(),
    }
}

// ---------------------------------------------------------------- .docx

#[derive(Default)]
struct RunState {
    bold: bool,
}

#[derive(Default)]
struct ParaBuilder {
    text: String,
    bold_chars: usize,
    plain_chars: usize,
}

impl ParaBuilder {
    fn push(&mut self, s: &str, bold: bool) {
        let visible = s.chars().filter(|c| !c.is_whitespace()).count();
        if bold {
            self.bold_chars += visible;
        } else {
            self.plain_chars += visible;
        }
        self.text.push_str(s);
    }
    fn finish(self) -> Paragraph {
        let bold = if self.bold_chars + self.plain_chars == 0 {
            Some(false)
        } else {
            Some(self.plain_chars == 0)
        };
        Paragraph {
            text: self.text,
            bold,
        }
    }
}

/// Reads `word/document.xml`: paragraphs, runs (bold or not), tabs, breaks and tables. A table inside a table is read
/// into its cell as text.
pub fn read_docx(bytes: &[u8]) -> Result<Document, ReadError> {
    use quick_xml::events::Event;
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes))
        .map_err(|e| ReadError::Docx(e.to_string()))?;
    let mut xml = String::new();
    zip.by_name("word/document.xml")
        .map_err(|e| ReadError::Docx(e.to_string()))?
        .read_to_string(&mut xml)
        .map_err(|e| ReadError::Docx(e.to_string()))?;
    let mut reader = quick_xml::Reader::from_str(&xml);
    let mut blocks = Vec::new();
    // Table stack: each level holds rows, the current row's cells, and the current cell's paragraphs.
    struct TableLevel {
        rows: Vec<Vec<Cell>>,
        row: Vec<Cell>,
        cell: Vec<Paragraph>,
    }
    let mut tables: Vec<TableLevel> = Vec::new();
    let mut para: Option<ParaBuilder> = None;
    let mut run = RunState::default();
    let mut in_rpr = false;
    let mut in_text = false;
    let mut in_instr = false;
    loop {
        let ev = reader
            .read_event()
            .map_err(|e| ReadError::Docx(e.to_string()))?;
        match ev {
            Event::Start(ref e) | Event::Empty(ref e) => {
                let is_empty = matches!(ev, Event::Empty(_));
                let name = e.local_name();
                match name.as_ref() {
                    b"p" if !is_empty => para = Some(ParaBuilder::default()),
                    b"p" => push_para(
                        &mut blocks,
                        &mut tables.last_mut().map(|t| &mut t.cell),
                        Paragraph::default(),
                    ),
                    b"r" => run = RunState::default(),
                    b"rPr" => in_rpr = !is_empty,
                    b"b" if in_rpr => {
                        let off = e.attributes().flatten().any(|a| {
                            a.key.local_name().as_ref() == b"val"
                                && matches!(a.value.as_ref(), b"0" | b"false" | b"off")
                        });
                        run.bold = !off;
                    }
                    b"t" if !is_empty => in_text = true,
                    b"instrText" if !is_empty => in_instr = true,
                    b"tab" if !in_rpr => {
                        if let Some(p) = para.as_mut() {
                            p.push("\t", run.bold);
                        }
                    }
                    b"br" | b"cr" => {
                        if let Some(p) = para.as_mut() {
                            p.push("\n", run.bold);
                        }
                    }
                    b"tbl" if !is_empty => {
                        if let Some(p) = para.take() {
                            // A table cannot start inside a paragraph, but be safe.
                            push_para(
                                &mut blocks,
                                &mut tables.last_mut().map(|t| &mut t.cell),
                                p.finish(),
                            );
                        }
                        tables.push(TableLevel {
                            rows: Vec::new(),
                            row: Vec::new(),
                            cell: Vec::new(),
                        })
                    }
                    b"tr" if !is_empty => {
                        if let Some(t) = tables.last_mut() {
                            t.row.clear();
                        }
                    }
                    b"tc" if !is_empty => {
                        if let Some(t) = tables.last_mut() {
                            t.cell.clear();
                        }
                    }
                    _ => {}
                }
            }
            Event::Text(t) => {
                if in_text && !in_instr {
                    let s = t.decode().map_err(|e| ReadError::Docx(e.to_string()))?;
                    if let Some(p) = para.as_mut() {
                        p.push(&s, run.bold);
                    }
                }
            }
            Event::GeneralRef(r) => {
                if in_text && !in_instr {
                    let s = match r.resolve_char_ref() {
                        Ok(Some(c)) => c.to_string(),
                        _ => match r
                            .decode()
                            .map_err(|e| ReadError::Docx(e.to_string()))?
                            .as_ref()
                        {
                            "amp" => "&".to_string(),
                            "lt" => "<".to_string(),
                            "gt" => ">".to_string(),
                            "quot" => "\"".to_string(),
                            "apos" => "'".to_string(),
                            _ => String::new(),
                        },
                    };
                    if let Some(p) = para.as_mut() {
                        p.push(&s, run.bold);
                    }
                }
            }
            Event::End(e) => match e.local_name().as_ref() {
                b"p" => {
                    if let Some(p) = para.take() {
                        push_para(
                            &mut blocks,
                            &mut tables.last_mut().map(|t| &mut t.cell),
                            p.finish(),
                        );
                    }
                }
                b"rPr" => in_rpr = false,
                b"t" => in_text = false,
                b"instrText" => in_instr = false,
                b"tc" => {
                    if let Some(t) = tables.last_mut() {
                        let paras = std::mem::take(&mut t.cell);
                        let text = paras
                            .iter()
                            .map(|p| p.text.as_str())
                            .collect::<Vec<_>>()
                            .join("\n");
                        let visible: Vec<&Paragraph> =
                            paras.iter().filter(|p| !p.text.trim().is_empty()).collect();
                        let bold = if visible.is_empty() {
                            Some(false)
                        } else {
                            Some(visible.iter().all(|p| p.bold == Some(true)))
                        };
                        t.row.push(Cell { text, bold });
                    }
                }
                b"tr" => {
                    if let Some(t) = tables.last_mut() {
                        let row = std::mem::take(&mut t.row);
                        t.rows.push(row);
                    }
                }
                b"tbl" => {
                    if let Some(done) = tables.pop() {
                        if let Some(outer) = tables.last_mut() {
                            // A nested table becomes text in the outer cell.
                            for row in done.rows {
                                let text = row
                                    .iter()
                                    .map(|c| c.text.as_str())
                                    .collect::<Vec<_>>()
                                    .join("\t");
                                outer.cell.push(Paragraph { text, bold: None });
                            }
                        } else {
                            blocks.push(Block::Table(done.rows));
                        }
                    }
                }
                _ => {}
            },
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(Document { blocks })
}

fn push_para(blocks: &mut Vec<Block>, cell: &mut Option<&mut Vec<Paragraph>>, p: Paragraph) {
    match cell {
        Some(c) => c.push(p),
        None => blocks.push(Block::Para(p)),
    }
}

// ---------------------------------------------------------------- .doc (Word 97-2003)

fn u16_at(b: &[u8], at: usize) -> Option<u16> {
    b.get(at..at + 2).map(|s| u16::from_le_bytes([s[0], s[1]]))
}
fn u32_at(b: &[u8], at: usize) -> Option<u32> {
    b.get(at..at + 4)
        .map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

/// One character of Word's main text with what the formatting tables say about it.
#[derive(Debug, Clone, Copy)]
struct WordChar {
    c: char,
    /// The paragraph this mark ends is a table row end (sprmPFTtp or sprmPFInnerTtp).
    row_end: bool,
    /// The paragraph this mark ends is inside a table (sprmPFInTable).
    in_table: bool,
    bold: bool,
}

const ROW_END: u8 = 1;
const IN_TABLE: u8 = 2;
const BOLD: u8 = 1;

/// Runs of file offsets [start, end) with flags, from the formatting pages.
type Runs = Vec<(u32, u32, u8)>;

fn lookup(runs: &Runs, fc: u32) -> u8 {
    use std::cmp::Ordering;
    match runs.binary_search_by(|r| {
        if fc < r.0 {
            Ordering::Greater
        } else if fc >= r.1 {
            Ordering::Less
        } else {
            Ordering::Equal
        }
    }) {
        Ok(i) => runs[i].2,
        Err(_) => 0,
    }
}

/// Walks a grpprl (a list of property modifiers) and calls `f` with each opcode and its operand.
fn each_sprm(grpprl: &[u8], mut f: impl FnMut(u16, &[u8])) {
    let mut at = 0;
    while at + 2 <= grpprl.len() {
        let op = u16::from_le_bytes([grpprl[at], grpprl[at + 1]]);
        at += 2;
        let len = match (op >> 13) & 7 {
            0 | 1 => 1,
            2 | 4 | 5 => 2,
            3 => 4,
            7 => 3,
            _ => {
                // Variable length: one size byte, except sprmTDefTable (0xD608), which has two.
                if op == 0xD608 {
                    let Some(l) = u16_at(grpprl, at) else { return };
                    at += 2;
                    (l as usize).saturating_sub(1)
                } else {
                    let Some(&l) = grpprl.get(at) else { return };
                    at += 1;
                    l as usize
                }
            }
        };
        let Some(operand) = grpprl.get(at..at + len) else {
            return;
        };
        f(op, operand);
        at += len;
    }
}

/// Reads the bin table at `fc`/`lcb` in the table stream and every formatting page it points to. `papx` chooses
/// paragraph pages (row ends) or character pages (bold).
fn read_fkps(word: &[u8], table: &[u8], data: &[u8], fc: usize, lcb: usize, papx: bool) -> Runs {
    let mut runs = Vec::new();
    if lcb < 4 {
        return runs;
    }
    let Some(plc) = table.get(fc..fc + lcb) else {
        return runs;
    };
    let n = (lcb - 4) / 8;
    for i in 0..n {
        let Some(pn) = u32_at(plc, (n + 1) * 4 + i * 4) else {
            continue;
        };
        let base = (pn & 0x003F_FFFF) as usize * 512;
        let Some(page) = word.get(base..base + 512) else {
            continue;
        };
        let crun = page[511] as usize;
        for r in 0..crun {
            let (Some(start), Some(end)) = (u32_at(page, r * 4), u32_at(page, (r + 1) * 4)) else {
                continue;
            };
            let entry = (crun + 1) * 4 + r * if papx { 13 } else { 1 };
            let Some(&b_off) = page.get(entry) else {
                continue;
            };
            let mut value = 0u8;
            if b_off != 0 {
                let at = b_off as usize * 2;
                if papx {
                    // PapxInFkp: cb, then (when cb is 0) a second size byte; then istd and the grpprl.
                    if let Some(&cb) = page.get(at) {
                        let (g0, size) = if cb != 0 {
                            (at + 1, cb as usize * 2 - 1)
                        } else {
                            (at + 2, page.get(at + 1).copied().unwrap_or(0) as usize * 2)
                        };
                        if let Some(g) = page.get(g0..(g0 + size).min(512)) {
                            if g.len() > 2 {
                                each_sprm(&g[2..], |op, operand| {
                                    if (op == 0x2417 || op == 0x244C) && operand.first() == Some(&1)
                                    {
                                        value |= ROW_END;
                                    }
                                    if op == 0x2416 && operand.first() == Some(&1) {
                                        value |= IN_TABLE;
                                    }
                                    // sprmPHugePapx: the properties are too large for the page and live in the Data stream.
                                    if op == 0x6646 && operand.len() == 4 {
                                        let off = u32::from_le_bytes([
                                            operand[0], operand[1], operand[2], operand[3],
                                        ])
                                            as usize;
                                        if let Some(cb) = u16_at(data, off) {
                                            if let Some(huge) =
                                                data.get(off + 2..off + 2 + cb as usize)
                                            {
                                                // A PrcData: the size, then the property list (no style index).
                                                each_sprm(huge, |op2, operand2| {
                                                    if (op2 == 0x2417 || op2 == 0x244C)
                                                        && operand2.first() == Some(&1)
                                                    {
                                                        value |= ROW_END;
                                                    }
                                                    if op2 == 0x2416 && operand2.first() == Some(&1)
                                                    {
                                                        value |= IN_TABLE;
                                                    }
                                                });
                                            }
                                        }
                                    }
                                });
                            }
                        }
                    }
                } else if let Some(&cb) = page.get(at) {
                    if let Some(g) = page.get(at + 1..(at + 1 + cb as usize).min(512)) {
                        each_sprm(g, |op, operand| {
                            if op == 0x0835 {
                                // 1 = bold; 0x81 = the opposite of the style, taken as bold for body text.
                                value = if matches!(operand.first(), Some(1) | Some(0x81)) {
                                    BOLD
                                } else {
                                    0
                                };
                            }
                        });
                    }
                }
            }
            runs.push((start, end, value));
        }
    }
    runs.sort_by_key(|r| r.0);
    runs
}

/// Reads the main text of a Word 97-2003 file through its piece table (MS-DOC 2.4.1), with table row ends from the
/// paragraph properties and bold from the character properties (direct formatting only; bold that comes from a style is
/// not seen).
/// Word 6.0 and Word 95 files (an older layout of the same container): the text only, read straight from the file or
/// through its piece table. Formatting is not read, so table rows are told apart by a doubled cell mark and bold is
/// not seen.
fn read_word6(word: &[u8]) -> Result<Document, ReadError> {
    let err = |m: &str| ReadError::Doc(m.to_string());
    let flags = u16_at(word, 0x0A).ok_or_else(|| err("the file header is cut short"))?;
    if flags & 0x0100 != 0 {
        return Err(err("the file is password protected"));
    }
    let fc_min = u32_at(word, 0x18).ok_or_else(|| err("header"))? as usize;
    let ccp_text = u32_at(word, 0x34).ok_or_else(|| err("header"))? as usize;
    let mut bytes: Vec<u8> = Vec::with_capacity(ccp_text);
    if flags & 0x0004 != 0 {
        // A complex file: the text is in pieces listed by the piece table in the same stream.
        let fc_clx = u32_at(word, 0x01A2).ok_or_else(|| err("header"))? as usize;
        let lcb_clx = u32_at(word, 0x01A6).ok_or_else(|| err("header"))? as usize;
        let clx = word
            .get(fc_clx..fc_clx + lcb_clx)
            .ok_or_else(|| err("the piece table is outside the file"))?;
        let mut at = 0;
        while clx.get(at) == Some(&0x01) {
            at += 3 + u16_at(clx, at + 1).ok_or_else(|| err("piece table"))? as usize;
        }
        if clx.get(at) != Some(&0x02) {
            return Err(err("no piece table"));
        }
        let lcb = u32_at(clx, at + 1).ok_or_else(|| err("piece table"))? as usize;
        let plc = clx
            .get(at + 5..at + 5 + lcb)
            .ok_or_else(|| err("piece table"))?;
        let n = lcb.saturating_sub(4) / 12;
        for i in 0..n {
            let cp0 = u32_at(plc, i * 4).unwrap_or(0) as usize;
            let cp1 = u32_at(plc, (i + 1) * 4).unwrap_or(0) as usize;
            if cp0 >= ccp_text {
                break;
            }
            let len = cp1.min(ccp_text).saturating_sub(cp0);
            let fc =
                u32_at(plc, (n + 1) * 4 + i * 8 + 2).ok_or_else(|| err("piece table"))? as usize;
            bytes.extend_from_slice(
                word.get(fc..fc + len)
                    .ok_or_else(|| err("text is outside the file"))?,
            );
        }
    } else {
        bytes.extend_from_slice(
            word.get(fc_min..fc_min + ccp_text)
                .ok_or_else(|| err("text is outside the file"))?,
        );
    }
    let text = encoding_rs::WINDOWS_1252
        .decode_without_bom_handling(&bytes)
        .0
        .into_owned();
    // With no row information, a cell mark that follows another ends the row.
    let mut chars = Vec::with_capacity(text.len());
    let mut prev_cell = false;
    for c in text.chars() {
        let row_end = c == '\u{07}' && prev_cell;
        chars.push(WordChar {
            c,
            row_end,
            in_table: false,
            bold: false,
        });
        prev_cell = c == '\u{07}' && !row_end;
    }
    Ok(blocks_from_word_chars(&chars))
}

/// Opens the container. Some files end before the last sectors their allocation table lists (Word reads them anyway,
/// since those sectors are unused); such a file is padded with empty sectors in memory and opened again.
fn open_container(bytes: &[u8]) -> Result<cfb::CompoundFile<std::io::Cursor<Vec<u8>>>, ReadError> {
    match cfb::CompoundFile::open(std::io::Cursor::new(bytes.to_vec())) {
        Ok(c) => Ok(c),
        Err(e) => {
            let msg = e.to_string();
            let entries = msg
                .split("FAT has ")
                .nth(1)
                .and_then(|r| r.split(' ').next())
                .and_then(|n| n.parse::<usize>().ok())
                .filter(|_| msg.contains("but file has only"));
            let Some(entries) = entries else {
                return Err(ReadError::Doc(msg));
            };
            // The header takes one sector of the sector size; sectors are 512 bytes in version 3 files.
            let sector = if bytes.get(0x1E) == Some(&12) {
                4096
            } else {
                512
            };
            let want = (entries + 1) * sector;
            if want <= bytes.len() || want > bytes.len() + 64 * 1024 * 1024 {
                return Err(ReadError::Doc(msg));
            }
            let mut padded = bytes.to_vec();
            padded.resize(want, 0);
            cfb::CompoundFile::open(std::io::Cursor::new(padded))
                .map_err(|e| ReadError::Doc(e.to_string()))
        }
    }
}

pub fn read_doc(bytes: &[u8]) -> Result<Document, ReadError> {
    let err = |m: &str| ReadError::Doc(m.to_string());
    let mut cfb = open_container(bytes)?;
    let mut word = Vec::new();
    cfb.open_stream("/WordDocument")
        .map_err(|e| ReadError::Doc(e.to_string()))?
        .read_to_end(&mut word)?;
    if u16_at(&word, 0) == Some(0xA5DC) {
        return read_word6(&word);
    }
    if u16_at(&word, 0) != Some(0xA5EC) {
        return Err(err("this is not a Word document"));
    }
    let flags = u16_at(&word, 0x0A).ok_or_else(|| err("the file header is cut short"))?;
    if flags & 0x0100 != 0 {
        return Err(err("the file is password protected"));
    }
    let table_name = if flags & 0x0200 != 0 {
        "/1Table"
    } else {
        "/0Table"
    };
    let mut table = Vec::new();
    cfb.open_stream(table_name)
        .map_err(|e| ReadError::Doc(e.to_string()))?
        .read_to_end(&mut table)?;
    let mut data = Vec::new();
    if let Ok(mut s) = cfb.open_stream("/Data") {
        s.read_to_end(&mut data)?;
    }

    // FibBase is 32 bytes, then csw and fibRgW, then cslw and fibRgLw, then cbRgFcLcb and the fc/lcb pairs.
    let csw = u16_at(&word, 32).ok_or_else(|| err("header"))? as usize;
    let lw_start = 34 + csw * 2;
    let cslw = u16_at(&word, lw_start).ok_or_else(|| err("header"))? as usize;
    let rglw = lw_start + 2;
    let ccp_text = u32_at(&word, rglw + 3 * 4).ok_or_else(|| err("header"))? as usize;
    let fclcb = rglw + cslw * 4 + 2;
    let pair = |i: usize| -> Option<(usize, usize)> {
        Some((
            u32_at(&word, fclcb + i * 8)? as usize,
            u32_at(&word, fclcb + i * 8 + 4)? as usize,
        ))
    };
    let (fc_clx, lcb_clx) = pair(33).ok_or_else(|| err("header"))?;
    let chpx = pair(12)
        .map(|(fc, lcb)| read_fkps(&word, &table, &data, fc, lcb, false))
        .unwrap_or_default();
    let papx = pair(13)
        .map(|(fc, lcb)| read_fkps(&word, &table, &data, fc, lcb, true))
        .unwrap_or_default();
    let clx = table
        .get(fc_clx..fc_clx + lcb_clx)
        .ok_or_else(|| err("the piece table is outside the file"))?;

    // Skip any Prc entries (0x01), then read the Pcdt (0x02).
    let mut at = 0;
    while clx.get(at) == Some(&0x01) {
        let cb = u16_at(clx, at + 1).ok_or_else(|| err("piece table"))? as usize;
        at += 3 + cb;
    }
    if clx.get(at) != Some(&0x02) {
        return Err(err("no piece table"));
    }
    let lcb = u32_at(clx, at + 1).ok_or_else(|| err("piece table"))? as usize;
    let plc = clx
        .get(at + 5..at + 5 + lcb)
        .ok_or_else(|| err("piece table"))?;
    let n = lcb.saturating_sub(4) / 12;
    let mut chars: Vec<WordChar> = Vec::with_capacity(ccp_text);
    for i in 0..n {
        let cp0 = u32_at(plc, i * 4).unwrap_or(0) as usize;
        let cp1 = u32_at(plc, (i + 1) * 4).unwrap_or(0) as usize;
        if cp0 >= ccp_text {
            break;
        }
        let len = cp1.min(ccp_text).saturating_sub(cp0);
        let pcd = (n + 1) * 4 + i * 8;
        let fc_raw = u32_at(plc, pcd + 2).ok_or_else(|| err("piece table"))?;
        let compressed = fc_raw & 0x4000_0000 != 0;
        let (start, width) = if compressed {
            (((fc_raw & 0x3FFF_FFFF) / 2) as usize, 1)
        } else {
            (fc_raw as usize, 2)
        };
        let raw = word
            .get(start..start + len * width)
            .ok_or_else(|| err("text is outside the file"))?;
        let decoded: Vec<char> = if compressed {
            // One byte per character; Windows-1252 maps each byte to exactly one character.
            raw.iter()
                .map(|b| {
                    encoding_rs::WINDOWS_1252
                        .decode_without_bom_handling(&[*b])
                        .0
                        .chars()
                        .next()
                        .unwrap_or('\u{FFFD}')
                })
                .collect()
        } else {
            let units: Vec<u16> = raw
                .as_chunks::<2>()
                .0
                .iter()
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .collect();
            // Keep one character per code unit so offsets stay in step; a surrogate pair becomes its character then a gap.
            let mut v = Vec::with_capacity(units.len());
            let mut k = 0;
            while k < units.len() {
                let u = units[k];
                if (0xD800..0xDC00).contains(&u) && k + 1 < units.len() {
                    let c = char::decode_utf16([u, units[k + 1]])
                        .next()
                        .and_then(|r| r.ok())
                        .unwrap_or('\u{FFFD}');
                    v.push(c);
                    v.push('\u{1F}');
                    k += 2;
                } else {
                    v.push(char::from_u32(u as u32).unwrap_or('\u{FFFD}'));
                    k += 1;
                }
            }
            v
        };
        for (k, c) in decoded.into_iter().enumerate() {
            let fc = (start + k * width) as u32;
            let p = if c == '\u{07}' || c == '\r' {
                lookup(&papx, fc)
            } else {
                0
            };
            chars.push(WordChar {
                c,
                row_end: c == '\u{07}' && p & ROW_END != 0,
                in_table: p & IN_TABLE != 0,
                bold: lookup(&chpx, fc) & BOLD != 0,
            });
        }
    }
    Ok(blocks_from_word_chars(&chars))
}

#[derive(Default)]
struct BoldCount {
    text: String,
    bold: usize,
    plain: usize,
}

impl BoldCount {
    fn push(&mut self, c: char, bold: bool) {
        if !c.is_whitespace() {
            if bold {
                self.bold += 1;
            } else {
                self.plain += 1;
            }
        }
        self.text.push(c);
    }
    fn is_bold(&self) -> bool {
        self.bold > 0 && self.plain == 0
    }
    fn has_text(&self) -> bool {
        self.bold + self.plain > 0
    }
}

/// Turns Word's internal text (paragraph mark 13, cell mark 7, fields 19/20/21) into blocks.
fn blocks_from_word_chars(chars: &[WordChar]) -> Document {
    // Drop field instructions, keeping each field's shown result, and map Word's special characters.
    let mut clean: Vec<WordChar> = Vec::with_capacity(chars.len());
    let mut fields: Vec<bool> = Vec::new(); // per open field: still in its instruction part?
    for &wc in chars {
        let mapped = match wc.c {
            '\u{13}' => {
                fields.push(true);
                None
            }
            '\u{14}' => {
                if let Some(last) = fields.last_mut() {
                    *last = false;
                }
                None
            }
            '\u{15}' => {
                fields.pop();
                None
            }
            _ if fields.iter().any(|&i| i) => None,
            '\u{1E}' => Some('-'),
            '\u{1F}' | '\u{01}' | '\u{08}' | '\u{02}' | '\u{05}' => None,
            '\u{0B}' => Some('\n'),
            '\u{0C}' => Some('\r'),
            '\u{A0}' => Some(' '),
            c => Some(c),
        };
        if let Some(c) = mapped {
            clean.push(WordChar { c, ..wc });
        }
    }
    let mut blocks = Vec::new();
    let mut rows: Vec<Vec<Cell>> = Vec::new();
    let mut row: Vec<Cell> = Vec::new();
    let mut cell_paras: Vec<BoldCount> = Vec::new();
    let mut current = BoldCount::default();
    let mut in_table_row = false;
    for wc in clean {
        match wc.c {
            '\r' => {
                let done = std::mem::take(&mut current);
                if wc.in_table || in_table_row || !row.is_empty() || !cell_paras.is_empty() {
                    cell_paras.push(done);
                } else {
                    if !rows.is_empty() {
                        blocks.push(Block::Table(std::mem::take(&mut rows)));
                    }
                    let bold = Some(done.is_bold());
                    blocks.push(Block::Para(Paragraph {
                        text: done.text,
                        bold,
                    }));
                }
            }
            '\u{07}' if wc.row_end => {
                rows.push(std::mem::take(&mut row));
                current = BoldCount::default();
                cell_paras.clear();
                in_table_row = false;
            }
            '\u{07}' => {
                cell_paras.push(std::mem::take(&mut current));
                let text = cell_paras
                    .iter()
                    .map(|p| p.text.as_str())
                    .collect::<Vec<_>>()
                    .join("\n");
                let visible: Vec<&BoldCount> = cell_paras.iter().filter(|p| p.has_text()).collect();
                let bold = Some(!visible.is_empty() && visible.iter().all(|p| p.is_bold()));
                cell_paras.clear();
                row.push(Cell { text, bold });
                in_table_row = true;
            }
            c => current.push(c, wc.bold),
        }
    }
    if !row.is_empty() {
        rows.push(row);
    }
    if !rows.is_empty() {
        blocks.push(Block::Table(rows));
    }
    if !current.text.is_empty() {
        let bold = Some(current.is_bold());
        blocks.push(Block::Para(Paragraph {
            text: current.text,
            bold,
        }));
    }
    Document { blocks }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Word text for tests: "~" stands for a row-end mark, "*x*" spans are bold.
    fn chars(s: &str) -> Vec<WordChar> {
        let mut bold = false;
        let mut out = Vec::new();
        for c in s.chars() {
            match c {
                '*' => bold = !bold,
                '~' => out.push(WordChar {
                    c: '\u{7}',
                    row_end: true,
                    in_table: true,
                    bold,
                }),
                '^' => out.push(WordChar {
                    c: '\r',
                    row_end: false,
                    in_table: true,
                    bold,
                }),
                c => out.push(WordChar {
                    c,
                    row_end: false,
                    in_table: false,
                    bold,
                }),
            }
        }
        out
    }

    #[test]
    fn pipe_lines_become_a_table() {
        let d = read_txt(b"Title\n|a |b |\n|c |d |\nafter\n");
        assert!(
            matches!(&d.blocks[1], Block::Table(rows) if rows.len() == 2 && rows[1][1].text == "d")
        );
    }

    #[test]
    fn empty_cells_are_not_row_ends() {
        let d = blocks_from_word_chars(&chars(
            "Title\r1.\u{7}\u{7}1\u{7}2\u{7}\u{7}~2.\u{7}hot\u{7}1\u{7}2\u{7}cold\u{7}~After\r",
        ));
        assert_eq!(d.blocks.len(), 3);
        match &d.blocks[1] {
            Block::Table(rows) => {
                assert_eq!(rows.len(), 2);
                assert_eq!(rows[0].len(), 5);
                assert_eq!(rows[0][1].text, "");
                assert_eq!(rows[1][4].text, "cold");
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_cell_with_two_paragraphs_stays_in_its_table() {
        let d = blocks_from_word_chars(&chars("Before\r1. first^second\u{7}.5\u{7}~After\r"));
        assert_eq!(d.blocks.len(), 3, "{:?}", d.blocks);
        assert!(
            matches!(&d.blocks[1], Block::Table(rows) if rows[0][0].text == "1. first\nsecond")
        );
    }

    #[test]
    fn bold_cells_and_paragraphs() {
        let d = blocks_from_word_chars(&chars("*Kept item*\rplain\r*1.*\u{7}*text*\u{7}~"));
        assert!(matches!(&d.blocks[0], Block::Para(p) if p.bold == Some(true)));
        assert!(matches!(&d.blocks[1], Block::Para(p) if p.bold == Some(false)));
        assert!(matches!(&d.blocks[2], Block::Table(rows) if rows[0][1].bold == Some(true)));
    }

    #[test]
    fn field_instructions_are_dropped() {
        let d = blocks_from_word_chars(&chars("See \u{13}HYPERLINK x\u{14}the link\u{15} here\r"));
        assert!(matches!(&d.blocks[0], Block::Para(p) if p.text == "See the link here"));
    }

    #[test]
    fn windows_1252_text_is_decoded() {
        let d = read_txt(b"don\x92t\n");
        assert!(matches!(&d.blocks[0], Block::Para(p) if p.text == "don\u{2019}t"));
    }
}
