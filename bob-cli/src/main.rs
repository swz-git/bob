use std::path::PathBuf;

use bob_lib::dirhasher;
use clap::{Parser, Subcommand};

mod build;
mod buildinfo;
mod config;
mod diff;
mod split;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

/// Bob is a build tool for the RLBot v5 botpack
#[derive(Subcommand)]
#[command(version, about, long_about = None)]
enum Command {
    /// Build based on a bob.toml
    Build(BuildCommand),

    /// Split bob build directory into platform-specific directories
    Split { dir: PathBuf },

    /// Diffing tool for directories, based on qbsdiff. Outputs a diff to stdout
    Diff { old: PathBuf, new: PathBuf },
    /// Diffing tool for directories, based on qbsdiff. Reads a diff from stdin and applies it
    DiffApply { dir: PathBuf },

    /// Generate a hash for a directory, the same function is used internally for incremental
    /// builds.
    Hash { dir: PathBuf },
}

#[derive(Parser, Debug)]
struct BuildCommand {
    config_path: PathBuf,
    #[arg(short, long, default_value = "./bob_build")]
    /// By default, bob will reuse already-built projects if the project hash matches
    out_dir: PathBuf,
}

fn command_hash(dir: PathBuf) -> anyhow::Result<()> {
    let hash = dirhasher(dir)?;
    println!("{hash:016x}");
    Ok(())
}

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let cli = Cli::parse();
    match cli.command {
        Command::Build(x) => build::command_build(x),
        Command::Split { dir } => split::command_split(dir),
        Command::Diff { old, new } => diff::command_diff(old, new),
        Command::DiffApply { dir } => diff::command_diff_apply(dir),
        Command::Hash { dir } => command_hash(dir),
    }
}
