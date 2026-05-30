mod cli;
mod dedupe;
mod hash;
mod scan;

use std::io::{Result, stderr, stdout};
use std::path::PathBuf;

use clap::Parser;
use tracing::Level;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::writer::MakeWriterExt;

use cli::Args;
use dedupe::dedupe;

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(false)
        .with_writer(
            stderr
                .with_max_level(Level::WARN)
                .or_else(stdout.with_max_level(Level::INFO)),
        )
        .without_time()
        .init();

    let args = Args::parse();
    if args.threads == 0 {
        tracing::error!("number of threads must be positive");
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
