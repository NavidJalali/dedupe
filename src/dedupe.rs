use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use rayon::prelude::*;

use crate::hash::hash_file;

pub fn dedupe(files: Vec<PathBuf>, dry_run: bool) {
    let hashed: Vec<(String, PathBuf)> = files
        .into_par_iter()
        .filter_map(|file| match hash_file(&file) {
            Ok(hash) => Some((hash, file)),
            Err(error) => {
                tracing::warn!("skipping {}: {error}", file.display());
                None
            }
        })
        .collect();

    let mut file_by_hash: HashMap<String, Vec<PathBuf>> = HashMap::new();
    for (hash, file) in hashed {
        file_by_hash.entry(hash).or_default().push(file);
    }

    for (_, mut group) in file_by_hash {
        if group.len() < 2 {
            continue;
        }
        group.sort();
        let (head, duplicates) = group.split_first().expect("non-empty group");
        tracing::info!(
            "Found {} duplicate(s) of {}",
            duplicates.len(),
            head.display()
        );
        for file in duplicates {
            if file == head {
                continue; // Just to be safe.
            }
            if dry_run {
                tracing::info!("Would remove {}", file.display());
            } else if let Err(error) = fs::remove_file(file) {
                tracing::error!("failed to remove {}: {error}", file.display());
            } else {
                tracing::info!("Removed {}", file.display());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    /// Write `bytes` to `dir/name`, returning the canonicalized path.
    fn write_file(dir: &std::path::Path, name: &str, bytes: &[u8]) -> PathBuf {
        let path = dir.join(name);
        let mut file = fs::File::create(&path).expect("create file");
        file.write_all(bytes).expect("write file");
        fs::canonicalize(&path).expect("canonicalize file")
    }

    #[test]
    fn removes_duplicates_keeping_lexicographically_smallest() {
        let dir = tempdir().unwrap();
        let a = write_file(dir.path(), "a.txt", b"same");
        let b = write_file(dir.path(), "b.txt", b"same");

        // Pass them out of order to prove the kept file comes from sorting.
        dedupe(vec![b.clone(), a.clone()], false);

        assert!(a.exists(), "smallest name should be kept");
        assert!(!b.exists(), "later name should be removed");
    }

    #[test]
    fn dry_run_removes_nothing() {
        let dir = tempdir().unwrap();
        let a = write_file(dir.path(), "a.txt", b"same");
        let b = write_file(dir.path(), "b.txt", b"same");

        dedupe(vec![a.clone(), b.clone()], true);

        assert!(a.exists());
        assert!(b.exists());
    }

    #[test]
    fn distinct_files_are_untouched() {
        let dir = tempdir().unwrap();
        let a = write_file(dir.path(), "a.txt", b"alpha");
        let b = write_file(dir.path(), "b.txt", b"beta");

        dedupe(vec![a.clone(), b.clone()], false);

        assert!(a.exists());
        assert!(b.exists());
    }

    #[test]
    fn keeps_only_one_among_many_duplicates() {
        let dir = tempdir().unwrap();
        let a = write_file(dir.path(), "a.txt", b"same");
        let b = write_file(dir.path(), "b.txt", b"same");
        let c = write_file(dir.path(), "c.txt", b"same");

        dedupe(vec![c.clone(), a.clone(), b.clone()], false);

        assert!(a.exists());
        assert!(!b.exists());
        assert!(!c.exists());
    }

    #[test]
    fn deduplicates_each_group_independently() {
        let dir = tempdir().unwrap();
        let a1 = write_file(dir.path(), "a1.txt", b"groupA");
        let a2 = write_file(dir.path(), "a2.txt", b"groupA");
        let b1 = write_file(dir.path(), "b1.txt", b"groupB");
        let b2 = write_file(dir.path(), "b2.txt", b"groupB");

        dedupe(vec![a1.clone(), a2.clone(), b1.clone(), b2.clone()], false);

        assert!(a1.exists());
        assert!(!a2.exists());
        assert!(b1.exists());
        assert!(!b2.exists());
    }

    #[test]
    fn single_file_is_untouched() {
        let dir = tempdir().unwrap();
        let a = write_file(dir.path(), "a.txt", b"lonely");

        dedupe(vec![a.clone()], false);

        assert!(a.exists());
    }

    #[test]
    fn unhashable_files_are_skipped_without_panicking() {
        let dir = tempdir().unwrap();
        let a = write_file(dir.path(), "a.txt", b"same");
        let b = write_file(dir.path(), "b.txt", b"same");
        let ghost = dir.path().join("ghost.txt"); // never created

        dedupe(vec![a.clone(), ghost, b.clone()], false);

        assert!(a.exists());
        assert!(!b.exists());
    }

    #[test]
    fn duplicate_path_entries_do_not_self_delete() {
        // The same path listed twice must not be removed: the head guard
        // protects the single underlying file.
        let dir = tempdir().unwrap();
        let a = write_file(dir.path(), "a.txt", b"same");

        dedupe(vec![a.clone(), a.clone()], false);

        assert!(a.exists());
    }
}
