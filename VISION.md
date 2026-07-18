# EmuContainer Vision

## The product

EmuContainer turns a portable emulator, one game, and their launch configuration into one movable game image. A player opens that image and goes directly into the game without separately installing or configuring the emulator.

The v1 experience is deliberately concrete:

> Build one `.emuc` file, move that file to another Windows x64 machine with the EmuContainer runtime installed, and launch the same preconfigured game while keeping player state outside the image.

The `.emuc` file is the complete per-game transport artifact. It contains the emulator folder, ROM, configuration, metadata, integrity hashes, provenance, and notices. The installed runtime verifies and materializes the image, creates a writable game instance, connects persistent state, launches the emulator directly, and supervises its process tree.

## Why this project exists

Emulation commonly asks players to install an emulator, organize game files, choose plugins, configure paths and controllers, and reproduce those choices on every machine. Frontends improve discovery but usually still depend on a separately maintained emulator installation and ROM library.

EmuContainer moves that setup into an inspectable per-game image. The player receives one file and the host receives one shared runtime. The source emulator folder and ROM are build inputs only; neither is needed after the image is built.

This resembles Docker's image/runtime split without claiming Docker's security boundary:

- A `.emuc` image is immutable, content-addressed, inspectable, and movable.
- The runtime maintains disposable verified caches and writable instances.
- Saves and emulator-specific mutable data live in stable per-game state or named volumes.
- Explicit lifecycle commands install the runtime, report status, prune disposable data, and uninstall it.

## v1 priorities

EmuContainer v1 prioritizes:

- One distributable `.emuc` file per game.
- Windows 10/11 x64 and portable Windows x64 emulator folders.
- Deterministic builds from declared inputs.
- Complete image verification before executing packaged code.
- Direct fullscreen launch with ordered arguments and no intermediary shell.
- Generic emulator support through declarative entrypoints, arguments, environment, and persistent mounts.
- Battery saves, save states, and mutable emulator data that survive cache pruning and image replacement.
- Clean process-tree supervision and actionable machine-readable logs.
- User-scoped installation and `.emuc` file association without administrator rights.
- Agent-operable build and runtime diagnostics.

## Trust and isolation boundary

A `.emuc` file contains native executable code and must be trusted like an unsigned executable. Its inventory and hashes prove internal consistency and detect accidental or naive corruption; without an externally trusted digest or signature, they do not prove who built it or rule out deliberate replacement of both payload and metadata.

v1 is not a sandbox. The emulator runs with the invoking user's normal access to files, devices, the registry, and the network. `isolation = "none"` and `network_enforced = false` are recorded in every image so inspection cannot imply otherwise. Per-game directories prevent ordinary configuration collisions, but they are an organization and lifecycle boundary rather than a security boundary.

## Content and distribution boundary

EmuContainer does not grant rights to distribute games, firmware, BIOS files, emulators, or artwork. Builders are responsible for every input's license. Personal ROMs and resulting images remain ignored local fixtures and must not enter version control or be published without explicit redistribution permission.

Notices and provenance travel inside the image. No host source path, private build path, or undeclared file may be embedded in its metadata.

## First meaningful success

The v1 proof of concept succeeds when the same generic runtime can:

- Build several mGBA images and at least one melonDS image from private ignored fixtures.
- Produce byte-identical images from identical inputs.
- Inspect and reject a corrupted image before launching a process.
- Cold-launch a moved image after its source inputs and prior cache are unavailable.
- Launch directly into each game with working graphics, audio, fullscreen, and controller input.
- Preserve independent battery saves and save states across cache pruning and compatible emulator-version updates.
- Refuse state reuse when a game ID is paired with a different ROM or emulator identity.
- Terminate and account for the complete emulator process tree.
- Install, associate, prune, uninstall, and reinstall the runtime without deleting player state.

## Deferred direction

Linux, macOS, ARM, OCI compatibility, production signing, stronger isolation, network enforcement, shared layers, delta updates, multi-file disc images, BIOS management, automatic emulator acquisition, and a polished GUI launcher are later decisions. The historical GameImage, FlatImage, libretro, and MSIX research remains useful context, but it does not define the v1 architecture.
