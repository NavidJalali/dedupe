mod cli;
mod dedupe;
mod hash;
mod scan;

use std::io::Result;
use std::path::PathBuf;

use clap::Parser;

use cli::Args;
use dedupe::dedupe;

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
    let candidates: Vec<PathBuf> = scan::collect(&args.roots, args.recursive)?
        .into_iter()
        .filter(|(_, files)| files.len() > 1)
        .flat_map(|(_, files)| files)
        .collect();
    dedupe(candidates, args.dry_run);
    Ok(())
}
