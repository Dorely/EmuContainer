use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use emucontainer::Manifest;

#[derive(Debug, Parser)]
#[command(
    name = "emucontainer",
    version,
    about = "Build native application packages for emulated games"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Parse and validate an EmuContainer package manifest.
    Validate {
        /// Path to emucontainer.toml.
        #[arg(value_name = "MANIFEST")]
        manifest: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Validate { manifest } => validate_manifest(&manifest),
    }
}

fn validate_manifest(path: &std::path::Path) -> Result<()> {
    let manifest =
        Manifest::load(path).with_context(|| format!("could not validate {}", path.display()))?;
    let targets = manifest
        .targets
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ");

    println!(
        "valid: {} ({}, targets: {})",
        manifest.id, manifest.title, targets
    );

    Ok(())
}
