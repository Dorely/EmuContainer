use std::{
    collections::BTreeMap,
    io::{self, Write},
    path::PathBuf,
    process,
};

use anyhow::{Context, Result, bail};
use clap::{Args, Parser, Subcommand};
use emucontainer::{
    Recipe, Target,
    image::{BuildRequest, build_image, verify_image},
    recipe::{Content, Emulator, Notice, SUPPORTED_SCHEMA_VERSION, SeedPolicy, State, StateMount},
    runtime::{
        RunOptions, install_runtime, prune_runtime, run_image, runtime_status, uninstall_runtime,
    },
};
use serde::Serialize;

#[derive(Debug, Parser)]
#[command(
    name = "emucontainer",
    version,
    about = "Build and run self-contained emulator and ROM images"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Parse and validate an EmuContainer recipe.
    Validate {
        /// Path to an EmuContainer schema-v2 recipe.
        #[arg(value_name = "RECIPE")]
        recipe: PathBuf,
    },
    /// Build one portable emulator folder and one ROM into a .emuc image.
    Build(Box<BuildArgs>),
    /// Verify every entry in an image and print its metadata.
    Inspect {
        /// Path to the .emuc image.
        #[arg(value_name = "IMAGE")]
        image: PathBuf,
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// Verify, materialize, and launch an image through the Windows runtime.
    Run {
        /// Path to the .emuc image.
        #[arg(value_name = "IMAGE")]
        image: PathBuf,
        /// Echo lifecycle events as JSON lines.
        #[arg(long)]
        json: bool,
    },
    /// Install, inspect, prune, or uninstall the user-scoped runtime.
    Runtime {
        #[command(subcommand)]
        command: RuntimeCommand,
    },
}

#[derive(Debug, Args)]
struct BuildArgs {
    /// Load the image contract from a schema-v2 TOML recipe.
    #[arg(long, value_name = "RECIPE")]
    recipe: Option<PathBuf>,

    /// Stable dotted game identifier for all-flags mode.
    #[arg(long)]
    id: Option<String>,
    /// Player-facing game title for all-flags mode.
    #[arg(long)]
    title: Option<String>,
    /// Emulated system slug for all-flags mode.
    #[arg(long)]
    system: Option<String>,
    /// Image target; v1 accepts only windows-x86_64.
    #[arg(long)]
    target: Option<String>,
    /// Stable emulator identifier for all-flags mode.
    #[arg(long)]
    emulator_id: Option<String>,
    /// Emulator version for all-flags mode.
    #[arg(long)]
    emulator_version: Option<String>,
    /// Emulator provenance URL for all-flags mode.
    #[arg(long)]
    emulator_source: Option<String>,
    /// Emulator-root-relative executable path for all-flags mode.
    #[arg(long)]
    entrypoint: Option<String>,
    /// Emulator-root-relative working directory for all-flags mode.
    #[arg(long)]
    working_directory: Option<String>,
    /// Ordered emulator argument; repeat to add more.
    #[arg(long = "arg", value_name = "VALUE", allow_hyphen_values = true)]
    arguments: Vec<String>,
    /// Environment override in KEY=VALUE form; repeat to add more.
    #[arg(long = "env", value_name = "KEY=VALUE")]
    environment: Vec<String>,
    /// Packaged ROM file name for all-flags mode.
    #[arg(long)]
    rom_name: Option<String>,
    /// Required ROM SHA-256 for all-flags mode.
    #[arg(long)]
    rom_sha256: Option<String>,
    /// Persistent mount in NAME=RELATIVE_TARGET form.
    #[arg(long = "state-mount", value_name = "NAME=TARGET")]
    state_mounts: Vec<String>,
    /// Notice mapping in EMULATOR_RELATIVE_SOURCE=OUTPUT_NAME form.
    #[arg(long = "notice", value_name = "SOURCE=OUTPUT")]
    notices: Vec<String>,

    /// Complete portable emulator directory to package.
    #[arg(long, value_name = "DIR")]
    emulator_root: PathBuf,
    /// ROM file to package.
    #[arg(long, value_name = "FILE")]
    rom: PathBuf,
    /// Destination .emuc file.
    #[arg(long, value_name = "GAME.emuc")]
    output: PathBuf,
    /// Atomically replace an existing output image.
    #[arg(long)]
    force: bool,
}

