# Technology Assessment

> **Historical research:** This document predates and is superseded by the Runtime+Image v1 architecture in [`VISION.md`](../../VISION.md) and [`docs/mvp.md`](../mvp.md). Its recommendations are retained only as decision history.

Research snapshot: 2026-07-17

This document compares Linux runtime and packaging technologies. It does not claim that the winning Linux artifact runs natively on other operating systems. The separate [GameImage cross-platform gap analysis](cross-platform-gap-analysis.md) maps the Windows, macOS, Android, and web backends.

## Evaluation criteria

The technologies were evaluated as Linux backends against the experience described in the project vision:

- Launchable like a game, preferably without a visible container workflow.
- Hardware-accelerated graphics with low overhead.
- Reliable Wayland and X11 display behavior.
- Reliable audio and controller access, including hotplug where possible.
- Immutable, reproducible runtime and configuration.
- Persistent saves that survive runtime replacement.
- Clean exit back to the calling frontend.
- Cross-distribution Linux support.
- No root requirement for ordinary play.
- Reasonable update, deduplication, inspection, and signing story.
- Compatibility with Steam and controller-first Linux frontends.

## Summary matrix

The ratings below are directional rather than benchmark results. A hands-on spike is still required.

| Technology | Player-facing launch | Gaming host integration | Reproducibility | Shared storage and updates | Maturity | Recommended role |
| --- | --- | --- | --- | --- | --- | --- |
| Flatpak | Good after installation | Strong | Strong | Strong | High | Linux behavior reference and initial ARM64 option |
| FlatImage / GameImage | Excellent single-file goal | Promising | Strong in principle | Weak when every game embeds a runtime | Emerging | Primary Linux portable output to evaluate |
| OCI with rootless Podman | Requires a launcher and runtime | Manual and host-specific | Excellent | Excellent | High | Build system and later advanced backend |
| Docker | Poor for a local consumer executable | Manual and often privileged | Excellent | Excellent | High | Local reproducible builds or server research, not first player runtime |
| AppImage | Excellent single-file launch | Native host access, no sandbox policy | Moderate | Duplicate per file | High | Simple fallback packaging, not isolation foundation |
| Conty | Excellent single-file launch | Strong but broadly exposed by default | Good | Duplicate per file | Medium | Reference and experimental backend |
| Steam pressure-vessel | Excellent inside its intended ecosystem | Excellent | Excellent | Strong | High | Architecture reference, not initial public API |

## Flatpak

Flatpak is the strongest technology for a controlled Linux behavior reference. GameImage/FlatImage remains the closer match to the desired portable product, while Flatpak provides a mature comparison point for permissions, graphics, controllers, persistence, and ARM64 builds.

