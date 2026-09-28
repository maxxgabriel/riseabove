//! Building worlds: the FM-derived CSV import (`data/IMPORT_FORMAT.md`) and a
//! synthetic fixture used only by tests and benchmarks.

pub mod builder;
mod csvimport;
mod positions;
pub mod synthetic;

pub use csvimport::{ImportError, ImportReport, load_dir};
pub use positions::parse_positions;
