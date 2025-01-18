use std::{fs, path::PathBuf};

use anyhow::anyhow;
use clap::{Parser, Subcommand};

mod build;
mod buildinfo;
mod ci;
mod config;
mod diff;

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

    /// Build incrementally and produce platform-specific tarballs and diffs
    CI { dir: PathBuf },

    /// Diffing tool for directories, based on qbsdiff
    Diff { dir: PathBuf },
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
        Command::Build(x) => build::command(x),
        Command::CI { dir } => ci::command(dir),
        Command::Diff { dir } => diff::command(dir),
    }
}
