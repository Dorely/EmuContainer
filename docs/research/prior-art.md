# Prior Art

> **Historical research:** This document predates and is superseded by the Runtime+Image v1 architecture in [`VISION.md`](../../VISION.md) and [`docs/mvp.md`](../mvp.md). Its recommendations are retained only as decision history.

Research snapshot: 2026-07-17

## Closest match: GameImage

[GameImage](https://github.com/gameimage/gameimage) is the closest known match to the EmuContainer vision.

GameImage describes itself as a tool that packages a runner, such as an emulator or Wine, together with a game and its configuration in one FlatImage. Its documented package types include RetroArch, PCSX2, RPCS3, Wine, and native Linux software. It supports directly booting a selected game rather than always showing its launcher.

Its current output is specifically Linux x86-64 rather than a native Windows, macOS, Android, or Linux ARM64 package. That boundary and the resulting project decision are documented in the [GameImage cross-platform gap analysis](cross-platform-gap-analysis.md).

The overlap with EmuContainer is substantial:

- One game and its runner can be placed in a portable package.
- The result can be launched directly.
- Emulator configuration can travel with the game.
- Saves can be colocated with the package.
- The runtime is isolated from the host userspace.

This does not make EmuContainer unnecessary, but it changes the first question. We should first determine whether GameImage already satisfies the vision, can be extended to satisfy it, or reveals gaps that justify a new implementation.

Questions for a hands-on GameImage evaluation:

- Does a current package work on SteamOS, a mainstream desktop distribution, and a second distribution without modification?
- Do Wayland, X11, audio, gamepad hotplug, and hardware acceleration all work without privileged setup?
- Can saves be placed in an explicit external XDG state directory rather than modifying the game artifact?
- Can a package launch and exit cleanly from Steam and ES-DE?
- Can a user supply a ROM or BIOS file without rebuilding the distributable package?
- Are runtime and emulator versions reproducible and auditable?
- How are updates, signing, license notices, and shared emulator data handled?
- Does the current GameImage release integrate cleanly with the current FlatImage release?
- Can its broad default generated permissions be narrowed to the manifest's declared capabilities, with network and general home-directory access disabled by default?

## FlatImage

[FlatImage](https://flatimage.github.io/docs/) is the container and package technology beneath current GameImage.

It combines a compressed, single-file executable with an unprivileged sandbox. Its current documentation describes granular permissions for GPU, input, udev, audio, Wayland, X11, external media, and network access. It uses Bubblewrap for isolation and supports compressed layers and writable overlays.

This is a strong match for the eventual EmuContainer experience because it is designed to run directly without requiring a container daemon or prior installation. It is also a smaller ecosystem than Flatpak, so it needs broader compatibility and maintenance testing before becoming the only supported backend.

## Conty

[Conty](https://github.com/Kron4ek/Conty) is a compressed, unprivileged Linux container packaged as one executable. It explicitly supports OpenGL, Vulkan, X11, Wayland, Steam Deck, Bubblewrap sandboxing, and gaming applications including RetroArch and several standalone emulators.

Conty proves that a gaming-capable Linux userspace can be shipped as a portable executable. Its default model is a broad Arch Linux gaming environment rather than a narrowly specified per-game package, and its default container can see the user's home directory unless sandboxing is enabled. It is therefore more useful as a reference implementation or experimental backend than as EmuContainer's product model.

## RetroDECK

[RetroDECK](https://github.com/RetroDECK/RetroDECK) is a self-contained Flatpak that combines emulators, engines, tools, and a controller-oriented frontend. It is designed for Steam Deck, desktop Linux, HTPCs, and Linux console systems.

RetroDECK demonstrates that Flatpak can support a polished emulation environment with controllers, graphics, audio, configuration management, and SteamOS-oriented usage. Its unit of packaging is an entire emulation environment, not one game, so it solves a neighboring problem rather than the same one.

Useful lessons for EmuContainer include:

- Flatpak is viable for real emulation workloads.
- Emulator versions and configuration need to be managed as a tested set.
- Per-game overrides are still necessary inside a standardized environment.
- Nested sandbox situations, such as a Flatpak frontend launching another Flatpak, require special care.

## EmuDeck

[EmuDeck](https://manual.emudeck.com/) installs and configures emulators and related tools. It is primarily configuration automation rather than application isolation or per-game packaging.

EmuDeck is evidence that setup and configuration are major parts of the user problem. It is also a likely future integration environment, but it is not a container solution.

## Batocera and Lakka

[Batocera](https://batocera.org/) and [Lakka](https://www.lakka.tv/) package emulation as complete operating systems. Lakka uses RetroArch and libretro cores to centralize display, audio, and input behavior.

These systems validate the console-first experience but achieve consistency by controlling the entire operating system. EmuContainer is trying to achieve a useful portion of that consistency while running inside an existing Linux desktop or gaming distribution.

## Valve Steam Linux Runtime and UMU

[Valve's Steam Linux Runtime](https://github.com/ValveSoftware/steam-runtime) uses Linux namespaces to run games in a predictable userspace on many distributions. Modern native Steam games and Proton releases use this container runtime.

[UMU Launcher](https://github.com/Open-Wine-Components/umu-launcher) makes Valve's pressure-vessel-based runtime usable for Windows games outside Steam. These projects are strong evidence that containerized userland is compatible with high-performance gaming when host graphics and session integration are handled deliberately.

Pressure-vessel is not recommended as EmuContainer's first implementation. It is specialized, relatively complex, and UMU is aimed at Proton rather than native emulators. It should be studied as an architecture reference, especially for graphics-driver and host-runtime integration.

## Games on Whales / Wolf

[Wolf](https://github.com/games-on-whales/wolf) launches graphical applications in Docker or Podman containers and streams them to Moonlight clients with audio and gamepad support.

Wolf proves that conventional containers can host games and emulators, including hotplugged gamepads. Its [quick-start configuration](https://games-on-whales.github.io/wolf/stable/user/quickstart.html) also illustrates the cost: access to `/dev/dri`, `/dev/input`, `/dev/uinput`, `/dev/uhid`, udev data, host networking, and the container engine socket can all be involved.

This makes Wolf a useful source for a future remote or multi-user EmuContainer backend. Its streaming architecture is not the simplest route to a local, double-clickable game.

## LinuxServer RetroArch

[LinuxServer's RetroArch image](https://docs.linuxserver.io/images/docker-retroarch/) runs RetroArch in a container and exposes its interface through a browser-accessible desktop. It is useful evidence that the emulator can be containerized, but it is optimized for remote GUI access rather than local, low-latency, controller-first launching.

## Steam ROM Manager and ES-DE

[Steam ROM Manager](https://github.com/SteamGridDB/steam-rom-manager) bulk-imports emulator games and other executables into Steam and manages their artwork and controller templates. It already solves much of the Steam library integration problem.

[ES-DE](https://gitlab.com/es-de/emulationstation-de/-/blob/master/USERGUIDE.md) supports custom system definitions and launch commands. An EmuContainer bundle only needs a stable command-line entry point to participate in its library.

These projects suggest a clear boundary: EmuContainer should produce a good launch target and metadata, then let mature frontends own browsing, artwork, and controller-first navigation.

## Debian game-data-packager

[Debian's game-data-packager](https://wiki.debian.org/Games/GameDataPackager) creates local packages from game data that cannot be redistributed, verifies that data using checksums, and places it where a free engine expects it.

This is a useful model for legally restricted ROM and BIOS files. An EmuContainer manifest should be able to declare required user-supplied files and hashes without distributing those files.

## What appears to remain open

The individual technical pieces already exist. A useful EmuContainer contribution could combine them around a clearer contract:

- One game is the unit presented to the player.
- The package has a backend-neutral manifest.
- Emulator, core, configuration, content, and firmware requirements are explicit and versioned.
- Redistributable and user-supplied assets are cleanly separated.
- Saves remain stable across package and emulator updates.
- Permissions are declared narrowly, with networking disabled by default.
- The same package can be exposed to Steam, ES-DE, desktop menus, or a command line.
- More than one runtime backend can implement the contract.
