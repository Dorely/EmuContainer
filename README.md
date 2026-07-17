# EmuContainer

EmuContainer is an experiment in turning an emulator, one game, and their required configuration into a dependable application-like package.

The first product goal is deliberately concrete:

> One legally redistributable retro game should build from one recipe into a native Linux x86-64 package and a native Windows x64 package. Both should launch directly into the game, work with a controller, keep saves outside the immutable payload, and exit cleanly.

GameImage/FlatImage will supply the Linux packaging runtime. Windows will receive a native RetroArch/libretro payload in an MSIX. EmuContainer's job is the shared recipe, build orchestration, state and permission contract, and cross-platform behavior definition.

## Current status

The development foundation is in place:

- A pinned Rust toolchain.
- A cross-platform command-line application.
- A versioned TOML package manifest with structural validation.
- A contentless libretro example used only to test plumbing.
- Local Windows and Linux bootstrap, quality, and build scripts.
- An agent-operable manual test plan for current and planned behavior.

The actual emulator assembly and package backends are the next implementation phase. No copyrighted ROMs, firmware, or BIOS files belong in this repository.

## Quick start

Install [rustup](https://rustup.rs/). On Windows PowerShell, run:

```text
.\scripts\bootstrap.ps1
.\scripts\check.ps1
```

On Linux, run:

```text
./scripts/bootstrap.sh
./scripts/check.sh
```

The local check scripts run the equivalent of:

```text
cargo fmt --all --check
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo build --locked
```

The checked-in `rust-toolchain.toml` selects the project's Rust version automatically. Run the functional scenarios in [the manual test plan](docs/manual-test-plan.md), including manifest validation, when the affected behavior requires them.

## Repository map

```text
src/                    CLI and shared package-contract code
examples/               package-recipe examples with legally clean inputs
docs/mvp.md             first Windows/Linux milestone and acceptance criteria
docs/development.md     developer workflow and architecture boundaries
docs/manual-test-plan.md agent-operable functional verification scenarios
docs/research/          feasibility and technology research
scripts/                local Windows and Linux setup and verification
```

Agents begin every session with the [project vision](VISION.md) and canonical [file map](FILEMAP.md). Then read the [MVP](docs/mvp.md), [development guide](docs/development.md), and [cross-platform research](docs/research/cross-platform-gap-analysis.md) as relevant.

## Project rules

- A game recipe must remain platform-neutral. Platform-specific work belongs in a backend adapter.
- Network access defaults to disabled.
- Saves and other player state remain separate from immutable application files.
- Emulator, core, tool, and content versions will be pinned by hashes before producing distributable artifacts.
- ROMs, BIOS files, firmware, artwork, and other game data may be committed only when their exact redistribution terms have been verified and recorded.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). The project license has not yet been selected; do not assume that repository access grants redistribution rights beyond those provided by applicable law.
