# EmuContainer Research

Research snapshot: 2026-07-17

This folder records the initial research into whether EmuContainer is feasible, what similar projects already exist, and which technologies are the best candidates for a first implementation.

## Short answer

The project is feasible, but the cross-platform requirement changes where it should innovate.

[GameImage](https://github.com/gameimage/gameimage) is almost exactly the EmuContainer vision for Linux x86-64: an emulator, game, and configuration packaged into a single launchable [FlatImage](https://flatimage.github.io/docs/). Its current build and release paths do not produce native Windows, macOS, Android, or Linux ARM64 applications.

EmuContainer therefore should not build another Linux container runtime. It should define one declarative game recipe and compile it through native backends:

1. GameImage/FlatImage on Linux x86-64, with Flatpak retained as a reference backend and initial Linux ARM64 option.
2. MSIX on Windows.
3. A signed and notarized application bundle on macOS.
4. A signed APK on Android.
5. An optional WebAssembly/PWA backend for the subset of libretro cores that work well in browsers.

Use RetroArch and one pinned libretro core for the first runner path. Keep immutable packages separate from saves, map abstract permissions onto each operating system, and perform build and signing steps in local native Windows and Linux environments. If the scope ever narrows to Linux x86-64, the right answer is to contribute to GameImage rather than continue a separate packaging implementation.

## Research documents

- [Prior art](prior-art.md) covers projects that already solve all or part of this problem.
- [Technology assessment](technology-assessment.md) compares the candidate runtime and packaging technologies.
- [GameImage cross-platform gap analysis](cross-platform-gap-analysis.md) determines what GameImage covers and where a multi-platform EmuContainer would add something new.
- [Recommended direction](recommended-direction.md) proposes a staged architecture and proof-of-concept plan.
- [Sources](sources.md) lists the primary documentation and repositories used in this research.

## Main conclusion

There is no fundamental technical blocker to running an emulator in an isolated, portable Linux userspace. Valve's Steam Linux Runtime, RetroDECK, FlatImage, Conty, and Games on Whales all demonstrate important parts of the solution.

The hard part is dependable host integration: GPU drivers, display sockets, audio, controllers, hotplug events, persistent saves, and clean process shutdown. Mature desktop-oriented sandbox tools already address more of those boundaries than a conventional Docker image does. EmuContainer should build around that fact.

The project's opportunity is not merely to prove that an emulator can run in a container. That has already been proven. The opportunity is to define a clean, open, per-game package contract and make the same recipe behave consistently across native operating-system packages and controller-first frontends.
