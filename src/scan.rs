use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Result;
use std::path::{Path, PathBuf};

use rayon::prelude::*;

/// Walk every root and group the discovered files by size.
pub fn collect(roots: &[String], recursive: bool) -> Result<HashMap<u64, Vec<PathBuf>>> {
    let paths = roots
        .iter()
        .map(|raw| fs::canonicalize(Path::new(raw)))
        .collect::<Result<Vec<_>>>()?;

    let pairs = paths
        .into_par_iter()
        .map(|root| walk(&root, recursive))
        .collect::<Result<Vec<_>>>()?;

    // If user fucks up we can reach the same file in multiple ways.
    let mut seen: HashSet<PathBuf> = HashSet::new();
    let mut files: HashMap<u64, Vec<PathBuf>> = HashMap::new();
    for (size, path) in pairs.into_iter().flatten() {
        if seen.insert(path.clone()) {
            files.entry(size).or_default().push(path);
        }
    }
    Ok(files)
}

fn walk(root: &Path, recursive: bool) -> Result<Vec<(u64, PathBuf)>> {
    let metadata = fs::symlink_metadata(root)?;
    let file_type = metadata.file_type();

    if file_type.is_symlink() {
        return Ok(Vec::new());
    }
    if file_type.is_file() {
        return Ok(vec![(metadata.len(), root.to_path_buf())]);
    }
    if !file_type.is_dir() {
        return Ok(Vec::new());
    }

    let entries = fs::read_dir(root)?
        .filter_map(|entry| match entry {
            Ok(entry) => Some(entry.path()),
            Err(err) => {
                tracing::warn!("skipping unreadable entry in {}: {err}", root.display());
                None
            }
        })
        .collect::<Vec<_>>();

    let files = entries
        .into_par_iter()
        .map(|entry| {
            let result = if recursive {
                walk(&entry, recursive)
            } else {
                fs::symlink_metadata(&entry).map(|metadata| {
                    if metadata.file_type().is_file() {
                        vec![(metadata.len(), entry.clone())]
                    } else {
                        Vec::new()
                    }
                })
            };
            result.unwrap_or_else(|error| {
                tracing::warn!("skipping {}: {error}", entry.display());
                Vec::new()
            })
        })
        .collect::<Vec<_>>()
        .into_iter()
        .flatten()
        .collect();

    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::io::Write;
    use tempfile::tempdir;

    /// Write `bytes` to `dir/name`, returning the canonicalized path.
    fn write_file(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
        let path = dir.join(name);
        let mut file = fs::File::create(&path).expect("create file");
        file.write_all(bytes).expect("write file");
        fs::canonicalize(&path).expect("canonicalize file")
    }

    fn root_string(path: &Path) -> String {
        path.to_str().expect("utf-8 path").to_string()
    }

    /// Flatten every discovered path out of the size-keyed map.
    fn all_paths(map: &HashMap<u64, Vec<PathBuf>>) -> HashSet<PathBuf> {
        map.values().flatten().cloned().collect()
    }

    #[test]
    fn groups_files_by_size() {
        let dir = tempdir().unwrap();
        write_file(dir.path(), "a.txt", b"22"); // 2 bytes
        write_file(dir.path(), "b.txt", b"33"); // 2 bytes
        write_file(dir.path(), "c.txt", b"444"); // 3 bytes

        let map = collect(&[root_string(dir.path())], false).unwrap();

        assert_eq!(map.get(&2).map(Vec::len), Some(2));
        assert_eq!(map.get(&3).map(Vec::len), Some(1));
        assert_eq!(all_paths(&map).len(), 3);
    }

    #[test]
    fn non_recursive_ignores_subdirectories() {
        let dir = tempdir().unwrap();
        let top = write_file(dir.path(), "top.txt", b"top");
        let sub = dir.path().join("nested");
        fs::create_dir(&sub).unwrap();
        write_file(&sub, "deep.txt", b"deep");

        let map = collect(&[root_string(dir.path())], false).unwrap();

        assert_eq!(all_paths(&map), HashSet::from([top]));
    }

    #[test]
    fn recursive_descends_into_subdirectories() {
        let dir = tempdir().unwrap();
        let top = write_file(dir.path(), "top.txt", b"top");
        let sub = dir.path().join("nested");
        fs::create_dir(&sub).unwrap();
        let deep = write_file(&sub, "deep.txt", b"deep");

        let map = collect(&[root_string(dir.path())], true).unwrap();

        assert_eq!(all_paths(&map), HashSet::from([top, deep]));
    }

    #[test]
    fn deduplicates_file_reached_via_multiple_roots() {
        let dir = tempdir().unwrap();
        let file = write_file(dir.path(), "only.txt", b"content");

        // Reach the same file twice: once via its directory, once directly.
        let roots = vec![root_string(dir.path()), root_string(&file)];
        let map = collect(&roots, false).unwrap();

        assert_eq!(all_paths(&map), HashSet::from([file]));
    }

    #[test]
    fn single_file_root_is_collected() {
        let dir = tempdir().unwrap();
        let file = write_file(dir.path(), "solo.txt", b"hello");

        let map = collect(&[root_string(&file)], false).unwrap();

        assert_eq!(map.get(&5).map(Vec::len), Some(1));
        assert_eq!(all_paths(&map), HashSet::from([file]));
    }

    #[test]
    fn empty_directory_yields_nothing() {
        let dir = tempdir().unwrap();
        let map = collect(&[root_string(dir.path())], true).unwrap();
        assert!(map.is_empty());
    }

    #[test]
    fn nonexistent_root_is_an_error() {
        let dir = tempdir().unwrap();
        let missing = dir.path().join("not-here");
        assert!(collect(&[root_string(&missing)], false).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn skips_symlinks_when_recursive() {
        let dir = tempdir().unwrap();
        let real = write_file(dir.path(), "real.txt", b"payload");
        std::os::unix::fs::symlink(&real, dir.path().join("link.txt")).unwrap();

        let map = collect(&[root_string(dir.path())], true).unwrap();

        assert_eq!(all_paths(&map), HashSet::from([real]));
    }

    #[cfg(unix)]
    #[test]
    fn skips_symlinks_when_not_recursive() {
        let dir = tempdir().unwrap();
        let real = write_file(dir.path(), "real.txt", b"payload");
        std::os::unix::fs::symlink(&real, dir.path().join("link.txt")).unwrap();

        let map = collect(&[root_string(dir.path())], false).unwrap();

        assert_eq!(all_paths(&map), HashSet::from([real]));
    }
}
