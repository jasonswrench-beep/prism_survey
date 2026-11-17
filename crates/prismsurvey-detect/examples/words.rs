//! Summarises the text layer of PDFs: `cargo run -p prismsurvey-detect --example words -- <file or folder>...`
use prismsurvey_detect::pdf::{read_text_layer, TextSource};
fn main() {
    let mut files = Vec::new();
    for a in std::env::args().skip(1) {
        let p = std::path::PathBuf::from(&a);
        if p.is_dir() {
            for e in std::fs::read_dir(&p).unwrap().flatten() {
                if e.path()
                    .extension()
                    .is_some_and(|x| x.eq_ignore_ascii_case("pdf"))
                {
                    files.push(e.path());
                }
            }
        } else {
            files.push(p);
        }
    }
    files.sort();
    let show = std::env::var("SHOW_PAGE")
        .ok()
        .and_then(|s| s.parse::<u32>().ok());
    for f in files {
        let name = f
            .file_name()
            .unwrap()
            .to_string_lossy()
            .chars()
            .take(50)
            .collect::<String>();
        match read_text_layer(&std::fs::read(&f).unwrap()) {
            Ok(pages) => {
                let empty = pages
                    .iter()
                    .filter(|p| p.source == TextSource::None)
                    .count();
                let garbled = pages.iter().filter(|p| p.garbled).count();
                let failed = pages.iter().filter(|p| p.failed).count();
                let words: usize = pages.iter().map(|p| p.words.len()).sum();
                println!(
                    "{name:50} pages {:3} words {:6} no-text {:3} garbled {:3} failed {:3}",
                    pages.len(),
                    words,
                    empty,
                    garbled,
                    failed
                );
                if let Some(n) = show {
                    if let Some(p) = pages.iter().find(|p| p.number == n) {
                        for w in p.words.iter().take(400) {
                            println!(
                                "  {:6.1} {:6.1} {:6.1} {:5.1} {}",
                                w.x0, w.y0, w.x1, w.size, w.text
                            );
                        }
                    }
                }
            }
            Err(e) => println!("{name:50} ERROR {e}"),
        }
    }
}
