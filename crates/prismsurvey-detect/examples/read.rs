//! Reads an article (text layer and OCR) and prints a page's words: `cargo run -p prismsurvey-detect --example read -- <file> [page]`
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let page: u32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
    let a = prismsurvey_detect::article::read_article(std::path::Path::new(&args[0])).unwrap();
    for n in &a.notes {
        println!("NOTE {n}");
    }
    for p in &a.pages {
        println!(
            "page {} {:?} words {} size {:.0}x{:.0}",
            p.number,
            p.source,
            p.words.len(),
            p.width,
            p.height
        );
    }
    if let Some(p) = a.pages.iter().find(|p| p.number == page) {
        let text: Vec<&str> = p.words.iter().map(|w| w.text.as_str()).collect();
        println!("{}", text.join(" ").chars().take(1500).collect::<String>());
    }
}
