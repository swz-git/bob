use std::{
    fs,
    path::{Path, PathBuf},
    str::FromStr,
};

use anyhow::{anyhow, Context};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BobConfig {
    #[serde(default)]
    pub dependencies: Vec<PathBuf>,
    #[serde(rename = "config", default)]
    pub build_configs: Vec<BuildConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildConfig {
    pub project_name: String,
    pub bot_configs: Vec<PathBuf>,
    pub builder_config: BuilderConfig,
}

#[serde(tag = "builder_type")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BuilderConfig {
    #[serde(rename = "pyinstaller")]
    PyInstaller(PyInstallerBuildConfig),
    #[serde(rename = "rust")]
    Rust(RustBuildConfig),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PyInstallerBuildConfig {
    entry_file: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustBuildConfig {
    bin_name: String,
    targets: Vec<String>,
}

impl std::str::FromStr for BobConfig {
    type Err = anyhow::Error; // TODO: this feels wrong
    fn from_str(input: &str) -> anyhow::Result<Self> {
        toml::from_str(input).context("Parsing bob config failed")
    }
}

pub fn read_build_configs(
    root_config_path: PathBuf,
) -> anyhow::Result<Vec<(PathBuf, BuildConfig)>> {
    fn recurse(
        config_path: PathBuf,
        configs: &mut Vec<(PathBuf, BuildConfig)>,
    ) -> anyhow::Result<()> {
        let canonical_config_path = config_path.canonicalize()?;

        let str_content = fs::read_to_string(&config_path)
            .context(format!("reading bob config at {:?}", canonical_config_path))?;
        let config = BobConfig::from_str(&str_content)
            .context(format!("parsing bob config at {:?}", canonical_config_path))?;

        let dep_paths: Vec<_> = config
            .dependencies
            .iter()
            .map(|x| config_path.parent().unwrap().to_owned().join(x))
            .collect();
        for build_config in config.build_configs {
            configs.push((config_path.clone(), build_config));
        }
        for dep in dep_paths {
            recurse(dep, configs)?
        }
        Ok(())
    }

    let mut configs: Vec<(PathBuf, BuildConfig)> = vec![];
    recurse(root_config_path, &mut configs)?;
    Ok(configs)
}
