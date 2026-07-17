# GameImage Cross-Platform Gap Analysis

Research snapshot: 2026-07-17

## Conclusion

GameImage is very close to the EmuContainer vision on one target: Linux on x86-64. It is not currently a cross-platform game format.

That gives this project a clear decision boundary:

- If the intended product is a portable emulator-and-game executable for Linux x86-64, EmuContainer should use or contribute to GameImage instead of creating another runtime.
- If the intended product is one game recipe that can become a native package for Linux, Windows, macOS, and Android, there is still a meaningful project. The missing piece is not another Linux container implementation; it is a common game-package contract and a build system with platform-native backends.

The recommended direction is therefore to treat GameImage as EmuContainer's first Linux backend, not as a competitor to replace or a codebase to fork immediately.

## Three meanings of cross-platform

These goals are easy to conflate, but they lead to different designs.

| Goal | Feasible? | GameImage today |
| --- | --- | --- |
| One identical artifact runs on every operating system | Not realistically for the full native emulator range without a VM; possible for a smaller WebAssembly-compatible subset | No |
| One source recipe produces a native artifact for each operating system | Yes | No |
| A developer can build from Windows, macOS, or Linux | Yes for locally supported targets when the required native SDK or local VM/WSL environment is installed | Indirectly; the current GameImage build itself needs a Linux environment |

EmuContainer should promise the second and third goals. It should not promise that the same binary file will run on all four operating systems.

In plain language: the portable unit is the recipe and its declared game data. The executable output is native to its destination.

## Where GameImage currently lands

