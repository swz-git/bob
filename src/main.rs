use std::{env, fs, io::Write, path::PathBuf};

use anyhow::anyhow;
use config::read_build_configs;

mod builder;
mod config;

fn main() -> anyhow::Result<()> {
    let arg = env::args()
        .skip(1)
        .next()
        .ok_or(anyhow!("Couldn't read first argument"))?;

    let path = PathBuf::from(arg);
    if !fs::exists(&path)? {
        return Err(anyhow!("File doesn't exist"));
    }

    let build_configs = read_build_configs(path)?;

    for (p, build_config) in build_configs {
        // TODO: cache hashes
        let result = builder::build(p.clone(), build_config, None)?;
        fs::File::create(p.parent().unwrap().join("build.bin"))?
            .write_all(&result.unwrap().binary)?;
    }

    Ok(())
}
