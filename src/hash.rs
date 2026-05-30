use std::fs;
use std::io::{BufReader, Result, copy};
use std::path::Path;

use blake3::Hasher;

pub fn hash_file(path: &Path) -> Result<String> {
    let mut hasher = Hasher::new();
    let mut reader = BufReader::new(fs::File::open(path)?);
    copy(&mut reader, &mut hasher)?;
    Ok(hasher.finalize().to_hex().to_string())
}
