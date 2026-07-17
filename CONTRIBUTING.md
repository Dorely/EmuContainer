# Contributing to EmuContainer

## Prerequisites

- Git.
- Rust installed through [rustup](https://rustup.rs/).
- A native Windows or Linux environment for platform-adapter verification.

The repository pins its Rust version and required `rustfmt` and Clippy components in `rust-toolchain.toml`.

## Local checks

Run the platform-appropriate local verification script before sharing a change:

```text
.\scripts\check.ps1
./scripts/check.sh
```

The scripts execute:

```text
cargo fmt --all --check
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo build --locked
```

Run the applicable functional scenarios from [the manual test plan](docs/manual-test-plan.md). For example, validate the checked-in manifest with:

```text
cargo run -- validate examples/contentless/emucontainer.toml
```

## Change boundaries

- Read `VISION.md` and `FILEMAP.md` at the beginning of every agent session.
- Search the codebase before implementing behavior. Reuse or generalize the existing implementation instead of duplicating its function.
- Delete dead, superseded, and unused code and all stale references in the same change that replaces them.
- Keep `FILEMAP.md` synchronized whenever any repository file is added, deleted, renamed, moved, or materially repurposed.
- Keep package recipes platform-neutral.
- Put operating-system-specific packaging code behind an adapter.
- Update the agent-operable manual test plan whenever observable behavior changes. Automated regression infrastructure is deferred until the project has a working product.
- Do not commit build outputs, signing credentials, personal game dumps, copyrighted firmware, or unverified ROM images.
- Record the license and checksum of every redistributable binary or content asset added to an example.
- Prefer changes to GameImage or FlatImage upstream when the problem belongs to their Linux runtime rather than EmuContainer's contract.

## Change handoff

Keep changes focused and explain:

- What changed.
- Why the shared package contract or a target adapter needs it.
- Which local builds and manual scenarios were run, with their operating systems and concrete evidence.
- Any new host permissions, bundled binaries, licenses, or signing requirements.
