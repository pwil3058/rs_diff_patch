# json_diff_lib

A high-performance, type-safe Rust differential and transactional directory patching core engine. It generates
human-reviewable, portable JSON patch manifests tracking differences across loose directories or active Git
repositories.

## Features

- **Type-System Enforced Pruning**: Employs an idiomatic compile-time `Option<Self>` constructor design pattern that
  guarantees clean, unmodified file records never leak into final patch streams.
- **Git Object Database Ingest**: Connects directly to local repository backing structures via `git2` to pull historical
  tracking states out of virtual blobs into volatile memory blocks.
- **Unified Metadata Container**: Embeds type-safe `FileMeta` slots tracking short 7-character commit identifiers
  (`Commit`) or human-readable local modification datetimes (`Modified`) in lockstep.
- **Dynamic Program Table Optimizations**: Employs an allocations-reused, pre-allocated hash row swapping mechanism that
  completely cuts heap-thrashing overhead out of hot loops.

## Core API Usage

```rust
use std::path::Path;
use json_diff_lib::dir_diff_scanner::DirDiffScanner;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let repo_root = Path::new(".");
    let context_lines = 3;
    let exclusions = vec![String::from("target")];

    // Compute active uncommitted modifications against HEAD baseline snapshots
    let patch_set = DirDiffScanner::compare_git_workspace(
        repo_root, 
        context_lines, 
        &exclusions
    )?;

    // Serialize to standard stream outputs
    patch_set.to_writer(std::io::stdout().lock(), true)?;
    Ok(())
}
```
