//! Saves: bincode → lz4, written to a temp file then renamed (crash-safe, 01 §8).

use std::io::Write;
use std::path::Path;

use serde::Serialize;
use serde::de::DeserializeOwned;

const MAGIC: &[u8; 8] = b"PWSAVE01";

#[derive(Debug, thiserror::Error)]
pub enum SaveError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("encode: {0}")]
    Encode(#[from] bincode::Error),
    #[error("not a Pathway save or unsupported version")]
    Format,
    #[error("decompress: {0}")]
    Decompress(#[from] lz4_flex::block::DecompressError),
}

pub fn save<T: Serialize>(value: &T, path: &Path) -> Result<(), SaveError> {
    let raw = bincode::serialize(value)?;
    let packed = lz4_flex::compress_prepend_size(&raw);
    let tmp = path.with_extension("tmp");
    {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(MAGIC)?;
        f.write_all(&packed)?;
        f.sync_all()?;
    }
    std::fs::rename(&tmp, path)?;
    Ok(())
}

pub fn load<T: DeserializeOwned>(path: &Path) -> Result<T, SaveError> {
    let bytes = std::fs::read(path)?;
    if bytes.len() < MAGIC.len() || &bytes[..MAGIC.len()] != MAGIC {
        return Err(SaveError::Format);
    }
    let raw = lz4_flex::decompress_size_prepended(&bytes[MAGIC.len()..])?;
    Ok(bincode::deserialize(&raw)?)
}