#[derive(Debug, Subcommand)]
enum RuntimeCommand {
    /// Copy this runtime user-locally and register the .emuc association.
    Install,
    /// Verify the installed runtime binary and file association.
    Status {
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// Remove inactive caches and instances while preserving player data.
    Prune {
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// Remove the owned runtime and association while preserving player data.
    Uninstall,
}

fn main() -> Result<()> {
    if let Some(exit_code) = execute(Cli::parse())? {
        process::exit(exit_code);
    }
    Ok(())
}

fn execute(cli: Cli) -> Result<Option<i32>> {
    match cli.command {
        Command::Validate { recipe } => {
            let recipe = Recipe::load(&recipe)
                .with_context(|| format!("could not validate {}", recipe.display()))?;
            println!(
                "valid recipe: {} ({}, target: {})",
                recipe.id, recipe.title, recipe.target
            );
            Ok(None)
        }
        Command::Build(arguments) => {
            let recipe = resolve_build_recipe(&arguments)?;
            let result = build_image(&BuildRequest {
                recipe: &recipe,
                emulator_root: &arguments.emulator_root,
                rom: &arguments.rom,
                output: &arguments.output,
                force: arguments.force,
            })
            .with_context(|| format!("could not build {}", arguments.output.display()))?;
            println!(
                "built: {} ({} bytes, sha256 {})",
                result.output.display(),
                result.image_size,
                result.image_sha256
            );
            Ok(None)
        }
        Command::Inspect { image, json } => {
            let verified = verify_image(&image)
                .with_context(|| format!("could not inspect {}", image.display()))?;
            if json {
                write_json(&serde_json::json!({
                    "image": image,
                    "image_sha256": verified.image_sha256,
                    "image_size": verified.image_size,
                    "manifest": verified.manifest,
                }))?;
            } else {
                println!(
                    "verified: {} ({} bytes, sha256 {})",
                    image.display(),
                    verified.image_size,
                    verified.image_sha256
                );
                println!(
                    "game: {} ({}, system: {})",
                    verified.manifest.game.id,
                    verified.manifest.game.title,
                    verified.manifest.game.system
                );
                println!(
                    "emulator: {} {} ({})",
                    verified.manifest.emulator.id,
                    verified.manifest.emulator.version,
                    verified.manifest.emulator.entrypoint
                );
                println!(
                    "content: {} ({} bytes, sha256 {})",
                    verified.manifest.content.filename,
                    verified.manifest.content.size,
                    verified.manifest.content.sha256
                );
                println!("inventory: {} entries", verified.manifest.inventory.len());
                println!("isolation: none; network enforcement: disabled");
            }
            Ok(None)
        }
        Command::Run { image, json } => {
            let report = run_image(&image, RunOptions { json_events: json })
                .with_context(|| format!("could not run {}", image.display()))?;
            if !json {
                println!(
                    "run complete: {} (exit {}, logs: {})",
                    report.game_id,
                    report.exit_code,
                    report.log_directory.display()
                );
            }
            Ok((report.exit_code != 0).then_some(report.exit_code))
        }
        Command::Runtime { command } => execute_runtime(command),
    }
}

fn execute_runtime(command: RuntimeCommand) -> Result<Option<i32>> {
    match command {
        RuntimeCommand::Install => {
            let report = install_runtime()?;
            println!(
                "runtime installed: {} (sha256 {}, association: {})",
                report.executable.display(),
                report.executable_sha256,
                report.association_registered
            );
            Ok(None)
        }
        RuntimeCommand::Status { json } => {
            let report = runtime_status()?;
            if json {
                write_json(&report)?;
            } else {
                println!(
                    "runtime status: installed={}, healthy={}, executable={}",
                    report.installed,
                    report.healthy,
                    report.executable.display()
                );
                for diagnostic in &report.diagnostics {
                    println!("- {diagnostic}");
                }
            }
            Ok((!report.healthy).then_some(1))
        }
        RuntimeCommand::Prune { json } => {
            let report = prune_runtime()?;
            if json {
                write_json(&report)?;
            } else {
                println!(
                    "pruned: {} image caches, {} container instances; preserved {}",
                    report.removed_image_caches,
                    report.removed_container_instances,
                    report.preserved_game_data_root.display()
                );
            }
            Ok(None)
        }
        RuntimeCommand::Uninstall => {
            let report = uninstall_runtime()?;
            if report.deferred_to_worker {
                println!("runtime uninstall worker started; player data will be preserved");
            } else {
                println!(
                    "runtime uninstalled: installation_removed={}, association_removed={}; preserved {}",
                    report.removed_installation,
                    report.removed_association,
                    report.preserved_game_data_root.display()
                );
            }
            Ok(None)
        }
    }
}

fn resolve_build_recipe(arguments: &BuildArgs) -> Result<Recipe> {
    if let Some(path) = &arguments.recipe {
        if has_inline_contract(arguments) {
            bail!("--recipe cannot be combined with inline image-contract flags");
        }
        return Recipe::load(path)
            .with_context(|| format!("could not load recipe {}", path.display()));
    }

    let target = required(arguments.target.clone(), "--target")?;
    let target = match target.as_str() {
        "windows-x86_64" => Target::WindowsX86_64,
        other => bail!("unsupported --target {other:?}; v1 accepts windows-x86_64"),
    };

    let mut environment = BTreeMap::new();
    for value in &arguments.environment {
        let (name, value) = split_mapping(value, "--env")?;
        if environment
            .insert(name.to_owned(), value.to_owned())
            .is_some()
        {
            bail!("--env declares {name:?} more than once");
        }
    }

    let notices = arguments
        .notices
        .iter()
        .map(|value| {
            let (source, output) = split_mapping(value, "--notice")?;
            Ok(Notice {
                source: source.to_owned(),
                output: output.to_owned(),
            })
        })
        .collect::<Result<Vec<_>>>()?;

    let mounts = arguments
        .state_mounts
        .iter()
        .map(|value| {
            let (name, target) = split_mapping(value, "--state-mount")?;
            Ok(StateMount {
                name: name.to_owned(),
                target: target.to_owned(),
                seed: SeedPolicy::IfEmpty,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    let recipe = Recipe {
        schema_version: SUPPORTED_SCHEMA_VERSION,
        id: required(arguments.id.clone(), "--id")?,
        title: required(arguments.title.clone(), "--title")?,
        system: required(arguments.system.clone(), "--system")?,
        target,
        emulator: Emulator {
            id: required(arguments.emulator_id.clone(), "--emulator-id")?,
            version: required(arguments.emulator_version.clone(), "--emulator-version")?,
            source: required(arguments.emulator_source.clone(), "--emulator-source")?,
            entrypoint: required(arguments.entrypoint.clone(), "--entrypoint")?,
            working_directory: arguments
                .working_directory
                .clone()
                .unwrap_or_else(|| ".".to_owned()),
            arguments: arguments.arguments.clone(),
            environment,
        },
        content: Content {
            filename: required(arguments.rom_name.clone(), "--rom-name")?,
            sha256: required(arguments.rom_sha256.clone(), "--rom-sha256")?,
        },
        notices,
        state: State { mounts },
    };
    recipe.validate()?;
    Ok(recipe)
}

fn has_inline_contract(arguments: &BuildArgs) -> bool {
    arguments.id.is_some()
        || arguments.title.is_some()
        || arguments.system.is_some()
        || arguments.target.is_some()
        || arguments.emulator_id.is_some()
        || arguments.emulator_version.is_some()
        || arguments.emulator_source.is_some()
        || arguments.entrypoint.is_some()
        || arguments.working_directory.is_some()
        || !arguments.arguments.is_empty()
        || !arguments.environment.is_empty()
        || arguments.rom_name.is_some()
        || arguments.rom_sha256.is_some()
        || !arguments.state_mounts.is_empty()
        || !arguments.notices.is_empty()
}

fn required<T>(value: Option<T>, flag: &str) -> Result<T> {
    value.with_context(|| format!("{flag} is required when --recipe is not used"))
}

fn split_mapping<'a>(value: &'a str, flag: &str) -> Result<(&'a str, &'a str)> {
    let (left, right) = value
        .split_once('=')
        .with_context(|| format!("{flag} value {value:?} must use LEFT=RIGHT form"))?;
    if left.is_empty() || right.is_empty() {
        bail!("{flag} value {value:?} must have nonempty values on both sides of '='");
    }
    Ok((left, right))
}

fn write_json(value: &impl Serialize) -> Result<()> {
    let stdout = io::stdout();
    let mut output = stdout.lock();
    serde_json::to_writer_pretty(&mut output, value)?;
    writeln!(output)?;
    Ok(())
}
