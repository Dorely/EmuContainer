# EmuContainer File Map

This is the canonical inventory of every version-controlled file in EmuContainer, grouped by primary feature. Ignored private fixtures, generated `.emuc` images, runtime data, build output, and Git metadata are deliberately excluded.

Every repository file must appear exactly once. Agents must update this map in the same change whenever a tracked file is added, deleted, renamed, moved, or materially repurposed.

## Product direction and milestones

| File | Purpose | Function | Related features |
| --- | --- | --- | --- |
| `README.md` | Public project entry point | Explains project status, prerequisites, source setup, Runtime+Image workflows, storage, feedback, licensing, and funding. | User onboarding; `.emuc` images; Windows runtime; sharing |
| `VISION.md` | Product vision and boundaries | Defines the Docker-like image/runtime experience, immutable and persistent layers, v1 limits, and success criteria. | Product direction; portability; player state |
| `docs/mvp.md` | v1 proof-of-concept contract | Fixes the recipe, image, runtime, lifecycle, verification, and explicit deferral scope for the first usable release. | Schema v2; deterministic images; Windows x64 |

## Repository governance and contribution

| File | Purpose | Function | Related features |
| --- | --- | --- | --- |
| `AGENTS.md` | Repository-wide agent policy | Requires startup context reads, search and reuse, dead-code removal, documentation synchronization, manual evidence, and committing and pushing completed changes. | Agentic development; maintainability; verification; delivery |
| `CONTRIBUTING.md` | Contributor handoff guide | Documents supported scope, private-input handling, local quality gates, manual checks, and file-map discipline. | Development workflow; ROM hygiene; review |
| `LICENSE` | Project license | Grants MIT permissions and states copyright, conditions, and warranty disclaimer. | Public distribution; contribution terms |
| `.github/FUNDING.yml` | GitHub funding configuration | Connects the repository Sponsor button to Dorely's Buy Me a Coffee page. | Optional project support |
| `FILEMAP.md` | Canonical tracked-file inventory | Describes every version-controlled file once and records feature ownership and relationships. | Repository navigation; change integrity |

## Rust package and command-line surface

| File | Purpose | Function | Related features |
| --- | --- | --- | --- |
| `Cargo.lock` | Locked dependency graph | Pins exact transitive Rust packages for reproducible locked builds. | Dependency integrity; release builds |
| `Cargo.toml` | Rust package manifest | Declares package metadata, shared dependencies, Windows-only runtime dependencies, and lint policy. | CLI; ZIP/SHA support; Windows integration |
| `src/lib.rs` | Shared library boundary | Exposes the recipe, path-policy, image, and runtime subsystems to the binary and future consumers. | Reusable core; public API |
| `src/main.rs` | CLI entry point | Implements validate, recipe/all-flags build, inspect, run, and runtime lifecycle commands with meaningful exit codes. | Automation; artifact operations; installed launcher |

## Recipe, path, and image core

| File | Purpose | Function | Related features |
| --- | --- | --- | --- |
| `src/path_policy.rs` | Portable path policy | Applies one Windows-safe relative-path contract, archive-path validation, collision keys, and reparse detection. | Builder safety; verifier safety; extraction safety |
| `src/recipe.rs` | Schema-v2 recipe model | Parses strict TOML, aggregates semantic diagnostics, validates mounts and tokens, and expands launch values once. | Build contract; generic emulators; CLI flags |
| `src/image/mod.rs` | Image subsystem facade | Re-exports build, model, and reader APIs and defines shared image errors and digest helpers. | `.emuc` format; library API |
| `src/image/model.rs` | Canonical image metadata | Defines `image.json`, inventory structures, format limits, and embedded-contract validation. | Integrity; compatibility; deterministic JSON |
| `src/image/build.rs` | Deterministic image builder | Validates inputs, rejects links and collisions, inventories and hashes payloads, writes canonical ZIP data, and atomically publishes. | Emulator packaging; ROM verification; reproducibility |
| `src/image/read.rs` | Image verifier and extractor | Rejects malformed archives, verifies metadata/inventory/content without extraction, then supports staged verified extraction. | Inspect; tamper rejection; runtime cache |

## Windows runtime and lifecycle

