# Development Guide

## Architecture

EmuContainer v1 has a platform-neutral image toolchain and a Windows runtime:

```text
schema-v2 recipe + portable emulator root + ROM
                       |
               parse and validate
                       |
           deterministic image builder
                       |
                  one .emuc
                       |
          verify complete image inventory
                       |
        Windows cache + instance + state mounts
                       |
        direct launch under process supervision
```

The recipe declares data and launch behavior. Windows integration is isolated behind platform adapters, but the runtime contains no mGBA-, melonDS-, or game-specific branch: entrypoint, working directory, arguments, environment, and state mounts express those differences.

## Trust model

A `.emuc` image contains native executable code. Hash verification proves only that the bytes match the image inventory; it does not prove publisher identity, licensing, or safety.

v1 deliberately provides no sandbox and no network enforcement. The emulator runs as the invoking user with normal filesystem, registry, device, and network access. Every image reports `isolation = "none"` and `network_enforced = false`. Per-game paths are a state-management boundary, not a security boundary.

## Toolchain and commands

The repository pins Rust, rustfmt, and Clippy through `rust-toolchain.toml`.

```powershell
.\scripts\bootstrap.ps1
.\scripts\check.ps1
```

```sh
./scripts/bootstrap.sh
./scripts/check.sh
```

Equivalent local gates are:

```text
cargo fmt --all --check
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo build --locked
cargo build --release --locked
```

Validate the metadata-only examples:

```text
cargo run --locked -- validate examples/mgba-pokemon-blue/emucontainer.toml
cargo run --locked -- validate examples/melonds-pokemon-renegade-platinum/emucontainer.toml
```

Windows 10/11 x64 with NTFS-backed LocalAppData is required for runtime commands. Validation, building, and inspection remain usable on other development hosts; run and runtime lifecycle commands must return a clear unsupported-platform error there.

## Recipe schema v2

A recipe contains:

- `schema_version = 2`.
- Stable `id`, player-facing `title`, emulated `system`, and `target = "windows-x86_64"`.
- An `[emulator]` ID, version, provenance URL, relative executable, relative working directory, ordered arguments, and optional environment values.
- A `[content]` packaged filename and mandatory lowercase SHA-256.
- Optional `[[notices]]` mappings from a file inside the emulator input root to an output notice name.
- Optional `[[state.mounts]]` entries with a unique name, relative target under the emulator payload, and `seed = "if-empty"`.

The emulator root and ROM are explicit build inputs, never recipe fields. At least one argument or environment value must contain `{rom}`. The complete token set is:

| Token | Runtime value |
| --- | --- |
| `{emulator}` | Emulator root in the writable instance |
| `{rom}` | Packaged ROM in the writable instance |
| `{state}` | Stable per-game state directory |
| `{logs}` | Current run's log directory |
| `{instance}` | Current writable instance root |

Tokens expand once. Unknown tokens, unmatched braces, and nested expansion are validation errors. The runtime launches the resulting executable and argument vector directly without a shell.

The all-flags `build` form has the same contract. Recipe fields and inline contract flags are mutually exclusive so there is one source of truth per invocation.

## Image format

`.emuc` is a deterministic ZIP container:

```text
image.json
payload/emulator/**
payload/content/<declared filename>
notices/**
```

`image.json` records format and minimum-runtime versions, game/content/emulator identities, provenance, target, launch data, state mounts, isolation facts, and a sorted inventory of every file and empty directory. File entries include size and SHA-256.

Builder invariants include sorted UTF-8 forward-slash paths, canonical typed JSON, fixed ZIP timestamps and permissions, pinned compression behavior, and atomic same-directory publication. Existing output requires `--force`; a failed build must leave the previous output untouched.

One shared Windows path policy is used during recipe validation, input walking, image inspection, and materialization. It rejects:

- Absolute, rooted, parent, backslash-form, empty-segment, and non-UTF-8 paths.
- Alternate-data-stream colons, control characters, reserved Windows device names, and trailing dots or spaces.
- Case-insensitive collisions and duplicate archive entries.
- Symlinks, junctions, and other input reparse points.
- Mount `.`; nested/overlapping mounts; and mounts covering the entrypoint.

