# Recommended Technical Direction

> **Historical research:** This document predates and is superseded by the Runtime+Image v1 architecture in [`VISION.md`](../../VISION.md) and [`docs/mvp.md`](../mvp.md). Its recommendations are retained only as decision history.

Research snapshot: 2026-07-17

## Decision

Build EmuContainer as a backend-neutral game package contract and cross-platform artifact builder.

Do not implement another Linux container runtime. Use [GameImage](https://github.com/gameimage/gameimage) and FlatImage as the first Linux x86-64 backend, and propose runtime improvements upstream. Retain Flatpak as a Linux behavior reference and as a practical initial Linux ARM64 path.

The first architectural proof should generate two materially different native outputs from one recipe:

1. A Linux x86-64 GameImage.
2. A Windows x64 MSIX, with an optional portable archive for comparison.

If that shared contract holds, add a signed universal macOS application bundle and an Android ARM64 APK. Use OCI containers for repeatable component builds where useful, not as a required player runtime or as a claim that one Linux artifact runs on every operating system.

The detailed evidence and platform mapping are in the [GameImage cross-platform gap analysis](cross-platform-gap-analysis.md).

## Initial target platforms

The Linux behavior benchmark remains intentionally narrow:

- Operating system: modern Linux.
- Architecture: x86-64 first.
- Primary environments: a normal Wayland desktop and a SteamOS/Steam Deck-style gaming environment.
- Graphics: Intel and AMD Mesa drivers first; proprietary NVIDIA validation second.
- Steam installation for the first test: native host Steam, avoiding a nested Flatpak-on-Flatpak launch until the basic path works.
- WSL or a conventional VM may be used to request or host builds, but not as the Linux acceptance environment for controllers, latency, audio, or Gamescope behavior.

The cross-platform contract benchmark adds native Windows x64 immediately after the GameImage package. Windows is not expected to run the FlatImage; it receives a native RetroArch/core payload in an MSIX generated from the same manifest. macOS universal and Android ARM64 outputs follow once Linux and Windows demonstrate that the manifest is genuinely portable.

## Proposed layers

```text
Steam / ES-DE / desktop / command line
                  |
          stable launch command
                  |
       EmuContainer manifest + launcher
          /                     \
 runtime adapter            persistent state
 Flatpak or FlatImage       saves/config/logs
          |
 pinned emulator + core + permitted content
          |
     host display/audio/GPU/controllers
```

The contract between these layers is more important than the first file format.

## Package contents and state

Treat these as separate concerns:

### Immutable package

- Manifest and schema version.
- Game ID, title, platform, and artwork metadata.
- Emulator and core identifiers with pinned versions or hashes.
- Default emulator configuration and controller mapping overrides.
- Redistributable game content, when permitted.
- Required user-supplied content and BIOS declarations.
- License notices and source-offer information.
- Declared runtime permissions.

### Mutable player state

- Battery-backed saves.
- Save states.
- Per-game user configuration.
- Screenshots and recordings.
- Logs and crash reports.

Mutable state should live in an explicit XDG location such as:

```text
$XDG_DATA_HOME/emucontainer/games/<game-id>/
$XDG_CONFIG_HOME/emucontainer/games/<game-id>/
$XDG_STATE_HOME/emucontainer/games/<game-id>/
$XDG_CACHE_HOME/emucontainer/games/<game-id>/
```

Do not make the only copy of a player's saves part of a writable package image. Keeping state external allows an artifact to be replaced, verified, rolled back, or shared without risking saves.

## Illustrative manifest

This is a design sketch, not yet a committed schema:

```toml
schema_version = 1
id = "example.open-game.gb"
title = "Example Open Game"
system = "game-boy"

[runner]
kind = "libretro"
frontend = "retroarch"
frontend_version = "pinned-version-or-hash"
core = "sameboy"
core_version = "pinned-version-or-hash"

[content]
mode = "user-supplied"
filename = "game.gb"
sha256 = "expected-hash-if-known"

[permissions]
display = true
audio = true
gpu = true
input = true
network = false

[presentation]
fullscreen = true
gamescope = "auto"

[state]
save_directory = "saves"
save_state_directory = "states"
```

Why TOML for the sketch: it is readable, has less ambiguous scalar behavior than YAML, and is well supported from Rust and other likely implementation languages. A JSON Schema or equivalent machine-readable validation contract should accompany the final format.

## First emulator path

Use RetroArch with one pinned libretro core.

Recommended sequence:

1. Use the contentless `Anarch` or `2048` core as a plumbing smoke test. The libretro license list identifies these as CC0 and public domain respectively. This avoids ROM and BIOS questions while display, controller, process, and package behavior are being debugged.
2. Use the SameBoy core with a separately verified, redistributable homebrew Game Boy ROM or a user-supplied ROM. SameBoy is listed as MIT licensed and Game Boy does not require a BIOS for this test.
3. Add a hardware-heavier emulator or core after the first path works to validate Vulkan/OpenGL performance and more demanding save/config behavior.
4. Add one standalone emulator to prove that the manifest can describe more than libretro.

Do not choose the first test ROM merely because it is easy to download. Verify the license for the exact binary and assets, retain the license text, and record its checksum.

## Flatpak proof of concept

The first real package should:

- Build RetroArch and one core at pinned revisions.
- Use a currently supported `org.freedesktop.Platform` runtime.
- Request Wayland with fallback X11.
- Request DRI graphics, audio playback, and input devices.
- Require Flatpak 1.15.6 or newer for the narrower `input` device permission, or document a temporary older-version fallback.
- Disable network access.
- Store saves outside the immutable application payload.
- Export a desktop file and icon.
- Launch directly into the game without showing the RetroArch menu first.
- Exit fully when the emulator closes and return the caller's exit status.

This package is the reference behavior. Later backends should be tested against the same acceptance criteria.

## FlatImage and GameImage proof of concept

Package the same pinned emulator, core, configuration, and content with GameImage or directly with FlatImage.

Validate:

- Direct double-click and command-line launch.
- `gpu`, `input`, `udev`, `audio`, and Wayland/X11 permissions.
- No access to the rest of the home directory.
- External save-state binding.
- Operation on at least two substantially different Linux distributions.
- AMD/Intel GPU behavior, then NVIDIA.
- Steam shortcut and ES-DE launch behavior.
- Cold-start time, final file size, and update workflow.

The comparison should distinguish GameImage workflow limitations from FlatImage runtime limitations.

## Launcher implementation

Start with the smallest possible wrapper needed to test process and path behavior. Once the contract stabilizes, Rust is a strong candidate for the installed launcher because it can produce a single native binary, has good process and signal handling, and has mature TOML/JSON serialization libraries.

The launcher should eventually own:

- Manifest validation.
- Resolution of user-supplied ROM and BIOS assets.
- State-directory creation and migration.
- Backend selection.
- Environment construction.
- Optional Gamescope wrapping.
- Signal forwarding and child-process cleanup.
- Structured diagnostics.
- Desktop and frontend integration generation.

It should not own an emulator UI or a game-library UI.

## Frontend sequence

1. Run from a terminal to expose diagnostics.
2. Export and test a desktop entry.
3. Add the desktop entry or executable manually to native Steam and test controller-first launch and exit.
4. Generate a Steam ROM Manager-compatible entry and artwork metadata.
5. Add an ES-DE custom system definition that invokes the same command.

The launch behavior must remain identical across all five entry points.

## Feasibility acceptance criteria

The first phase is successful when one package:

- Launches directly into a playable game.
- Uses hardware acceleration rather than software rendering.
- Produces synchronized audio without an obvious latency regression.
- Recognizes a controller at launch and after reconnect, where supported.
- Runs fullscreen and does not expose setup UI during normal launch.
- Saves progress and retains it after the runtime artifact is replaced.
- Exits cleanly to the calling frontend with no orphan emulator process.
- Runs on two supported Linux distributions without rebuilding.
- Requires neither root privileges nor a background Docker daemon for play.
- Has network access disabled.
- Produces enough diagnostics to explain a missing device, asset, or permission.

Record cold-start time, package size, steady-state CPU/GPU behavior, and input/audio observations. These measurements will make the Flatpak-versus-FlatImage decision evidence-based.

## Main risks

### GPU driver compatibility

Userspace graphics libraries and the host kernel driver must cooperate. Flatpak has the strongest existing driver-extension system. FlatImage and general containers need explicit validation, especially on proprietary NVIDIA drivers.

### Controller permissions and hotplug

Reading `/dev/input` is only part of controller support; udev visibility, device permissions, Steam Input, and reconnection behavior also matter. Test physical controllers rather than relying only on synthetic input.

### Nested sandboxes

Flatpak Steam, Flatpak frontends, and container-inside-container launches can prevent namespace creation or host access. Treat each nesting combination as a separate support target.

### Licensing

RetroArch, cores, emulator assets, firmware, and games can all use different licenses. Some popular cores are non-commercial. The manifest and build pipeline need a component-level license inventory from the start.

### Duplication versus portability

A completely self-contained file is easy to move but repeats emulator and runtime data for every game. Shared runtimes save space but introduce installation and version-management concepts. Supporting a Flatpak-style installed backend and a FlatImage-style portable backend lets the project test both ends of this tradeoff.

### Configuration and save compatibility

Pinned software improves reproducibility, but users will eventually want updates. Emulator updates can change configuration syntax and save-state compatibility. Battery-backed/native saves should be preferred over save states for long-term continuity, and migrations must be explicit.

## Proposed next research-to-build step

The next concrete task should be a hands-on cross-platform benchmark, not a new runtime implementation:

1. Select a legally clean smoke-test core and content path.
2. Create the first `emucontainer.toml`, component lock file, reference RetroArch configuration, and logical save-directory layout.
3. Package it for Linux x86-64 with GameImage/FlatImage.
4. Package it for native Windows x64 as an MSIX from the same recipe.
5. Run the same acceptance checklist against both artifacts on their native operating systems.
6. Keep the existing Flatpak experiment as the Linux reference and ARM64 follow-up rather than the proof of cross-platform differentiation.

Only after the Linux and Windows comparison should EmuContainer commit to the manifest and add macOS and Android adapters. FlatImage runtime gaps should normally become upstream issues or contributions; EmuContainer's own implementation should focus on orchestration, platform mapping, and conformance.
