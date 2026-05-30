use clap::Parser;

/// Find and remove duplicate files by comparing their contents.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Report what would be removed without deleting anything.
    #[arg(short, long, default_value_t = false)]
    pub dry_run: bool,
    /// Descend into subdirectories instead of only scanning the roots directly.
    #[arg(short, long, default_value_t = false)]
    pub recursive: bool,
    /// Number of worker threads to use (defaults to the available parallelism).
    #[arg(short, long, default_value_t = default_threads())]
    pub threads: usize,
    /// Files and directories to scan for duplicates.
    #[arg(required = true)]
    pub roots: Vec<String>,
}

fn default_threads() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
}
