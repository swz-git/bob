use std::{
    env,
    ffi::OsStr,
    fs,
    io::{BufReader, Cursor, Read, Write},
    path::PathBuf,
};

use anyhow::{anyhow, Context};
use buildinfo::{BuildInfo, Project};
use chrono::Local;
use clap::{Parser, Subcommand};
use config::read_build_configs;
use toml::value::Datetime;

mod build;
mod buildinfo;
mod config;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

/// Doc comment
#[derive(Subcommand)]
#[command()]
enum Command {
    /// Build based on a bob.toml
    Build(BuildCommand),

    /// Package a bob output dir
    Pack { dir: PathBuf },
}

#[derive(Parser, Debug)]
struct BuildCommand {
    config_path: PathBuf,
    #[arg(short, long, default_value = "./bob_build")]
    out_dir: PathBuf,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Build(x) => build::build(x),
        Command::Pack { dir } => pack(dir),
    }
}

fn pack(dir: PathBuf) -> anyhow::Result<()> {
    if !fs::exists(&dir)? {
        return Err(anyhow!("Directory doesn't exist"));
    }

    Ok(())
}
