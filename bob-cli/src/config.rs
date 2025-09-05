use std::{
    borrow::Cow,
    fs,
    path::{Path, PathBuf},
    str::FromStr,
};

use anyhow::Context;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootConfig {
    #[serde(default)]
    pub dependencies: Vec<PathBuf>,
    #[serde(rename = "config", default)]
    pub configs: Vec<BobConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BobConfig {
    pub project_name: String,
    pub bot_configs: Vec<PathBuf>,
    pub builder_config: BuilderConfigVariant,
}

pub trait BuilderConfig: erased_serde::Serialize {
    fn get_dockerfile_contents(&self, project_root: &Path) -> anyhow::Result<Cow<'static, str>>;
}
erased_serde::serialize_trait_object!(BuilderConfig);

macro_rules! builder_configs {
    ($($i:ident $rename:literal $get_path:expr => $struct_contents:tt),+) => {
        #[derive(Debug, Clone, Serialize, Deserialize)]
        #[serde(tag = "builder_type")]
        pub enum BuilderConfigVariant {
            $(
                #[serde(rename = $rename)]
                $i(builder_configs::$i)
            ),+
        }
        impl BuilderConfigVariant {
            pub fn get_inner_as_generic(&self) -> &dyn BuilderConfig {
                match self {
                    $(
                        Self::$i(x) => x
                    ),+
                }
            }
        }
        pub mod builder_configs {
            // Allows invocations of the macro to use imported types
            use super::*;
            $(
                #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
                pub struct $i $struct_contents
                impl super::BuilderConfig for $i {
                    fn get_dockerfile_contents(&self, project_root: &Path)
                        -> anyhow::Result<std::borrow::Cow<'static, str>> {
                        $get_path(self, project_root).map(Into::into)
                    }
                }
            )+
        }
    };
}

builder_configs!(
    // InternalName "toml_name"
    // |_self: builder_configs::InternalName, project_root: &Path|
    //   todo!("should return the contents of a dockerfile")
    // => {
    //   field: Type,
    // },
    PyInstaller "pyinstaller"
    |_,_| Ok(include_str!("../dockerfiles/pyinstaller.Dockerfile"))
    => {
        entry_file: PathBuf,
    },
    Rust "rust"
    |_,_| Ok(include_str!("../dockerfiles/rust.Dockerfile"))
    => {
        bin_name: String,
        targets: Vec<String>,
    },
    Custom "custom"
    |s,r| {get_custom_dockerfile_contents(s,r)}
    => {
        pub dockerfile: PathBuf,
        pub values: Option<serde_value::Value>,
    }
);

fn get_custom_dockerfile_contents(
    s: &builder_configs::Custom,
    project_root: &Path,
) -> anyhow::Result<String> {
    let path = project_root.to_owned().join(&s.dockerfile);
    fs::read_to_string(&path).context(format!(
        "couldn't read file at custom dockerfile path `{path:?}`"
    ))
}

impl std::str::FromStr for RootConfig {
    type Err = anyhow::Error; // TODO: this feels wrong
    fn from_str(input: &str) -> anyhow::Result<Self> {
        toml::from_str(input).context("Parsing bob config failed")
    }
}

pub fn read_build_configs(root_config_path: PathBuf) -> anyhow::Result<Vec<(PathBuf, BobConfig)>> {
    fn recurse(
        config_path: PathBuf,
        configs: &mut Vec<(PathBuf, BobConfig)>,
    ) -> anyhow::Result<()> {
        let canonical_config_path = config_path.canonicalize()?;

        let str_content = fs::read_to_string(&config_path)
            .context(format!("reading bob config at {:?}", canonical_config_path))?;
        let root_config = RootConfig::from_str(&str_content)
            .context(format!("parsing bob config at {:?}", canonical_config_path))?;

        let config_path_parent = config_path.parent().unwrap().to_owned();
        let dep_paths = root_config
            .dependencies
            .iter()
            .flat_map(|sub_path| {
                glob::glob(config_path_parent.join(sub_path).to_str().unwrap())
                    .expect("Failed to read glob pattern")
            })
            .collect::<Result<Vec<_>, _>>()?;
        for build_config in root_config.configs {
            configs.push((config_path.clone(), build_config));
        }
        for dep in dep_paths {
            recurse(dep, configs)?
        }
        Ok(())
    }

    let mut configs: Vec<(PathBuf, BobConfig)> = vec![];
    recurse(root_config_path, &mut configs)?;
    Ok(configs)
}
