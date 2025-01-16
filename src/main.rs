use std::{fs, path::PathBuf};

use anyhow::anyhow;
use clap::{Parser, Subcommand};

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
    /// By default, bob will reuse already-built projects if the project hash matches
    out_dir: PathBuf,
}

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
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

    // this is probably gonna be a built in .tar.xz archiver or something similar
    todo!("packing/unpacking");

    Ok(())
}
