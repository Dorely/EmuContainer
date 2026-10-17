# Contributing to EmuContainer

## Issues and pull requests

Use [GitHub issues](https://github.com/Dorely/EmuContainer/issues) for reproducible bugs and scoped feature proposals. Include the CLI and emulator versions, operating system, expected and actual behavior, exact commands, and sanitized diagnostics. Never upload private inputs or generated game images; remove personal paths and credentials from logs before sharing.

For a pull request, describe the resulting behavior and include the relevant local-check and manual-scenario evidence below. Contributions are under the repository's [MIT License](LICENSE).

## Start with repository truth

At the beginning of every agent session:

1. Read `VISION.md` and `FILEMAP.md` in full.
2. Search the repository for the behavior, callers, types, documentation, and examples affected by the change.
3. Reuse or generalize existing code instead of creating a second implementation.

Delete superseded behavior and stale references in the same change. Keep `FILEMAP.md` synchronized whenever a tracked file is added, deleted, moved, renamed, or materially repurposed.

## Product boundaries

- v1 builds one Windows x64 `.emuc` image from one schema-v2 recipe, one portable emulator folder, and one ROM.
- The recipe is declarative and emulator-neutral. Emulator differences belong in entrypoint, arguments, environment, configuration files, and declared state mounts, not hard-coded adapters.
- Immutable image contents, disposable caches, writable instances, and stable player state remain distinct.
- A `.emuc` image is an unsigned native-code package. Do not describe it as safe merely because its hashes verify.
- v1 has no sandbox and no enforced network restriction. Do not add permissions language that implies otherwise.
- Linux and other platform runtimes, production signing, automatic downloads, and GUI-launcher polish remain deferred.

## Content and local fixtures

Never commit personal ROMs, firmware, BIOS files, emulator archives or extracted emulator folders, generated game images, saves, signing material, or credentials. Keep them under `.emucontainer/`, which is already ignored.

Checked-in recipes contain only metadata, expected checksums, and relative package configuration. Before using a local binary:

- Record its source and exact version.
- Verify its SHA-256 checksum.
- Retain every required license and notice in the emulator input folder so the builder can package it.
- Confirm `git check-ignore -v --no-index <path>` identifies the repository's `/.emucontainer/` rule.
- Confirm `git ls-files --error-unmatch <path>` fails and `git status --short` does not list the fixture.

Do not copy an existing personal emulator installation into a fixture: it may contain saves, credentials, play history, or machine-specific configuration. Start from a clean official portable archive.

## Local checks

Prerequisites are Git, rustup, and a native Windows host for runtime verification. The repository pins Rust, rustfmt, and Clippy.

```powershell
.\scripts\bootstrap.ps1
.\scripts\check.ps1
```

```sh
./scripts/bootstrap.sh
./scripts/check.sh
```

The scripts run formatting verification, warning-denied Clippy, and a locked build. Use `cargo fmt --all` to apply Rust formatting.

Validate both public recipes:

```powershell
cargo run --locked -- validate examples/mgba-pokemon-blue/emucontainer.toml
cargo run --locked -- validate examples/melonds-pokemon-renegade-platinum/emucontainer.toml
```

Run every applicable scenario in [docs/manual-test-plan.md](docs/manual-test-plan.md). Automated tests, harnesses, coverage, continuous integration, and GitHub Actions are intentionally deferred by repository policy.

## Implementation discipline

- Keep shared recipe, path policy, image, and runtime concerns in focused modules with explicit interfaces.
- Use one Windows-safe path validator for building, inspecting, and materializing images.
- Reject unsafe or ambiguous input instead of normalizing it silently.
- Do not follow symlinks or reparse points while packaging inputs.
- Never execute an image before its declared inventory and hashes verify.
- Launch entrypoints directly with an ordered argument vector; never pass package-controlled values through a shell.
- Treat caches and instances as disposable, but never delete identity records, state, volumes, or logs as a side effect of pruning or uninstalling.
- Keep commands deterministic, diagnostics concise, exit codes meaningful, and evidence paths discoverable.
- Treat compiler and linter warnings as errors and remove dead code immediately.

## Handoff evidence

Agents must commit and push each completed, validated change before handing off, following [AGENTS.md](AGENTS.md#commit-and-push-every-completed-change), unless the user explicitly requests otherwise. Include the commit hash, destination branch, and push result in the handoff.

Report:

- What behavior and public contracts changed.
- Every tracked file added, deleted, moved, or materially repurposed.
- Exact formatting, linting, build, and manual scenario commands run.
- Operating system, exit codes, artifact hashes, log paths, and state paths inspected.
- Any new native binary, notice, dependency, registry value, or host capability.
- Human-observation steps that remain for graphics, audio, fullscreen, or controller behavior.

Do not claim a manual scenario passed when it was not executed on the required native host.