[Flatpak runtimes](https://docs.flatpak.org/en/latest/basic-concepts.html) provide a distribution-independent userspace and share unchanged data across installed applications and updates. Its sandbox and desktop export model are already integrated into Linux desktops.

The current [sandbox permission model](https://docs.flatpak.org/en/latest/sandbox-permissions.html) directly covers the important EmuContainer boundaries:

- `--device=dri` for direct rendering.
- `--device=input` for `/dev/input`, including game controllers, on Flatpak 1.15.6 or newer.
- `--socket=pulseaudio` for playback and related audio devices.
- `--socket=wayland` plus `--socket=fallback-x11` for modern and legacy display sessions.
- Per-application XDG config, data, and cache directories for persistent state.

Flatpak also manages graphics-driver extensions, including active host GL driver selection, through the Freedesktop runtime extension system. This is a major advantage over hand-assembling host graphics libraries in a general OCI container.

Limitations:

- A Flatpak bundle is installed before use; it is not itself the final double-click executable.
- A single-file `.flatpak` bundle does not include its runtime dependencies, so fully offline delivery requires additional packaging or a configured runtime source.
- Giving each game a unique Flatpak application ID may become operationally heavy.
- A Flatpak frontend launching another sandboxed application can introduce nested-sandbox problems.

These limitations do not weaken Flatpak as a feasibility baseline. It is valuable precisely because it lets the project prove the difficult host boundaries before inventing a portable packaging layer.

## FlatImage and GameImage

[FlatImage](https://flatimage.github.io/docs/) is the closest technology to the eventual artifact described in the vision. It packages a userspace, application, dependencies, configuration, and sandbox machinery into a single executable.

Its declared permissions map well to EmuContainer: `gpu`, `input`, `udev`, `audio`, `wayland`, and `xorg`. It can embed compressed layers and set a default boot command. Current documentation says it is sandboxed with no access by default.

[GameImage](https://github.com/gameimage/gameimage) adds a game-focused creation workflow on top of FlatImage and already supports several emulator families.

Limitations and questions:

- A separate self-contained runtime per game duplicates emulator and userspace data.
- A smaller project has less distribution coverage than Flatpak and needs compatibility testing.
- Proprietary NVIDIA drivers remain a special host-integration case.
- Writable layers inside the executable blur the boundary between an immutable release artifact and irreplaceable player saves.
- Update, signing, rollback, and garbage-collection behavior need evaluation.
- GameImage and FlatImage version compatibility should be tested rather than assumed.

Recommendation: treat FlatImage as the leading portable-backend candidate and GameImage as both prior art and a potential implementation dependency. Do not fork or replace either until a hands-on gap analysis is complete.

## OCI containers, Podman, and Docker

OCI images provide excellent reproducibility, content-addressed layers, registries, signatures, and deduplication. They are attractive for the internal build pipeline and for sharing emulator base layers.

They are less attractive as the first player-facing runtime:

- The player needs a container engine and image store.
- The launcher must pass through display sockets, audio sockets, GPU devices, input devices, udev information, and persistent volumes.
- Rootless device access still depends on host permissions and groups.
- NVIDIA commonly requires additional host-specific integration.
- The resulting command line and failure modes look like infrastructure rather than an ordinary game.

[Games on Whales](https://games-on-whales.github.io/wolf/stable/user/quickstart.html) demonstrates that these problems can be solved, but its required device and socket access shows why a purpose-built desktop sandbox is the better first local target.

Recommendation: use Podman or another OCI implementation to make builds reproducible. Revisit an OCI runtime adapter for streaming, multi-user servers, development, and shared-layer installations after the local experience works.

## AppImage

[AppImage](https://docs.appimage.org/introduction/concepts.html) offers the simplest one-file application model. It is widely understood and does not require installation.

AppImage deliberately does not provide shared frameworks between application files. It also depends on selected host resources, including graphics libraries, and does not define a sandbox or game-state policy. EmuContainer would need to build those features around it.

Recommendation: retain AppImage as a possible launcher distribution format or non-isolated fallback. Do not use it as the primary isolation design.

## Bubblewrap

[Bubblewrap](https://github.com/containers/bubblewrap) is the low-level unprivileged sandbox used by Flatpak and FlatImage. It constructs Linux namespaces and a controlled filesystem view.

Bubblewrap is a mechanism, not a complete security policy. Its maintainers explicitly leave the security model to the higher-level tool that constructs its arguments.

Recommendation: consume Bubblewrap through Flatpak or FlatImage during early work. A custom Bubblewrap runner would make EmuContainer responsible for years of subtle display, D-Bus, filesystem, device, and security policy decisions.

## RetroArch and libretro

RetroArch is the recommended first emulator frontend. The [libretro API](https://docs.libretro.com/development/cores/developing-cores/) separates emulator cores from platform-specific audio, video, and input behavior. RetroArch also has a mature [controller autoconfiguration system](https://docs.libretro.com/guides/controller-autoconfiguration/).

Benefits for the first proof:

- One command-line shape can cover many systems.
- Input, display, and audio configuration are centralized.
- A pinned core is a small, auditable unit.
- Save and system directories can be explicitly redirected.
- The first package can avoid firmware-heavy, high-complexity emulators.

Limitations:

- RetroArch is GPLv3 and individual cores use different licenses. Distribution obligations must be tracked per component.
- Some systems work better in standalone emulators and will eventually need a second runner type.
- Core and emulator version changes can alter compatibility and save-state behavior.

Recommendation: begin with RetroArch, then deliberately add one standalone emulator later to prove that the manifest is not tied to libretro.

## Gamescope

[Gamescope](https://github.com/ValveSoftware/gamescope) is a gaming-focused Wayland micro-compositor. It can give a game a controlled virtual resolution, fullscreen or borderless output, scaling, and isolation from the rest of the desktop.

It is useful for making desktop launches feel more console-like, but it should be optional:

- SteamOS Game Mode already runs inside Gamescope.
- Nested Gamescope sessions can introduce different behavior and additional failure modes.
- A game package should still be able to run without it.

Recommendation: expose Gamescope as a host-side launch policy for desktop sessions, not as a hard requirement embedded in every package.

## Frontend integration

Use existing frontend contracts:

- Export a standard `.desktop` entry and icon.
- Make the launch command stable and noninteractive.
- Support manual addition to Steam first.
- Generate metadata that [Steam ROM Manager](https://github.com/SteamGridDB/steam-rom-manager) can import.
- Add an [ES-DE custom system](https://gitlab.com/es-de/emulationstation-de/-/blob/master/USERGUIDE.md) adapter after the command-line contract is stable.

This keeps the core project independent from any single frontend and avoids recreating mature library, artwork, and controller-navigation software.
