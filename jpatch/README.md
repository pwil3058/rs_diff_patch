# jpatch

An atomic, transactional patch applicator front-end binary that processes portable JSON patch files and securely maps
updates onto a target destination root directory workspace.

## Features

- **Transactional Rollbacks**: Features a native `-r, --reverse` mode flag that perfectly inverts patch tracking
  transactions, cleanly restoring deleted assets and pruning additions symmetrically.
- **Stream Piping Capabilities**: Seamlessly integrates with standard Unix shell streams (`stdin`), enabling tool
  composition workflows via pipeline links.
- **Atomic Operations**: Employs an index-renaming temporary layout strategy (`.tmp_patch`) ensuring text operations
  complete successfully on disk before overwriting target tracks.

## Installation

```bash
cargo install --path ./jpatch
```

## Usage Syntax

```bash
# Apply a generated JSON patch file forward onto a project directory
jpatch --input ~/changes.json --target-dir /var/www/production

# Symmetrically rollback an applied patch entirely to its baseline state
jpatch --input ~/changes.json --target-dir /var/www/production --reverse

# Stream patches directly over network links via SSH pipeline pipes
cat patch.json | ssh remote-server "jpatch --target-dir ./deploy"
```
