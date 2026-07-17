# Development Guide

## Architecture boundary

EmuContainer has three layers:

```text
package recipe and component lock
              |
      shared validation and planning
              |
       platform backend adapters
          /               \
 Linux GameImage       Windows MSIX
```

The package recipe declares desired behavior. It must not contain target-specific command scripts. Backend adapters translate the same runner, content, state, presentation, and permission declarations into native artifacts.

GameImage and FlatImage own the Linux sandbox runtime. EmuContainer owns the recipe translation and shared behavior contract. Windows uses native Windows binaries and packaging rather than running the Linux artifact through WSL or a virtual machine.

## Toolchain

The shared CLI is written in Rust. The repository pins the compiler version in `rust-toolchain.toml`; rustup installs it automatically when a Cargo command is run in the project.

Required now:

- Git.
- rustup.
- A supported Windows or Linux host.

Required when the adapters are implemented:

- Linux: a native Linux environment capable of executing FlatImage and the GameImage build workflow.
- Windows: the Windows SDK and MSIX packaging/signing tools.
- Two local verification environments: native Windows and native Linux. WSL2 may host Linux build work, but native Linux remains the acceptance environment for graphics, audio, controller, and session behavior.

## Commands

```text
.\scripts\bootstrap.ps1
.\scripts\check.ps1
./scripts/bootstrap.sh
./scripts/check.sh

cargo run -- validate examples/contentless/emucontainer.toml
cargo fmt --all --check
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo build --locked
```

Use `cargo fmt --all` to apply Rust formatting.

## Current package contract

The initial manifest includes:

- `schema_version`: contract version, currently `1`.
- `id`: stable lowercase dotted game identifier.
- `title` and `system`: player-facing title and emulated system.
- `targets`: currently `linux-x86_64` and `windows-x86_64`.
- `runner`: initially a RetroArch frontend and libretro core.
- `content`: contentless, bundled, or user-supplied.
- `permissions`: display, audio, GPU, input, and network intent.
- `presentation`: fullscreen intent.

Bundled content requires a forward-slash-separated relative path and a SHA-256 hash. The validator does not yet acquire or hash the referenced file; that belongs to the component-lock and build-planning milestone.

The manifest is deliberately smaller than the design sketch in the research documents. Add fields only with a concrete adapter or shared-behavior use case, validation rules, an agent-operable manual scenario, and migration consideration.

## Planned modules

As functionality arrives, keep these concerns separate:

- `manifest`: parsing and semantic validation.
- `lock`: exact component sources, versions, licenses, and hashes.
- `plan`: target-neutral resolution of a recipe into required components and behavior.
- `cache`: verified content-addressed acquisition.
- `backend::linux_gameimage`: GameImage recipe generation and artifact assembly.
- `backend::windows_msix`: native payload layout, manifest generation, and development signing.
- `verification`: reusable launch, shutdown, state, permission, and diagnostic evidence collection for the manual behavior plan.

Do not add empty modules solely to reserve names; introduce them with the first working behavior and its manual validation path.

## Content and secrets

Never commit personal ROM dumps, firmware, BIOS files, signing certificates, private keys, or store credentials.

Redistributable example content must include provenance, its exact license or permission grant, required notices, and a checksum. Development signing material should be generated and stored locally outside Git.

## Local verification

Development checks and artifact builds run locally. Automated regression suites, test harnesses, coverage systems, continuous-integration services, and GitHub Actions are deferred until EmuContainer has a working product whose behavior needs regression protection.

`scripts/check.ps1` is the Windows entry point and `scripts/check.sh` is the Linux entry point. They run formatting checks, Clippy with warnings denied, and a locked local build. Functional behavior is exercised through [the agent-operable manual test plan](manual-test-plan.md). Platform-adapter verification will add explicit local commands and machine-readable evidence as the first artifacts are implemented.
