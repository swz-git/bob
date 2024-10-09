use std::{
    fs,
    path::{Path, PathBuf},
    str::FromStr,
};

use anyhow::{anyhow, Context};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct BobConfig {
    #[serde(default)]
    pub dependencies: Vec<PathBuf>,
    #[serde(rename = "config", default)]
    pub build_configs: Vec<BuildConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NuitkaBuildConfig {
    entry_file: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RustBuildConfig {
    cargo_toml_dir: PathBuf,
    bin_name: String,
}

#[serde(tag = "type")]
#[derive(Debug, Serialize, Deserialize)]
pub enum BuildConfig {
    #[serde(rename = "nuitka")]
    Nuitka(NuitkaBuildConfig),
    #[serde(rename = "rust")]
    Rust(RustBuildConfig),
}

impl std::str::FromStr for BobConfig {
    type Err = anyhow::Error; // TODO: this feels wrong
    fn from_str(input: &str) -> anyhow::Result<Self> {
        toml::from_str(input).context("Parsing bob config failed")
    }
}

fn read_build_configs_recursive(
    config_path: PathBuf,
    configs: &mut Vec<(PathBuf, BuildConfig)>,
) -> anyhow::Result<()> {
    let str_content = fs::read_to_string(&config_path).context(format!(
        "reading bob config at {:?}",
        config_path.canonicalize()?
    ))?;
    let config = BobConfig::from_str(&str_content).context("parsing bob config")?;
    let dep_paths: Vec<_> = config
        .dependencies
        .iter()
        .map(|x| config_path.parent().unwrap().to_owned().join(x))
        .collect();
    for build_config in config.build_configs {
        configs.push((config_path.clone(), build_config));
    }
    for dep in dep_paths {
        read_build_configs_recursive(dep, configs)?
    }
    Ok(())
}

pub fn read_build_configs(
    root_config_path: PathBuf,
) -> anyhow::Result<Vec<(PathBuf, BuildConfig)>> {
    let mut configs: Vec<(PathBuf, BuildConfig)> = vec![];
    read_build_configs_recursive(root_config_path, &mut configs)?;
    Ok(configs)
}
