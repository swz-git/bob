use std::{
    fs,
    hash::{Hash as _, Hasher as _},
    path::PathBuf,
};

use anyhow::Context as _;
use rapidhash::RapidInlineHasher;

pub(crate) mod diff;
pub mod bobdiff {
    pub use crate::diff::*;
}

pub fn dirhasher(dir: PathBuf) -> anyhow::Result<u64> {
    let dir = dir.canonicalize()?;

    let mut paths = vec![];

    for result in ignore::WalkBuilder::new(&dir)
        .hidden(true)
        .git_ignore(true)
        .build()
    {
        let path = result?.into_path();
        if path.is_file() {
            paths.push(path)
        }
    }

    paths.sort();

    let mut hasher = RapidInlineHasher::default();

    for path in paths {
        let content = fs::read(&path).context("hasher couldn't read file")?;
        path.canonicalize()?.strip_prefix(&dir)?.hash(&mut hasher);
        content.hash(&mut hasher);
    }

    Ok(hasher.finish())
}
