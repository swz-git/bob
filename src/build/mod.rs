use std::{
    fs,
    io::{Cursor, Read as _, Write as _},
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    str::FromStr,
};

use crate::{
    buildinfo::{BuildInfo, Project},
    config::read_build_configs,
    BuildCommand,
};
use anyhow::{anyhow, Context as _};
use log::info;

mod bin_builder;

const BUILDINFO_PATH_RELATIVE: &str = "./buildinfo.toml";

pub fn command(build_command: BuildCommand) -> anyhow::Result<()> {
    if !fs::exists(&build_command.config_path)? {
        return Err(anyhow!("File doesn't exist"));
    }

    let build_configs = read_build_configs(build_command.config_path.clone())?;

    // If parsing/reading fails, we just ignore previous build info and remove previous build files
    let build_info_prev = fs::read_to_string(build_command.out_dir.join(BUILDINFO_PATH_RELATIVE))
        .ok()
        .and_then(|s| BuildInfo::from_str(&s).ok());

    let mut build_info = BuildInfo::new();

    for (bob_toml_path, build_config) in build_configs {
        let proj_src_root_dir = bob_toml_path
            .canonicalize()
            .context("bob config parent dir doesn't exist")?
            .parent()
            .ok_or(anyhow!("couldn't get parent dir of bob config"))?
            .canonicalize()?;
        drop(bob_toml_path);

        let proj_build_root_dir = build_command.out_dir.join(&build_config.project_name);

        let prev_project_info = proj_build_root_dir
            .exists()
            .then(|| {
                build_info_prev.as_ref().and_then(|x| {
                    x.projects
                        .iter()
                        .find(|x| x.name == build_config.project_name)
                })
            })
            .flatten();

        if let Some(bin_build_result) = bin_builder::build(
            proj_src_root_dir.to_owned(),
            &build_config,
            prev_project_info.map(|x| x.hash),
        )? {
            build_info.projects.push(Project {
                name: build_config.project_name.clone(),
                hash: bin_build_result.dir_hash,
                build_date: chrono::Local::now().into(),
            });

            if let Err(e) = fs::remove_dir_all(&proj_build_root_dir) {
                if e.kind() != std::io::ErrorKind::NotFound {
                    return Err(e).context("Couldn't clear old project dir in bob_build");
                }
            };
            fs::create_dir_all(&proj_build_root_dir)
                .context("Couldn't create project dir for bob_build")?;

            info!("Decompressing built binaries...");

            let (windows_binary_path, linux_binary_path) =
                build_bot_bins(bin_build_result.tar_binary, &proj_build_root_dir)?;

            build_bot_tomls(
                &build_config
                    .bot_configs
                    .iter()
                    .map(|x| proj_src_root_dir.join(x))
                    .collect::<Vec<_>>(),
                &build_config.project_name,
                &proj_src_root_dir,
                &proj_build_root_dir,
                windows_binary_path,
                linux_binary_path,
            )
            .context(format!(
                "Couldn't build bot tomls for project {}",
                &build_config.project_name
            ))?;
        } else {
            let prev_project_info = prev_project_info.unwrap();
            // Old is good
            build_info.projects.push(Project {
                name: build_config.project_name.clone(),
                hash: prev_project_info.hash,
                build_date: prev_project_info.build_date,
            });
        };

        fs::File::create(build_command.out_dir.join("buildinfo.toml"))?
            .write_all(build_info.to_string().as_bytes())?;
    }

    info!("Copy of buildinfo.toml:\n{}", build_info.to_string().trim());
    info!("Done!");

    Ok(())
}