The verifier additionally rejects missing, undeclared, encrypted, unsupported-compression, size-mismatched, and hash-mismatched archive entries. No packaged process may start before complete verification succeeds.

## Runtime layout

The user-scoped installation is separate from managed game data:

```text
%LOCALAPPDATA%\Programs\EmuContainer\
  emucontainer.exe
  install.json

%LOCALAPPDATA%\EmuContainer\
  cache\images\<image-sha256>\
    image.json
    rootfs\emulator\
    rootfs\content\
    notices\
    complete.json
  containers\<game-id>\<image-sha256>\rootfs\
  games\<game-id>\
    identity.json
    state\
    volumes\<mount-name>\
    logs\<run-id>\
      events.jsonl
      stdout.log
      stderr.log
  logs\launcher\
  locks\
```

Caches and container snapshots are disposable. Identity, state, volumes, and logs are durable and remain after pruning and uninstalling.
Launcher-level JSONL logs record requests rejected before a game log can be created. Per-run `cache_ready` events distinguish `created`, `reused`, and `repaired` materialization.

## Runtime flow

`run` performs these operations in order:

1. Validate the platform and runtime-owned roots, then acquire a shared lifecycle lock before opening the image.
2. Hash and completely verify the image, its target, and minimum runtime version.
3. Acquire a nonblocking game-ID lock, then the digest-specific materialization lock.
4. Reuse a complete verified cache or safely rebuild a corrupt/incomplete cache through a temporary sibling and atomic rename.
5. Validate durable game identity. Emulator version may change; game ID, system, ROM hash, and emulator ID may not.
6. Create or reuse the image-digest-specific writable instance and reverify its entrypoint and ROM before launch.
7. Seed each named volume through staging, atomically publish it once, persist an initialization marker, and replace its instance target with an NTFS directory junction.
8. Expand tokens, create state/log paths, and open stdout, stderr, and JSONL event logs.
9. Launch directly under a Job Object and wait for the entire emulator process tree.
10. Record root exit status and final descendant count, then release locks while retaining instance and state.

An intentionally emptied initialized volume is not reseeded. A second concurrent launch of the same game ID fails instead of sharing mutable state.

The runtime joins itself to a kill-on-close Job Object before spawning the emulator so descendants inherit supervision without a launch race. Failure to establish supervision aborts launch. Normal exit waits for descendants and releases the job without killing them; abnormal runtime termination closes the job and kills the remaining tree.

## Runtime lifecycle

- `runtime install` copies the running binary and `install.json` to the user installation directory and registers `.emuc` plus `EmuContainer.Image` under `HKCU\Software\Classes`.
- `runtime status` checks the installed binary/hash and registry ownership.
- `runtime prune` takes the lifecycle lock and removes only inactive caches and container snapshots.
- `runtime uninstall` removes only unchanged registry values owned by this installation and its installed binary. Installed-binary self-uninstall uses a hidden, generation-hash-bound temporary worker and removes that worker after completion. It preserves all game data.

The v1 association opens `"<installed>\emucontainer.exe" run "%1"`; one console window is acceptable.

## Private fixture workflow

All ROMs, emulator archives/folders, generated images, saves, logs, and evidence outputs live under `.emucontainer/`. The repository's single `/.emucontainer/` rule already ignores the entire tree; do not add redundant patterns or list local files in `FILEMAP.md`.

Start from clean official portable archives. Never copy a personal emulator directory containing saves or configuration. Verify expected hashes before building and package required notices from inside the clean input root. The checked-in examples intentionally contain metadata only.

Use [the manual test plan](manual-test-plan.md) for exact input paths, checksums, commands, and evidence. Private ROMs and derived `.emuc` files must not be published.

## Verification policy

Automated suites, test harnesses, coverage systems, CI services, and GitHub Actions are deferred until the working product needs regression protection. Local formatting, warning-denied Clippy, compilation, and agent-operable manual scenarios are required instead.

Update the manual plan whenever observable behavior changes. Record deterministic inputs, exact commands, exit codes, artifact/log/state paths, and machine-readable signals. Isolate only unavoidable graphics, audio, fullscreen, controller, and gameplay recognition as human observations.
