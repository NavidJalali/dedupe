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
                eprintln!("skipping unreadable entry in {}: {err}", root.display());
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
                eprintln!("skipping {}: {error}", entry.display());
                Vec::new()
            })
        })
        .collect::<Vec<_>>()
        .into_iter()
        .flatten()
        .collect();

    Ok(files)
}
