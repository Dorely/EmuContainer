# EmuContainer Historical Research

Research snapshot: 2026-07-17

This folder preserves the feasibility work that preceded the v1 implementation decision. The detailed documents evaluate GameImage, FlatImage, Flatpak, OCI, MSIX, libretro, and other cross-platform packaging approaches. They remain useful background and source material, but they are not the current product contract.

## Current decision supersedes this snapshot

EmuContainer v1 is Windows-first and uses a purpose-built Runtime+Image architecture:

- One deterministic `.emuc` file contains a portable Windows x64 emulator folder, one ROM, launch configuration, inventory, hashes, provenance, and notices.
- One user-scoped Windows runtime verifies and materializes that image, maintains a writable instance and persistent state, launches the emulator directly, and supervises its process tree.
- Generic standalone emulators are configured declaratively; v1 is not limited to RetroArch or libretro.
- The image/runtime split provides Docker-like lifecycle and state semantics but no sandbox or enforced network denial.
- Linux GameImage/FlatImage and Windows MSIX are deferred rather than v1 backends.

For normative behavior, use [the project vision](../../VISION.md), [v1 milestone](../mvp.md), [development guide](../development.md), and [manual test plan](../manual-test-plan.md).

## Historical documents

- [Prior art](prior-art.md) surveys projects that solve all or part of the original problem.
- [Technology assessment](technology-assessment.md) compares candidate Linux runtimes and package formats.
- [GameImage cross-platform gap analysis](cross-platform-gap-analysis.md) maps the platform coverage that motivated the earlier native-backend proposal.
- [Recommended direction](recommended-direction.md) records the superseded backend-neutral recipe and native-package recommendation.
- [Sources](sources.md) catalogs the primary documentation used by the research.

## Continuing value

The research correctly identifies host integration—graphics, audio, controllers, persistent state, and process shutdown—as the difficult part of dependable emulation packaging. It also remains relevant if future work adds Linux, macOS, Android, web, stronger isolation, or native store packaging. Any such expansion requires a new decision and acceptance plan rather than silently reviving the historical architecture.
