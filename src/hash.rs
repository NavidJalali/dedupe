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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    /// Write `bytes` to a fresh file inside `dir` and return its path.
    fn write_file(dir: &Path, name: &str, bytes: &[u8]) -> std::path::PathBuf {
        let path = dir.join(name);
        let mut file = fs::File::create(&path).expect("create temp file");
        file.write_all(bytes).expect("write temp file");
        path
    }

    /// The expected hex digest for `bytes`, computed via blake3's one-shot API.
    fn expected_hex(bytes: &[u8]) -> String {
        blake3::hash(bytes).to_hex().to_string()
    }

    #[test]
    fn hashes_match_blake3_one_shot() {
        let dir = tempdir().unwrap();
        let content = b"the quick brown fox jumps over the lazy dog";
        let path = write_file(dir.path(), "fox.txt", content);
        assert_eq!(hash_file(&path).unwrap(), expected_hex(content));
    }

    #[test]
    fn hashes_empty_file() {
        let dir = tempdir().unwrap();
        let path = write_file(dir.path(), "empty.txt", b"");
        assert_eq!(hash_file(&path).unwrap(), expected_hex(b""));
    }

    #[test]
    fn identical_content_hashes_equally() {
        let dir = tempdir().unwrap();
        let a = write_file(dir.path(), "a.txt", b"same bytes");
        let b = write_file(dir.path(), "b.txt", b"same bytes");
        assert_eq!(hash_file(&a).unwrap(), hash_file(&b).unwrap());
    }

    #[test]
    fn different_content_hashes_differently() {
        let dir = tempdir().unwrap();
        let a = write_file(dir.path(), "a.txt", b"alpha");
        let b = write_file(dir.path(), "b.txt", b"beta");
        assert_ne!(hash_file(&a).unwrap(), hash_file(&b).unwrap());
    }

    #[test]
    fn larger_than_buffer_still_matches() {
        // Exceed the BufReader capacity so the streaming copy spans multiple reads.
        let dir = tempdir().unwrap();
        let content = vec![0xABu8; 256 * 1024];
        let path = write_file(dir.path(), "big.bin", &content);
        assert_eq!(hash_file(&path).unwrap(), expected_hex(&content));
    }

    #[test]
    fn missing_file_is_an_error() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("does-not-exist");
        assert!(hash_file(&path).is_err());
    }
}
