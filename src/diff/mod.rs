// diffing cli tool + built-in library that wraps qbsdiff
// we might be able to use something that already exists, but it'll need to support diffing of folders

use anyhow::anyhow;
use std::fs;
use std::path::PathBuf;

pub fn command(dir: PathBuf) -> anyhow::Result<()> {
    if !fs::exists(&dir)? {
        return Err(anyhow!("Directory doesn't exist"));
    }

    // 1. find all paths that we should diff with the .gitignore walker used in the bin_builder.rs file
    // 2. enum of different diff types (new(content), deleted, modified(qbsdiff))
    // 3. dir diff will be Vec<FileDiff> where FileDiff is a struct with path and diff_type

    Ok(())
}
