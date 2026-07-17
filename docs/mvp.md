# Windows and Linux MVP

## Goal

Build one legally redistributable retro game from one EmuContainer recipe into:

- A Linux x86-64 GameImage/FlatImage.
- A native Windows x64 MSIX.

Each artifact will include the appropriate native RetroArch frontend, one pinned libretro core, the game configuration, and the game ROM when its license permits redistribution. The two artifacts are not expected to be byte-identical; they are expected to implement the same declared behavior.

## Acceptance criteria

Record these behaviors through the applicable scenarios in [the agent-operable manual test plan](manual-test-plan.md).

Both packages must:

- Launch directly into the game from a normal application shortcut.
- Start fullscreen without presenting emulator setup UI.
- Use native hardware-accelerated video and synchronized audio.
- Accept a controller at launch.
- Keep battery saves and other mutable state outside the immutable package payload.
- Retain saves when the application package is replaced.
- Exit fully and return control to the calling launcher.
- Run with network access disabled.
- Report a useful error when a required runtime capability or content file is unavailable.
- Identify the exact frontend, core, content, and build-tool revisions used.

## Milestones

- [x] Initialize local Git, the toolchain, manifest model, agent-operable manual test plan, and Windows/Linux development scripts.
- [ ] Select a small redistributable ROM and retain its license text and SHA-256 checksum.
- [ ] Pin a RetroArch revision and one core revision for both targets.
- [ ] Define the component lock file and acquisition cache.
- [ ] Assemble and launch the unpackaged reference payload on Windows and Linux.
- [ ] Generate the Linux GameImage through a noninteractive adapter.
- [ ] Generate and sign a development Windows MSIX.
- [ ] Run the shared manual behavior checklist on both native operating systems.
- [ ] Record package size, cold-start time, controller behavior, audio observations, and save migration behavior.

## First content sequence

Use the contentless `2048` libretro core only as an early plumbing test. It does not satisfy the ROM-based MVP.

For the real MVP, select a small homebrew ROM whose exact binary and accompanying assets are explicitly redistributable. Prefer a simple cartridge platform that needs no BIOS and a core available on both Windows and Linux. Record the source URL, license, attribution requirements, and SHA-256 hash before adding the binary.

## Explicitly deferred

- Automated regression suites, test harnesses, coverage systems, and continuous integration until a working product exists.
- macOS, Android, web, and ARM64 artifacts.
- A new game-library frontend.
- Standalone emulators outside libretro.
- Online play, achievements, cloud saves, and automatic updates.
- Production signing and store publication.
- Shared runtime deduplication across multiple installed games.

## Go/no-go check

The cross-platform design is validated when one manifest can produce both native packages without including Windows- or Linux-specific shell commands in the game recipe. If the recipe devolves into unrelated target scripts, revise the package contract before adding more platforms.
