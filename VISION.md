# EmuContainer Vision

## The idea

EmuContainer will explore whether emulators and their games can be packaged as self-contained, easily launched units.

The goal is larger than simply running an emulator inside a container. It is to turn an emulated game into a dependable, portable, console-like experience.

The experience we are working toward is simple:

> A player selects a game from a controller-first interface. The game opens fullscreen with the correct emulator, settings, controls, audio, and video. Saves persist automatically. When the player exits, they return cleanly to the interface. The experience behaves the same on every supported machine.

## Why this project exists

Today, emulation often requires users to install an emulator, organize game files, select cores or plugins, configure controllers, tune graphics settings, and troubleshoot differences between machines. Existing gaming-focused Linux distributions improve this experience, but games generally remain dependent on a larger, carefully configured system.

EmuContainer aims to move more of that configuration into a portable package. A package would describe or contain everything needed to launch a particular game: the emulator, its version, required configuration, controller mappings, display behavior, metadata, and any permitted supporting files. Game content could be supplied separately when licensing prevents it from being distributed.

The player should not need to understand the package's internals. From Steam Big Picture, a controller-first launcher, a desktop shortcut, or a command line, launching the package should feel like launching an ordinary game.

## Project priorities

EmuContainer will prioritize:

- A controller-first, console-like experience.
- Repeatable behavior across supported computers.
- Simple launching and clean shutdown.
- Working graphics, audio, controllers, and fullscreen display.
- Persistent save data kept separate from the disposable runtime.
- Isolation between games so one game's configuration cannot break another.
- Clear handling of user-supplied ROMs, BIOS files, and other legally restricted assets.
- Packages that can be inspected, versioned, backed up, and moved.
- Compatibility with existing frontends rather than requiring a new frontend immediately.

## Initial boundaries

Initially, EmuContainer does not need to:

- Replace established emulators or gaming frontends.
- Distribute copyrighted games, firmware, or BIOS files.
- Support every emulator, operating system, and graphics platform.
- Guarantee that each package is literally a single native executable.
- Hide all host dependencies at the expense of performance or reliability.

## First meaningful success

The first meaningful success will be a Linux-based proof of concept in which one legally distributable or user-supplied game:

- Can be launched from a controller-friendly interface.
- Uses hardware-accelerated graphics and a gamepad.
- Stores saves outside its runtime.
- Shuts down cleanly and reliably returns the player to the interface.
- Behaves consistently when moved between supported machines.

## Product direction

The initial unit of packaging will be one playable game per launchable unit, even if several units share the same emulator layers internally. This most closely matches the experience of selecting and launching a game like a native executable.

"Container" is a possible mechanism, not the product promise. GPU access, display systems, audio, controllers, low-latency input, and platform-specific drivers all cross container boundaries. The eventual package may use an OCI container with a host launcher, an AppImage-style bundle, Flatpak, or a hybrid approach.

EmuContainer commits to the portable, repeatable player experience while leaving room to discover the right technical machinery.
