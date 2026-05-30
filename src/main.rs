use std::collections::HashMap;
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
            .map(|root| Self::walk(root, self.recursive))
            .collect::<Result<Vec<_>>>()?;

        let mut files: HashMap<u64, Vec<PathBuf>> = HashMap::new();
        for (size, path) in pairs.into_iter().flatten() {
            files.entry(size).or_default().push(path);
        }
        Ok(files)
    }

    fn walk(root: PathBuf, recursive: bool) -> Result<Vec<(u64, PathBuf)>> {
        let metadata = fs::metadata(&root)?;
        if metadata.is_file() {
            Ok(vec![(metadata.len(), root)])
        } else if metadata.is_dir() && recursive {
            let entries = fs::read_dir(root)?
                .map(|entry| Ok(entry?.path()))
                .collect::<Result<Vec<_>>>()?;

            let nested = entries
                .into_par_iter()
                .map(|entry| Self::walk(entry, recursive))
                .collect::<Result<Vec<_>>>()?;

            Ok(nested.into_iter().flatten().collect())
        } else {
            Ok(Vec::new())
        }
    }
}

fn hash_file(path: &Path) -> Result<String> {
    let mut hasher = Hasher::new();
    let mut reader = BufReader::new(fs::File::open(path)?);
    copy(&mut reader, &mut hasher)?;
    Ok(hasher.finalize().to_hex().to_string())
}

fn dedupe(files: Vec<PathBuf>, dry_run: bool) -> Result<()> {
    let hashed = files
        .into_par_iter()
        .map(|file| Ok((hash_file(&file)?, file)))
        .collect::<Result<Vec<_>>>()?;

    let mut file_by_hash: HashMap<String, Vec<PathBuf>> = HashMap::new();
    for (hash, file) in hashed {
        file_by_hash.entry(hash).or_default().push(file);
    }
    for (_, files) in file_by_hash {
        if files.len() > 1 {
            let (head, tail) = files.split_first().expect("non empty files");
            println!("Found {} duplicates of {head:?}", tail.len());
            for file in tail {
                if dry_run {
                    println!("Would remove {file:?}");
                } else {
                    fs::remove_file(file)?;
                    println!("Removed {file:?}");
                }
            }
        }
    }
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();
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
    dedupe(candidates, args.dry_run)?;
    Ok(())
}
