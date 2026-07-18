# EmuContainer

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

## v1 commands

Validate either checked-in schema-v2 example:

```powershell
cargo run --locked -- validate examples/mgba-pokemon-blue/emucontainer.toml
cargo run --locked -- validate examples/melonds-pokemon-renegade-platinum/emucontainer.toml
```

Build and inspect an image with private local inputs:

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

Manage the user-scoped runtime:

```powershell
& $Cli runtime install
& $Cli runtime status --json
& $Cli runtime prune --json
& $Cli runtime uninstall
```

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
