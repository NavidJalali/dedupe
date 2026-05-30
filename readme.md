# dedupe

Find and remove duplicate files by comparing their contents

## Usage

```sh
dedupe [OPTIONS] <ROOTS>...
```

### Arguments

- `<ROOTS>...` — Files and directories to scan for duplicates.

### Options

- `-d, --dry-run` — Report what would be removed without deleting anything.
- `-r, --recursive` — Descend into subdirectories instead of only scanning the roots directly.
- `-t, --threads <THREADS>` — Number of worker threads to use (defaults to the available parallelism).
- `-h, --help` — Print help.
- `-V, --version` — Print version.

### Examples

```sh
# Preview duplicates under ./photos without deleting anything
dedupe --dry-run --recursive ./photos

# Remove duplicates across two directories
dedupe ./downloads ./photos
```
