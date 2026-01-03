use anyhow::anyhow;
use std::{
    fs,
    io::{Read as _, Write as _, stdin, stdout},
    path::PathBuf,
};

use bob_lib::bobdiff::DirDiff;

pub fn command_diff(old: PathBuf, new: PathBuf) -> anyhow::Result<()> {
    if !fs::exists(&old)? {
        return Err(anyhow!("Directory doesn't exist: {old:?}"));
    }
    if !fs::exists(&new)? {
        return Err(anyhow!("Directory doesn't exist: {new:?}"));
    }

    // use the DirDiff struct to diff the directory
    let diff = DirDiff::new(&old, &new);

    stdout().write_all(&diff.ser())?;

    Ok(())
}

pub fn command_diff_apply(dir: PathBuf) -> anyhow::Result<()> {
    if !fs::exists(&dir)? {
        return Err(anyhow!("Directory doesn't exist"));
    }

    let mut serialized = Vec::new();
    stdin().read_to_end(&mut serialized)?;
    let diff = DirDiff::deser(&serialized)?;

    diff.apply_to(&dir, true)?;

    Ok(())
}