GameImage is built on FlatImage. FlatImage describes itself as a sandboxed, single-file executable that runs on any **Linux distribution** and embeds configuration in an ELF binary. Its examples download `alpine-x86_64.flatimage`, and its implementation depends on Linux facilities such as Bubblewrap, namespaces, FUSE, OverlayFS, and Linux process and filesystem behavior. See the [FlatImage user documentation](https://flatimage.github.io/docs/) and [Bubblewrap repository](https://github.com/containers/bubblewrap).

The current release pipelines make the narrower limits explicit:

- The [GameImage release workflow](https://github.com/gameimage/gameimage/blob/master/.github/workflows/on-release.yml) is named `linux-x86_64`, runs on Ubuntu, downloads Linux AMD64 tools, and emits one `.flatimage` artifact.
- The [GameImage deployment script](https://github.com/gameimage/gameimage/blob/master/deploy/flatimage-alpine.sh) is Bash-based, invokes Docker for its component builds, downloads several hard-coded Linux x86-64 binaries, and starts from `alpine-x86_64.flatimage`.
- The [FlatImage release workflow](https://github.com/flatimage/flatimage/blob/master/.github/workflows/github-release.yml) currently has only `x86_64` in its architecture matrix.
- At this research snapshot, [GameImage's latest release](https://github.com/gameimage/gameimage/releases/latest) contains one FlatImage plus its checksum, while [FlatImage's latest release](https://github.com/flatimage/flatimage/releases/latest) contains Alpine, Arch, and Blueprint artifacts only for x86-64.
- [FlatImage ARM64 support](https://github.com/flatimage/flatimage/issues/16) remains an open issue and requires work in several bundled dependencies.
- A [FlatImage WSL2 report](https://github.com/flatimage/flatimage/issues/45) demonstrates that a FlatImage can execute inside the Linux environment supplied by WSL2. That is useful for development and perhaps for building, but the result remains a Linux executable rather than a native Windows application.

Docker in the GameImage build does not make the final artifact portable to Docker's host platforms. Docker helps make component builds repeatable; FlatImage still executes Linux binaries and relies on a Linux kernel at build and play time. A Windows or macOS developer could drive the build through WSL2 or a local Linux virtual machine, but that still produces a Linux x86-64 artifact.

There is also a policy difference to examine during the Linux benchmark. FlatImage defaults to zero permissions, but the current GameImage construction script grants `home`, `media`, `network`, `xorg`, `wayland`, `dbus_user`, and `dev`. EmuContainer's manifest should default to no network and the smallest required host access, then have its Linux adapter request only those capabilities.

Both [GameImage](https://github.com/gameimage/gameimage/blob/master/LICENSE) and [FlatImage](https://github.com/flatimage/flatimage/blob/master/LICENSE) declare the Apache License 2.0, so integration and upstream contribution are practical options subject to the license terms.

## Why the Linux artifact cannot simply be reused everywhere

A FlatImage is not an abstract container image like an OCI filesystem layer. It is a Linux ELF executable whose launcher and sandbox are part of the file. Windows uses PE executables and Windows security primitives; macOS uses Mach-O code inside application bundles and Apple code-signing rules; Android installs signed APKs and runs each application under an Android-assigned identity.

Putting the Linux artifact in a virtual machine can bridge that difference on Windows or macOS, but it introduces a guest operating system, GPU and controller forwarding, extra storage, slower startup, and more failure modes. It also does not provide a practical Android path. That is a useful compatibility escape hatch, not a good product foundation for a console-like experience.

## Recommended platform backends

| Target | Native artifact | Isolation model | Emulator path | Assessment |
| --- | --- | --- | --- | --- |
| Linux x86-64 | GameImage/FlatImage; Flatpak as a reference backend | Bubblewrap/namespaces or Flatpak sandbox | Native RetroArch plus a pinned libretro core | Ready for direct evaluation; do not rebuild this runtime |
| Linux ARM64 | Flatpak first; FlatImage after an upstream ARM64 port | Flatpak sandbox initially | Native ARM64 RetroArch/core | Feasible, but not covered by current FlatImage releases |
| Windows x64, later ARM64 | Signed MSIX; optionally a signed portable directory/launcher | MSIX filesystem and registry virtualization; AppContainer where compatible | Native Win32 RetroArch/core | Feasible new backend |
| macOS Apple silicon and Intel | Signed and notarized universal `.app`, normally distributed in a ZIP or DMG | macOS App Sandbox where compatible; application bundle and per-user data separation otherwise | Native macOS RetroArch/core | Feasible new backend; code signing and JIT need core-by-core validation |
| Android ARM64, optionally x86-64 | Signed APK for installation; AAB for store publishing | Android's built-in per-application sandbox | Android/libretro frontend plus native core libraries | Feasible new backend; requires an Android-specific launcher and lifecycle integration |
| Modern browsers | PWA/site with WebAssembly and packaged data | Browser origin sandbox | Emscripten-built libretro cores | Valuable optional backend for older systems, but not a universal replacement for native emulators |

### Linux

Use GameImage/FlatImage for the portable, directly launchable output. Keep Flatpak as a reference implementation and as the practical ARM64 route while FlatImage ARM64 support is incomplete.

EmuContainer's Linux work should concentrate on recipe generation, external save bindings, narrow permissions, repeatable component pinning, launcher integration, and shared manual behavior checks. Improvements to the FlatImage runtime itself should normally be proposed upstream.

### Windows

[MSIX](https://learn.microsoft.com/en-us/windows/msix/) is the closest native Windows analogue to the desired installed capsule. Windows places package files in a protected location, keeps runtime state separate, and can virtualize filesystem and registry writes. Microsoft is explicit that this is a native Windows process with a virtualized view, not a guest operating system; see the [MSIX containerization overview](https://learn.microsoft.com/en-us/windows/msix/msix-containerization-overview).

A first emulator build should use a full-trust desktop MSIX because it is most compatible with existing Win32 emulator code. The stricter AppContainer trust level can be tested after graphics, raw controller input, content access, and core loading are working. Every deployable MSIX must be signed and trusted by the destination machine, so signing is part of the backend rather than an optional release chore; see Microsoft's [MSIX signing overview](https://learn.microsoft.com/en-us/windows/msix/package/signing-package-overview).

For a no-install mode, EmuContainer can also emit a signed portable folder or archive containing a small launcher, RetroArch, a core, configuration, and permitted content. That mode is easier to move but has weaker isolation and update behavior than MSIX. A self-extracting single `.exe` can be explored later, but it should not define the first Windows proof of concept.

### macOS

A macOS `.app` already behaves like one launchable object in Finder while internally being a structured directory containing executable code and resources. Apple's [bundle structure documentation](https://developer.apple.com/go/?id=bundle-structure) explicitly supports executables, resources, private frameworks, and plug-ins in that unit. Libretro's [macOS compilation guide](https://docs.libretro.com/development/retroarch/compilation/osx/) confirms that RetroArch can run on ARM- and Intel-powered Macs.

The output should be a universal application containing both Apple silicon and Intel slices for the launcher, frontend, and every compiled core. Apple documents this model in [Building a universal macOS binary](https://developer.apple.com/documentation/Apple-Silicon/building-a-universal-macos-binary). Normal distribution outside the App Store requires Developer ID signing and notarization for a smooth Gatekeeper experience; see [Notarizing macOS software before distribution](https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution).

The first backend should bundle and sign every core rather than download executable cores at runtime. The [Hardened Runtime](https://developer.apple.com/documentation/Security/hardened-runtime) restricts JIT compilation and arbitrary code loading unless the appropriate entitlements are granted. Start with an interpreter-based libretro core, then test JIT-dependent emulators separately. Apple's App Sandbox is required for Mac App Store distribution and can be enabled for non-store builds, but it should be treated as a compatibility target rather than assumed to work with every emulator on day one.

### Android

Android already gives each installed application its own operating-system-enforced sandbox and private data area, as described in [Application fundamentals](https://developer.android.com/guide/components/fundamentals). The installable and executable package is an APK; an Android App Bundle is a publishing input from which a distributor creates APKs, not something a player launches directly. See the [Android App Bundle FAQ](https://developer.android.com/guide/app-bundle/faq).

A per-game APK can contain a thin controller-first launcher, the selected core as a native shared library, default configuration, artwork, and legally distributable content. Android expects native libraries under ABI-specific APK paths, so `arm64-v8a` should be the first hardware target and `x86_64` can be added for emulators, Chromebooks, and other applicable devices; see the [Android NDK ABI guide](https://developer.android.com/ndk/guides/abis.html).

Android's current game-controller APIs support connection changes, standardized mappings, multiple controllers, and haptics. That makes Android TV and handheld Android devices credible console-style targets, although touch UI, activity lifecycle, storage rules, and TV launcher metadata remain platform-specific work. See [Android game controller support](https://developer.android.com/games/sdk/game-controller/overview).

### WebAssembly as a portability branch

RetroArch itself lists Emscripten/WebAssembly among its supported platforms, and Libretro provides a [browser-based RetroArch player](https://docs.libretro.com/guides/web-player/). [EmulatorJS](https://github.com/EmulatorJS/EmulatorJS) is further evidence that libretro cores can be presented through a web frontend.

This route comes closest to one runtime running across Windows, macOS, Linux, and Android. Emscripten can embed or preload game data into its virtual filesystem, and save data can persist through IndexedDB-backed storage; see [Packaging files](https://emscripten.org/docs/porting/files/packaging_files.html) and the [Emscripten filesystem overview](https://emscripten.org/docs/porting/files/file_systems_overview.html). Browsers also expose controllers through the [W3C Gamepad API](https://www.w3.org/TR/gamepad/).

The tradeoff is emulator coverage and host integration. Browser builds have different threading, storage, audio, fullscreen, performance, and JIT constraints, and the available web core list is only a subset of native emulation. The web backend is promising for older consoles and easy sharing, but native backends should remain the primary direction.

## The cross-platform architecture

The common input should be a source package, not a prebuilt Linux root filesystem:

```text
game-package/
  emucontainer.toml
  content/              # only when redistribution is permitted
  config/
  artwork/
  licenses/
  patches/
  lock.json             # exact frontend/core/tool revisions and hashes
```

The build flow would be:

```text
one game recipe
      |
      +-- Linux adapter  --> .flatimage / Flatpak
      +-- Windows adapter --> .msix / portable archive
      +-- macOS adapter   --> signed .app / DMG
      +-- Android adapter --> signed .apk
      +-- Web adapter     --> offline-capable web bundle
```

The manifest should describe intent at a platform-neutral level:

- Game identity, metadata, artwork, and content requirements.
- Runner kind, initially `libretro`, with the frontend and core pinned by version and hash.
- Supported target operating systems and CPU architectures.
- Abstract capabilities such as display, audio, GPU, controller input, optional user-selected files, and network. Each backend maps those declarations to native permissions.
- Default emulator configuration and controller mappings.
- Save, save-state, screenshot, and log categories.
- Component licenses and source-offer obligations.
- Platform overrides only where behavior genuinely differs.

The payload cannot be completely identical. RetroArch and its cores need OS- and CPU-specific binaries, and some cores are unavailable or behave differently on some targets. The lock file should therefore record a logical component plus a hash for each target build rather than pretend one binary works everywhere.

Player state should follow one logical layout while mapping onto native storage:

| Platform | Native state location |
| --- | --- |
| Linux | XDG data/config/state directories or an explicit FlatImage binding |
| Windows | MSIX per-package application data or `%LOCALAPPDATA%` for portable mode |
| macOS | The app's sandbox container or `~/Library/Application Support/EmuContainer` |
| Android | App-private files and preferences |
| Web | IndexedDB-backed browser storage |

Battery-backed/native saves may be portable across platforms when the same core and format are used. Save states often are not. The manifest and state metadata should identify the core and version that produced each state instead of promising universal save-state compatibility.

## Target technologies

The best current technology set is:

- **Package contract:** TOML manifest plus a generated machine-readable schema and a lock file containing content hashes.
- **Build orchestrator:** a small Rust CLI. It can run on Windows, macOS, and Linux, validate the contract, manage a content-addressed cache, and invoke platform adapters without becoming the emulator frontend.
- **Common emulator interface:** libretro first. RetroArch describes libretro as a portable audio/video/input API whose C or C++ cores can run on many platforms with little or no porting, and RetroArch already targets Android, macOS, Linux, Windows, and Emscripten; see the [RetroArch repository](https://github.com/libretro/RetroArch).
- **Linux adapter:** GameImage/FlatImage, with Flatpak retained as the behavior reference and temporary ARM64 path.
- **Windows adapter:** native Win32 RetroArch plus MSIX tooling and signing on a Windows runner.
- **macOS adapter:** native RetroArch, universal application-bundle generation, `codesign`, and `notarytool` on a macOS runner.
- **Android adapter:** Gradle, Android NDK, signed APK output, and a thin Android lifecycle/controller wrapper around the selected libretro core or reusable RetroArch Android code.
- **Web adapter:** Emscripten-built cores and an offline-capable web frontend, added after the native contract is stable.
- **Build hosts:** local native Windows and Linux development environments. Each backend invokes the SDK and signing tools installed for its target; WSL2 or a local Linux virtual machine may provide a convenient Linux build environment from a Windows workstation.

Local builds should work when the relevant target SDK is installed. A future `emucontainer build --target all` may coordinate explicitly configured local target environments, but it will not submit builds to GitHub Actions. This is a more honest form of build portability than hiding several operating-system SDKs inside one container.

## Project boundary and upstream strategy

EmuContainer should own:

- The cross-platform manifest and lock format.
- Target selection and native artifact orchestration.
- The abstract permissions and persistent-state contract.
- Agent-operable manual checks for launch, controller, fullscreen, save, shutdown, and network behavior.
- Frontend integration metadata.
- Cross-platform diagnostics and component/license inventory.

EmuContainer should not own:

- A replacement Linux namespace/FUSE runtime.
- A forked emulator or libretro API.
- A new controller-first game library UI during the first phases.
- A virtual machine layer intended to make Linux executables masquerade as native packages.

Potential FlatImage/GameImage improvements such as ARM64 support, narrower generated permissions, a noninteractive recipe workflow, and better external state bindings should be attempted upstream. A fork should be a last resort after a concrete incompatibility and an unsuccessful upstream path.

## Recommended proof of concept

The next experiment should test whether the proposed project boundary is real:

1. Choose the same legally clean contentless core or redistributable homebrew used by the Linux benchmark.
2. Write one minimal `emucontainer.toml` and lock file.
3. Generate a Linux x86-64 GameImage without creating a new Linux runtime.
4. Generate a native Windows x64 package from the same recipe, starting with MSIX and optionally a portable archive.
5. Verify that both launch directly, use a controller, run fullscreen, keep saves outside immutable files, exit cleanly, and have network access disabled.
6. If the contract survives those two materially different backends, add a universal macOS `.app` and an Android ARM64 APK.
7. Add a WebAssembly build only for a core already known to work well in the browser.

Linux plus Windows is the best first architectural test because it forces the manifest to cross the Linux/Windows runtime boundary immediately. Building only GameImage and Flatpak would compare two Linux delivery systems, but it would not prove that EmuContainer has a distinct cross-platform purpose.

## Go/no-go rule

Continue EmuContainer as its own project if a single declarative recipe can produce the Linux and Windows proof-of-concept artifacts without embedding platform-specific shell scripts in the game recipe.

Stop at upstream contribution if the desired scope narrows to Linux x86-64, or if the common contract becomes only a thin list of unrelated per-platform build scripts with no shared behavior to test.

On the evidence available now, the multi-platform requirement creates a legitimate project: a cross-platform compiler for self-contained emulated-game applications, with GameImage supplying the Linux implementation.
