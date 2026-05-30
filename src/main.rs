use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use blake3::Hasher;
use clap::*;
use rayon::prelude::*;
use std::fs;
use std::io::{BufReader, Result, copy};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value_t = false)]
    dry_run: bool,
    #[arg(short, long, default_value_t = false)]
    recursive: bool,
    #[arg(short, long, default_value_t = default_threads())]
    threads: usize,
    #[arg(required = true)]
    roots: Vec<String>,
}

fn default_threads() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
}

impl Args {
    fn files(&self) -> Result<HashMap<u64, Vec<PathBuf>>> {
        let paths = self
            .roots
            .iter()
            .map(|raw| fs::canonicalize(Path::new(raw)))
            .collect::<Result<Vec<_>>>()?;

        let pairs = paths
            .into_par_iter()
            .map(|root| Self::walk(&root, self.recursive))
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
                    Self::walk(&entry, recursive)
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
}

fn hash_file(path: &Path) -> Result<String> {
    let mut hasher = Hasher::new();
    let mut reader = BufReader::new(fs::File::open(path)?);
    copy(&mut reader, &mut hasher)?;
    Ok(hasher.finalize().to_hex().to_string())
}

fn dedupe(files: Vec<PathBuf>, dry_run: bool) {
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

fn main() -> Result<()> {
    let args = Args::parse();
    if args.threads == 0 {
        eprintln!("number of threads must be positive");
        std::process::exit(1);
    }
    rayon::ThreadPoolBuilder::new()
        .num_threads(args.threads)
        .build_global()
        .expect("failed to configure thread pool");
    let candidates: Vec<PathBuf> = args
        .files()?
        .into_iter()
        .filter(|(_, files)| files.len() > 1)
        .flat_map(|(_, files)| files)
        .collect();
    dedupe(candidates, args.dry_run);
    Ok(())
}
