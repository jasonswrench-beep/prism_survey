//! Prints an article's blocks in reading order: `cargo run -p prismsurvey-detect --example blocks -- <file> [first page] [last page]`
use prismsurvey_detect::layout::{reading_order, BlockKind};
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let first: u32 = a.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
    let last: u32 = a.get(2).and_then(|s| s.parse().ok()).unwrap_or(first);
    let art = prismsurvey_detect::article::read_article(std::path::Path::new(&a[0])).unwrap();
    for b in reading_order(&art.pages)
        .iter()
        .filter(|b| b.page >= first && b.page <= last)
    {
        match b.kind {
            BlockKind::Paragraph => println!(
                "p{} [{:.0},{:.0}] {}",
                b.page,
                b.x0,
                b.y0,
                b.text().chars().take(170).collect::<String>()
            ),
            BlockKind::Table => {
                println!(
                    "p{} [{:.0},{:.0}] TABLE {} rows",
                    b.page,
                    b.x0,
                    b.y0,
                    b.lines.len()
                );
                for r in b.rows().iter().take(30) {
                    println!(
                        "    | {}",
                        r.join(" | ").chars().take(170).collect::<String>()
                    );
                }
            }
        }
    }
}
