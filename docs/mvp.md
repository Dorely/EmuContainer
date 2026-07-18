# Windows Runtime and Single-File Image v1

## Goal

Build one deterministic `.emuc` file from a schema-v2 recipe, a clean portable Windows x64 emulator folder, and one ROM. On Windows 10/11 x64, a shared user-scoped EmuContainer runtime must open that image and launch directly into the game without access to the original build inputs.

The v1 proof uses four private, ignored ROM fixtures:

- Pokémon Blue, Crystal, and Emerald through mGBA 0.10.5.
- Pokémon Renegade Platinum through melonDS 1.1.

The matrix demonstrates multiple content formats and sizes plus two unrelated standalone emulators. It does not authorize redistribution of those ROMs or generated images.

## Architecture acceptance

The implementation has two deliverables:

1. A single per-game `.emuc` transport artifact containing the emulator folder, ROM, launch configuration, inventory, hashes, provenance, and notices.
2. One installed runtime that verifies and materializes images, maintains disposable writable instances, attaches stable state, supervises the process tree, and manages user-scoped file association.

Arbitrary Windows executables and DLLs require filesystem paths. The runtime may therefore materialize a verified image under `%LOCALAPPDATA%`; "single file" describes distribution, not in-memory execution.

## Acceptance gates

Record each gate through [the manual test plan](manual-test-plan.md).

### Build and image contract

- Both checked-in schema-v2 recipes validate and the removed schema-v1 contentless recipe is no longer accepted or documented as current behavior.
- Recipe mode and the equivalent all-flags mode build valid images.
- Identical tool and input bytes produce byte-identical `.emuc` files.
- The image contains exactly the declared emulator, ROM, notices, metadata, and sorted inventory without host source paths.
- The builder rejects a wrong ROM hash, unsafe Windows path, reparse point, case collision, ROM nested inside the emulator tree, and overwrite without `--force`.
- Inspection verifies the complete archive and rejects a corrupted or undeclared entry before any packaged process starts.

### Runtime behavior

- A moved image cold-launches after its source emulator/ROM tree is unavailable and prior cache and instance data have been pruned.
- Each image starts directly into its game using declarative arguments; no emulator-specific Rust adapter or shell command is involved.
- mGBA state tokens and the melonDS junction-backed `portable` volume persist independently across launches and pruning.
- A compatible emulator-version update for the same game, system, ROM hash, and emulator ID reuses state.
- A reused game ID with a different ROM hash, system, or emulator ID is rejected before state is mounted.
- Only one launch per game ID is allowed at a time.
- Structured logs identify verification, paths, mounts, expanded launch data, PID, root exit code, and process-tree completion.
- The runtime's Job Object closes the complete emulator process tree when the runtime terminates unexpectedly.

### Player experience

- All four images launch fullscreen directly into the expected game.
- Graphics and synchronized audio work on native Windows.
- An XInput controller works at minimum with Pokémon Emerald.
- Battery saves and save states can be created, hashed, pruned around, and loaded again.
- Exiting the emulator returns cleanly to the invoking terminal or shell.

Graphics, audio, fullscreen, physical controller use, and recognizing the loaded save are explicitly human-observed. Process state, logs, hashes, state changes, cache layout, junctions, and exit codes remain agent-verifiable.

### Installation and lifecycle

- `runtime install` installs without elevation and registers `.emuc` under HKCU.
- The same image launches from the CLI and by file association.
- `runtime status` verifies the installed binary and association.
- `runtime prune` removes only inactive caches and instances.
- `runtime uninstall` removes the owned runtime and association but preserves game identity, state, volumes, and logs.
- Reinstalling the runtime can launch the prior image and load its saved state.

## Milestone sequence

1. Stage and hash private ignored ROMs plus clean official emulator folders.
2. Replace the schema-v1 libretro contract with the strict schema-v2 portable-emulator recipe.
3. Deliver deterministic image build, complete inspection, and safe path policy.
4. Deliver verified materialization, writable instances, identity guards, stable state, logging, locking, and process supervision.
5. Deliver user-scoped install, status, prune, uninstall, and `.emuc` association.
6. Build and launch the four-image emulator-neutral matrix.
7. Run locked local checks and record the applicable manual evidence.

## Go/no-go decision

v1 is successful when one generic runtime launches all four images and each image remains self-contained after being moved away from its unavailable inputs. If supporting melonDS requires emulator-specific runtime code rather than declarative configuration and a generic persistent mount, revise the runtime contract before adding more emulators.

## Explicitly deferred

- Automated test suites, test harnesses, coverage, and continuous integration.
- Linux, macOS, ARM64, Android, web, and OCI runtimes.
- Sandboxing or enforced network denial.
- Production code signing, a polished installer, and a GUI-subsystem launcher.
- Automatic emulator or ROM acquisition.
- Shared layers, delta updates, and runtime deduplication.
- BIOS management and multi-file/CUE-BIN disc content.
- Save migration between different emulator IDs.
- A new game-library frontend, cloud saves, achievements, netplay, and automatic updates.
