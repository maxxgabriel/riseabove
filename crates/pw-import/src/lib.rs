//! Building worlds: the FM-derived CSV import (`data/IMPORT_FORMAT.md`) and a
//! synthetic fixture used only by tests and benchmarks.

pub mod builder;
pub mod india;
mod csvimport;
mod positions;
pub mod synthetic;

pub use csvimport::{ImportError, ImportReport, load_dir, load_dir_seeded};
pub use positions::parse_positions;
