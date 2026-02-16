use anyhow::{Context, anyhow};
use glob::glob;
use log::{error, warn};
use std::fs;
use std::path::{Path, PathBuf};

use crate::build::BUILDINFO_PATH_RELATIVE;
use crate::buildinfo::BuildInfo;

struct Platform {
    name: &'static str,
    target_glob: &'static str,
    fallback_glob: Option<&'static str>,
}

const PLATFORMS: &[Platform] = &[
    Platform {
        name: "x86_64-linux",
        target_glob: "*x86*linux*",
        fallback_glob: Some("*x86*windows*"),
    },
    // arm example:
    // Platform {
    //     name: "aarch64 linux",
    //     target_glob: "*(aarch64|x86)*linux*",
    //     fallback_glob: Some("*(aarch64|x86)*windows*"),
    // },
    Platform {
        name: "x86_64-windows",
        target_glob: "*x86*windows*",
        fallback_glob: None,
    },
];

pub fn command_split(src_dir: PathBuf) -> anyhow::Result<()> {
    if !fs::exists(&src_dir)? {
        return Err(anyhow!("Directory doesn't exist"));
    }

    for platform in PLATFORMS {
        let dir_name_str = src_dir.as_path().file_name().unwrap();
        let dst_dir = src_dir.parent().unwrap().join(format!(
            "{}_{}",
            dir_name_str.to_str().unwrap(),
            platform.name
        ));

        if fs::read_dir(&dst_dir)
            .map(|mut x| x.any(|_| true))
            .unwrap_or(false)
        {
            return Err(anyhow!(
                "Directory already exists, refusing to overwrite: {dst_dir:?}"
            ));
        }

        fs::create_dir_all(&dst_dir)?;

        let build_info: BuildInfo = fs::read_to_string(src_dir.join(BUILDINFO_PATH_RELATIVE))
            .context("failed to read buildinfo")?
            .parse()
            .context("failed to parse buildinfo")?;

        fs::copy(
            src_dir.join(BUILDINFO_PATH_RELATIVE),
            dst_dir.join(BUILDINFO_PATH_RELATIVE),
        )
        .context("failed to copy buildinfo")?;

        for project in build_info.projects {
            let src_proj_dir = src_dir.join(&project.name);
            let dst_proj_dir = dst_dir.join(&project.name);

            if fs::read_dir(&dst_proj_dir)
                .map(|mut x| x.any(|_| true))
                .unwrap_or(false)
            {
                return Err(anyhow!(
                    "Directory already exists, refusing to overwrite: {dst_dir:?}"
                ));
            }

            fs::create_dir_all(&dst_proj_dir)?;

            if !src_proj_dir.exists() {
                return Err(anyhow!("Project directory doesn't exist: {src_proj_dir:?}"));
            }

            let final_glob = format!(
                "{}/{}",
                src_proj_dir.as_os_str().to_str().unwrap(),
                platform.target_glob
            );
            let mut src_proj_target_dirs = glob(&final_glob)
                .context(format!("invalid glob pattern: {final_glob}"))?
                .collect::<Vec<_>>();

            if src_proj_target_dirs.is_empty() {
                if let Some(fallback_glob) = platform.fallback_glob {
                    warn!(
                        "project {} has no matching native binaries, falling back to {}",
                        project.name,
                        platform.fallback_glob.unwrap_or("none")
                    );

                    let final_glob = format!(
                        "{}/{}",
                        src_proj_dir.as_os_str().to_str().unwrap(),
                        fallback_glob
                    );
                    src_proj_target_dirs = glob(&final_glob)
                        .context(format!("invalid glob pattern: {final_glob}"))?
                        .collect::<Vec<_>>();
                } else {
                    error!(
                        "project {} has no matching native binaries, no fallback glob found platform {}",
                        project.name, platform.name
                    );
                    continue; // with other projects, don't halt the entire splitting
                }
            }

            if src_proj_target_dirs.is_empty() {
                error!(
                    "project {} has no matching native or fallback binaries for platform {}",
                    project.name, platform.name
                );
                continue; // with other projects, don't halt the entire splitting
            }

            for src_proj_target_dir in src_proj_target_dirs {
                let src_proj_target_dir = src_proj_target_dir.context("globbing failed")?;
                let dst_proj_target_dir = dst_proj_dir.join(
                    src_proj_target_dir
                        .file_name()
                        .ok_or_else(|| anyhow!("invalid file name"))?,
                );
                copy_dir_all(src_proj_target_dir, dst_proj_target_dir)
                    .context("couldn't copy {src_proj_target_dir:?} to {dst_proj_target_dir:?}")?;
            }

            // Copy other files in the root of the project dir
            // Used for bot.toml icons etc
            // This means that files that aren't in the root of the project dir
            // and also aren't in the platform specific folder WILL NOT BE INCLUDED
            // TODO: document this somewhere ^
            for path in src_proj_dir
                .read_dir()
                .context("failed to read project directory")?
            {
                let path = path.context("failed to read path")?;
                if !path.file_type()?.is_file() {
                    continue;
                }
                fs::copy(path.path(), dst_proj_dir.join(path.file_name()))
                    .context("failed to copy file")?;
            }
        }
    }

    Ok(())
}

fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> std::io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}