| File | Purpose | Function | Related features |
| --- | --- | --- | --- |
| `src/runtime/mod.rs` | Runtime facade | Exposes launch and lifecycle operations through discovered user-scoped paths. | CLI integration; platform boundary |
| `src/runtime/paths.rs` | Runtime storage layout | Resolves installation, cache, container, game-state, log, and lock paths below LocalAppData. | Disposable data; persistent data; installation |
| `src/runtime/locks.rs` | Runtime locking | Provides blocking digest locks and nonblocking game/lifecycle file locks. | Concurrency; prune safety; materialization |
| `src/runtime/fs_util.rs` | Safe runtime filesystem utilities | Hashes, atomically writes JSON, copies normal trees, and removes trees without traversing junctions or links. | Cache repair; snapshots; prune/uninstall |
| `src/runtime/materialize.rs` | Verified image cache | Extracts images into digest-addressed staging directories, verifies cached trees, repairs corruption, and atomically publishes. | Content-addressed cache; integrity; cold launch |
| `src/runtime/instance.rs` | Writable instances and volumes | Creates digest-specific snapshots, protects game identity, seeds persistent volumes once, and mounts them with NTFS junctions. | Saves; emulator updates; state isolation |
| `src/runtime/logging.rs` | Structured lifecycle logging | Creates timestamped JSONL events and optionally mirrors them to standard output. | Diagnostics; manual evidence; launcher behavior |
| `src/runtime/process.rs` | Windows process-tree supervision | Uses a current-process Job Object so descendants are awaited and killed if the runtime terminates unexpectedly. | Clean shutdown; exact root exit status |
| `src/runtime/launch.rs` | Runtime execution pipeline | Verifies images, acquires locks, prepares cache/instance/state, expands arguments, launches directly, and records results. | `run`; emulator neutrality; logs |
| `src/runtime/lifecycle.rs` | Runtime installation lifecycle | Installs and hashes the executable, owns the HKCU association, reports health, prunes disposable data, and self-uninstalls. | Double-click launch; status; data preservation |

## Metadata-only local-fixture examples

| File | Purpose | Function | Related features |
| --- | --- | --- | --- |
| `examples/mgba-pokemon-blue/emucontainer.toml` | mGBA recipe example | Describes the ignored local mGBA 0.10.5 and Game Boy ROM fixture without tracking binaries. | Recipe mode; state tokens; notices |
| `examples/melonds-pokemon-renegade-platinum/emucontainer.toml` | melonDS recipe example | Describes the ignored melonDS 1.1 and Nintendo DS ROM fixture with a persistent portable-data mount. | Volume seeding; direct boot; notices |

## Local development and manual verification

| File | Purpose | Function | Related features |
| --- | --- | --- | --- |
| `.editorconfig` | Shared editor defaults | Standardizes UTF-8, indentation, line endings, final newlines, and whitespace handling. | Maintainability; cross-platform editing |
| `.gitattributes` | Git content rules | Normalizes text, preserves script line endings, and marks selected image/media artifact extensions as binary. | Artifact hygiene; cross-platform diffs |
| `.gitignore` | Repository exclusions | Excludes build output, the private workspace, common game payloads and saves, local settings, and credentials. | ROM privacy; generated images; runtime evidence |
| `docs/development.md` | Architecture and developer guide | Explains module ownership, data flow, path and integrity invariants, commands, and supported platform behavior. | Implementation maintenance; local workflow |
| `docs/manual-test-plan.md` | Manual verification specification | Defines MT-010 through MT-180 scenarios and evidence fields for public sharing, fixtures, builds, rejection, runtime, persistence, and lifecycle. | Agent-operable evidence; human observation boundaries |
| `rustfmt.toml` | Rust formatting policy | Selects Edition 2024 formatting semantics and consistent line endings. | Code style; clean diffs |
| `rust-toolchain.toml` | Pinned Rust toolchain | Selects Rust 1.97.1 with rustfmt and Clippy. | Reproducible development; linting |
| `scripts/bootstrap.ps1` | Windows bootstrap | Installs the pinned toolchain components and fetches locked dependencies from PowerShell. | Windows setup; dependency cache |
| `scripts/bootstrap.sh` | POSIX bootstrap | Installs the pinned toolchain components and fetches locked dependencies from a compatible shell. | Non-Windows build/inspect development |
| `scripts/check.ps1` | Windows quality gate | Runs formatting, warning-denied Clippy, and a locked debug build. | Local verification; handoff evidence |
| `scripts/check.sh` | POSIX quality gate | Runs the same formatting, lint, and locked-build gate in a compatible shell. | Cross-platform build/inspect verification |

## Historical research

These files record the superseded GameImage/MSIX investigation. They remain useful background, but `VISION.md` and `docs/mvp.md` define v1.

| File | Purpose | Function | Related features |
| --- | --- | --- | --- |
| `docs/research/README.md` | Historical research index | Marks the earlier backend investigation as superseded and links the retained studies. | Decision history; v1 context |
| `docs/research/cross-platform-gap-analysis.md` | Historical platform-gap analysis | Compares artifact and build portability across earlier native-packaging directions. | GameImage; MSIX; platform strategy |
| `docs/research/prior-art.md` | Historical related-project survey | Records reusable lessons from emulator, frontend, and gaming-container projects. | GameImage; RetroDECK; Steam runtime |
| `docs/research/recommended-direction.md` | Historical architecture proposal | Captures the prior backend-neutral manifest and packaging recommendation. | Superseded schema; state; permissions |
| `docs/research/sources.md` | Research source catalog | Lists repositories and official documentation used by the historical assessment. | Traceability; licensing research |
| `docs/research/technology-assessment.md` | Historical Linux technology comparison | Compares Flatpak, FlatImage, GameImage, OCI, AppImage, and Conty against project criteria. | Linux packaging; prior evaluation |
