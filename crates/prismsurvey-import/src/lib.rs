//! Importing stored scale documents: read the file (`document`), then parse its layout cues into a scale (`stored`).

pub mod directory;
pub mod document;
pub mod stored;

pub use stored::{import_path, ImportReport};