fn build_bot_bins(
    bin: Vec<u8>,
    proj_build_root_dir: &Path,
) -> anyhow::Result<(Option<PathBuf>, Option<PathBuf>)> {
    let mut windows_binary_path = None;
    let mut linux_binary_path = None;

    let mut archive = tar::Archive::new(Cursor::new(bin));
    for entry in archive
        .entries()
        .context("Couldn't build entries in built tar file")?
    {
        let entry = entry.context("Couldn't read entry in built tar file")?;
        if entry.header().entry_type().is_dir() {
            continue;
        }
        let entry_path = entry
            .path()
            .context("Couldn't read path of entry in built tar file")?
            .into_owned();
        let path_in_build = proj_build_root_dir.join(&entry_path);
        fs::create_dir_all(path_in_build.parent().unwrap())
            .context("Couldn't create dir in bob_build")?;
        let entry_mode = entry.header().mode().unwrap_or_default();
        let bytes = entry.bytes().map(|x| x.unwrap()).collect::<Vec<u8>>();

        match (
            infer::get(&bytes).map(|x| x.mime_type()),
            entry_path
                .file_name()
                .and_then(|x| x.to_str())
                .unwrap_or("")
                .to_owned(),
        ) {
            (Some("application/vnd.microsoft.portable-executable"), file_name)
                if file_name.ends_with(".exe") =>
            {
                windows_binary_path = Some(path_in_build.clone())
            }
            (Some("application/x-executable"), file_name)
                if !file_name.starts_with("lib") && !file_name.ends_with("so") =>
            {
                linux_binary_path = Some(path_in_build.clone())
            }
            _ => {}
        }

        let mut created_file = fs::File::create_new(path_in_build)?;

        created_file.set_permissions(fs::Permissions::from_mode(entry_mode))?;
        created_file.write_all(&bytes)?;
    }

    Ok((windows_binary_path, linux_binary_path))
}

fn build_bot_tomls(
    bot_configs: &[PathBuf],
    proj_name: &str,
    proj_src_root_dir: &Path,
    proj_build_root_dir: &Path,
    windows_binary_path: Option<PathBuf>,
    linux_binary_path: Option<PathBuf>,
) -> anyhow::Result<()> {
    for bot_toml_path in bot_configs {
        if !bot_toml_path.exists() {
            return Err(anyhow!("bot_toml_path {:?} doesn't exist", bot_toml_path));
        }
        if !bot_toml_path.canonicalize()?.starts_with(proj_src_root_dir) {
            return Err(anyhow!(
                "Bot config path {:?} is outside of project root {:?}",
                bot_toml_path,
                proj_src_root_dir
            ));
        }
        let str_contents =
            fs::read_to_string(bot_toml_path).context(format!("reading {:?}", bot_toml_path))?;
        let mut toml_bot_config: toml::Table = toml::from_str(&str_contents)?;

        let settings_table: &mut toml::Table = toml_bot_config
            .get_mut("settings")
            .context(format!(
                "Couldn't read settings field in bot toml at {:?}",
                bot_toml_path
            ))?
            .as_table_mut()
            .ok_or(anyhow!(
                "Couldn't read settings table in bot toml at {:?} (field isn't a table)",
                bot_toml_path
            ))?;

        if let Some(windows_binary_path) = &windows_binary_path {
            settings_table.insert(
                "run_command".to_owned(),
                toml::Value::String(
                    windows_binary_path
                        .canonicalize()?
                        .strip_prefix(proj_build_root_dir.canonicalize()?)?
                        .as_os_str()
                        .to_str()
                        .unwrap()
                        .to_owned(),
                ),
            );
        } else {
            return Err(anyhow!("No windows binary found for this project"));
        }

        if let Some(linux_binary_path) = &linux_binary_path {
            settings_table.insert(
                "run_command_linux".to_owned(),
                toml::Value::String(
                    linux_binary_path
                        .canonicalize()?
                        .strip_prefix(proj_build_root_dir.canonicalize()?)?
                        .as_os_str()
                        .to_str()
                        .unwrap()
                        .to_owned(),
                ),
            );
        } else {
            settings_table.remove("run_command_linux");
        }

        let bot_toml_parent = bot_toml_path.parent().unwrap();

        let local_logo_path = match settings_table.get("logo_file") {
            Some(toml::Value::String(str)) => str,
            _ => "logo.png",
        };
        let logo_path = bot_toml_parent.join(local_logo_path);
        if logo_path.exists() {
            fs::copy(&logo_path, proj_build_root_dir.join(local_logo_path))
                .context(format!("copying logo file {:?}", logo_path))?;
        }

        if let Some(toml::Value::String(local_loadout_path)) = settings_table.get("loadout_file") {
            let loadout_path = bot_toml_parent.join(local_loadout_path);
            if loadout_path.exists() {
                fs::copy(&loadout_path, proj_build_root_dir.join(local_loadout_path))
                    .context(format!("copying loadout file {:?}", loadout_path))?;
            }
        }

        let bot_toml_out_path = proj_build_root_dir.join(
            bot_toml_path
                .file_name()
                .context("Couldn't get filename from bot_toml_path")?,
        );
        fs::File::create(&bot_toml_out_path)
            .context(format!("creating file {:?}", bot_toml_out_path))?
            .write_all(toml_bot_config.to_string().as_bytes())?;
    }

    Ok(())
}
