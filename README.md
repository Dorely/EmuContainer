# EmuContainer

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Buy me a coffee](https://img.shields.io/badge/Buy%20me%20a%20coffee-dorely-FFDD00)](https://buymeacoffee.com/dorely)

EmuContainer packages a portable Windows emulator, one ROM, and their launch configuration into one distributable `.emuc` game image. A shared user-scoped runtime verifies the image, materializes a writable instance, attaches persistent state, and launches directly into the game.

```text
recipe + emulator folder + ROM
              |
      emucontainer build
              v
         one GAME.emuc
              |
   installed EmuContainer runtime
              v
          the game
```

The source emulator folder and ROM are not needed after a successful build. Runtime caches and instances may contain extracted files under `%LOCALAPPDATA%`, but the only per-game file that must be transported is the `.emuc` image.

## Security and content warning

A `.emuc` image contains native executable code. Treat an unsigned image exactly as you would an unsigned `.exe`: its internal hashes detect accidental corruption, but only a separately trusted digest or signature could prove publisher identity or deliberate replacement.

v1 is not a sandbox and does not disable networking. Packaged emulators run with the invoking user's normal permissions. Per-game state directories provide lifecycle and configuration isolation, not a security boundary.

Do not commit or publish personal ROMs, BIOS files, firmware, emulator archives, extracted emulator folders, generated `.emuc` files, or saves. Local fixtures belong under the already ignored `.emucontainer/` directory.

## Status and requirements

EmuContainer is an early v0.1.0 proof of concept. Build it from source using the steps below; the v1 goals and acceptance criteria are in [docs/mvp.md](docs/mvp.md). Graphics, audio, fullscreen, and controller compatibility require the documented manual checks for each emulator configuration.

- **Runtime:** Windows 10/11 x64 with NTFS-backed `%LOCALAPPDATA%`.
- **Development:** Git, [rustup](https://rustup.rs/), and the Visual Studio Build Tools C++ workload with a Windows SDK for the Windows MSVC toolchain. Rust, rustfmt, and Clippy versions are pinned by the repository.
- **Game inputs:** your own portable Windows x64 emulator and ROM, with their required notices. No emulator, game, BIOS, or firmware is included.
- **Other platforms:** recipe validation, image building, and inspection are platform-neutral; launching and runtime installation require Windows.

## Quick start from source

In PowerShell on Windows:

```powershell
git clone https://github.com/Dorely/EmuContainer.git
Set-Location EmuContainer
.\scripts\bootstrap.ps1
.\scripts\check.ps1
$Cli = '.\target\debug\emucontainer.exe'
& $Cli --help
```

The check script produces the debug executable used below. For an optimized executable, run `cargo build --release --locked` and use `target\release\emucontainer.exe`.

### Validate recipes

Validate either checked-in schema-v2 example:

```powershell
cargo run --locked -- validate examples/mgba-pokemon-blue/emucontainer.toml
cargo run --locked -- validate examples/melonds-pokemon-renegade-platinum/emucontainer.toml
```

### Build and inspect an image

First stage your private inputs using [the manual plan's fixture instructions](docs/manual-test-plan.md#mt-100-stage-private-roms-without-tracking-them). The checked-in examples describe specific ROM checksums and emulator versions; validation alone does not download or check those input files. For a different game, copy a recipe into `.emucontainer/`, update its identity, ROM SHA-256, launch configuration, and notices, and use that recipe when building. See the [recipe contract](docs/development.md#recipe-schema-v2).

With the example's matching private inputs in place:

```powershell
$Cli = '.\target\debug\emucontainer.exe'
New-Item -ItemType Directory -Force -Path .emucontainer\artifacts | Out-Null
& $Cli build `
  --recipe examples/mgba-pokemon-blue/emucontainer.toml `
  --emulator-root .emucontainer/test-inputs/emulators/mgba-0.10.5/mGBA-0.10.5-win64 `
  --rom .emucontainer/test-inputs/roms/gb/pokemon-blue.gb `
  --output .emucontainer/artifacts/pokemon-blue.emuc
& $Cli inspect .emucontainer/artifacts/pokemon-blue.emuc --json
# Interactive: this opens the emulator window. Do not run it unattended.
& $Cli run .emucontainer/artifacts/pokemon-blue.emuc
```

### Install the runtime

Install once per Windows user to register double-click opening of `.emuc` files. On another machine, build or copy a trusted Windows runtime executable, run its `runtime install` command there, then open the transported image. Administrator rights are not required.

```powershell
& $Cli runtime install
& $Cli runtime status --json
```

Maintenance commands:

```powershell
& $Cli runtime prune --json
& $Cli runtime uninstall
```

Pruning removes inactive caches and instances. Uninstalling removes the owned runtime and file association. Both retain player state and logs.

| Data | Location |
| --- | --- |
| Installed runtime | `%LOCALAPPDATA%\Programs\EmuContainer` |
| Disposable verified images | `%LOCALAPPDATA%\EmuContainer\cache\images` |
| Disposable writable instances | `%LOCALAPPDATA%\EmuContainer\containers` |
| Persistent saves, volumes, and game logs | `%LOCALAPPDATA%\EmuContainer\games\<game-id>` |

See the [manual test plan](docs/manual-test-plan.md) for fixture intake, the four-image validation matrix, integrity rejection, launch, persistence, isolation, and lifecycle scenarios.

## Development

Install [rustup](https://rustup.rs/), then run the local checks:

```powershell
.\scripts\bootstrap.ps1
.\scripts\check.ps1
```

```sh
./scripts/bootstrap.sh
./scripts/check.sh
```

The check scripts verify formatting, run Clippy with warnings denied, and perform a locked build. Automated test suites and CI are intentionally deferred until the working product needs regression protection.

The v1 runtime is Windows 10/11 x64 only. Recipe validation, image building, and image inspection are platform-neutral; runtime launch and installation report unsupported on other platforms.

## Repository map

```text
src/                     CLI, recipe/image logic, and Windows runtime
examples/                metadata-only schema-v2 recipes
docs/mvp.md              v1 outcome and acceptance gates
docs/development.md      contracts, architecture, and local workflow
docs/manual-test-plan.md exact agent-operable validation scenarios
docs/research/           historical feasibility research
scripts/                 local bootstrap and quality checks
```

Every agent session starts by reading [VISION.md](VISION.md) and the canonical [FILEMAP.md](FILEMAP.md). Contributions follow [CONTRIBUTING.md](CONTRIBUTING.md).

## Feedback and contributions

[Open an issue](https://github.com/Dorely/EmuContainer/issues) for bugs or feature requests. Include your Windows version, EmuContainer version, emulator version, reproduction steps, and relevant sanitized diagnostics. Do not attach ROMs, generated images, saves, credentials, or logs containing private paths. See [CONTRIBUTING.md](CONTRIBUTING.md) for scope and local verification requirements.

## License

EmuContainer is licensed under the [MIT License](LICENSE). Emulator, game, firmware, and artwork inputs remain subject to their own licenses and distribution permissions.

## Support the project

If EmuContainer is useful to you, you can [buy Dorely a coffee](https://buymeacoffee.com/dorely) or use GitHub's **Sponsor** button. Donations are optional and do not change the MIT license or create a paid support commitment.
