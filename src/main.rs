use std::collections::HashMap;
use std::path::{Path, PathBuf};

use blake3::Hasher;
use clap::*;
use std::fs;
use std::io::{BufReader, Result, copy};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value_t = false)]
    dry_run: bool,
    #[arg(short, long, default_value_t = false)]
    recursive: bool,
    #[arg(required = true)]
    roots: Vec<String>,
}

impl Args {
    fn files(&self) -> Result<HashMap<u64, Vec<PathBuf>>> {
        let mut files = HashMap::new();
        let paths = self
            .roots
            .iter()
            .map(|raw| fs::canonicalize(Path::new(raw)))
            .collect::<Result<Vec<_>>>()?;
        for root in paths {
            Self::walk(root, self.recursive, &mut files)?;
        }
        Ok(files)
    }

    fn walk(
        root: PathBuf,
        recursive: bool,
        accumulator: &mut HashMap<u64, Vec<PathBuf>>,
    ) -> Result<()> {
        let metadata = fs::metadata(&root)?;
        if metadata.is_file() {
            let full_path = root;
            let size = metadata.len();
            accumulator.entry(size).or_default().push(full_path);
        } else if metadata.is_dir() && recursive {
            let mut entries = fs::read_dir(root)?;
            while let Some(entry) = entries.next() {
                let entry = entry?.path();
                Self::walk(entry, recursive, accumulator)?;
            }
        }
        Ok(())
    }
}

fn dedupe(files: Vec<PathBuf>, dry_run: bool) -> Result<()> {
    let mut file_by_hash = HashMap::new();
    for file in files {
        let mut hasher = Hasher::new();
        let mut reader = BufReader::new(fs::File::open(&file)?);
        copy(&mut reader, &mut hasher)?;
        let hash = hasher.finalize().to_hex().to_string();
        file_by_hash.entry(hash).or_insert_with(Vec::new).push(file);
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
    let files = args.files()?;
    for (_, files) in files {
        dedupe(files, args.dry_run)?;
    }
    Ok(())
}
