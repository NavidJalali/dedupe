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

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn command_definition_is_valid() {
        // clap's own lint for conflicting/ill-formed argument definitions.
        Args::command().debug_assert();
    }

    #[test]
    fn default_threads_is_positive() {
        assert!(default_threads() >= 1);
    }

    #[test]
    fn defaults_when_only_roots_given() {
        let args = Args::try_parse_from(["dedupe", "/some/path"]).unwrap();
        assert!(!args.dry_run);
        assert!(!args.recursive);
        assert_eq!(args.threads, default_threads());
        assert_eq!(args.roots, vec!["/some/path".to_string()]);
    }

    #[test]
    fn short_flags_parse() {
        let args = Args::try_parse_from(["dedupe", "-d", "-r", "-t", "4", "p1", "p2"]).unwrap();
        assert!(args.dry_run);
        assert!(args.recursive);
        assert_eq!(args.threads, 4);
        assert_eq!(args.roots, vec!["p1".to_string(), "p2".to_string()]);
    }

    #[test]
    fn long_flags_parse() {
        let args =
            Args::try_parse_from(["dedupe", "--dry-run", "--recursive", "--threads", "8", "p"])
                .unwrap();
        assert!(args.dry_run);
        assert!(args.recursive);
        assert_eq!(args.threads, 8);
        assert_eq!(args.roots, vec!["p".to_string()]);
    }

    #[test]
    fn roots_are_required() {
        assert!(Args::try_parse_from(["dedupe"]).is_err());
    }

    #[test]
    fn non_numeric_threads_is_rejected() {
        assert!(Args::try_parse_from(["dedupe", "-t", "abc", "p"]).is_err());
    }
}
