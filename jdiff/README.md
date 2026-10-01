# jdiff

A fast, ergonomic CLI front-end binary driven by a **smart flag inference engine** that automatically deduces tracking
intentions from path counts and revision flags.

## Ergonomic Interface Matrix

The tool evaluates terminal invocations and adjusts behavior on-the-fly:

- **0 Paths**: Scans your entire active Git workspace directory, comparing local disk files against the `HEAD` index
  cache.
- **0 Paths + 2 Revisions (`-r rev1 -r rev2`)**: Compares an entire project layout between two historical commits
  straight from Git's database without checking out files.
- **1 Path**: Analyzes modifications for a specific text file or subfolder against the current Git tracking index.
- **1 Path + 1 Revision (`-r rev1`)**: Evaluates a file asset state against a specific tag, branch, or relative
  reference (e.g., `HEAD~2`).
- **2 Paths**: Runs an ordinary, plain filesystem directory diff between two distinct folder locations on disk.

## Installation

```bash
cargo install --path ./jdiff
```

## Global Command Line Options

```text
Arguments:
  [PATHS]...  Zero, one, or two target file system paths to evaluate

Options:
  -r, --revision <REV>  Git revision handles (e.g., branch, or HEAD~1). Pass up to two times.
  -c, --context <INT>   Number of context lines to anchor modifications [default: 3]
  -e, --exclude <STR>   Paths or folder names to exclude from directory scans
      --title <STR>     Custom title metadata attribute for the patch envelope header
      --output <PATH>   Output path for the generated patch file (defaults to stdout)
      --no-pretty       Prevent JSON pretty-printing to compress the output file footprint size
```

## Production Examples

```bash
# Ingest working modifications while cutting out target cache structures
jdiff -e target > ~/changes.json

# Diff the entire project tree between two historical releases
jdiff -r v1.0.0 -r v1.1.0 --output release_patch.json
```
