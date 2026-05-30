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
                eprintln!("skipping {}: {error}", file.display());
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
        println!(
            "Found {} duplicate(s) of {}",
            duplicates.len(),
            head.display()
        );
        for file in duplicates {
            if file == head {
                continue; // Just to be safe.
            }
            if dry_run {
                println!("Would remove {}", file.display());
            } else if let Err(error) = fs::remove_file(file) {
                eprintln!("failed to remove {}: {error}", file.display());
            } else {
                println!("Removed {}", file.display());
            }
        }
    }
}
