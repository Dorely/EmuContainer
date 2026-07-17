# EmuContainer File Map

This is the canonical inventory of every version-controlled file in EmuContainer. It covers source, configuration, documentation, examples, and local development tooling; generated build output, Git metadata, and ignored machine-local files are not repository files and are not listed.

Agents must keep this table synchronized with the repository in the same change that adds, deletes, renames, moves, or materially repurposes a file. Every repository file appears exactly once.

| File | Purpose | Function | Related features |
| --- | --- | --- | --- |
| `.editorconfig` | Shared editor defaults | Standardizes UTF-8, indentation, line endings, final newlines, and whitespace handling across common file types. | Maintainability; cross-platform editing |
| `.gitattributes` | Git content rules | Normalizes text line endings while preserving Windows script endings and marking package media formats as binary. | Cross-platform development; artifact inputs |
| `.gitignore` | Repository exclusion rules | Prevents build output, local state, editor metadata, symbols, and signing credentials from entering version control. | Local builds; secrets; packaging |
| `AGENTS.md` | Repository-wide agent operating policy | Requires vision and file-map startup reads, codebase search and reuse, dead-code removal, local-only validation, and agent-operable manual verification. | Agentic development; maintainability; repository governance |
| `Cargo.lock` | Locked Rust dependency graph | Pins the exact transitive package versions used by reproducible local builds. | Rust CLI; reproducibility; dependency integrity |
| `Cargo.toml` | Rust package manifest | Defines the EmuContainer crate, compiler baseline, runtime dependencies, metadata, and lint policy. | CLI; manifest validation; local builds |
| `CONTRIBUTING.md` | Change and handoff guide | Documents prerequisites, local checks, implementation boundaries, file-map discipline, and evidence expected from contributors. | Agentic development; local verification; package contract |
| `FILEMAP.md` | Canonical repository inventory | Describes the purpose, function, and feature relationships of every version-controlled file. | Repository navigation; change integrity; agent startup |
| `README.md` | Project entry point | Summarizes the Windows/Linux product goal, scaffold status, quick-start commands, repository layout, and project rules. | Developer onboarding; MVP; local workflow |
| `VISION.md` | Product vision and boundaries | States in plain language why EmuContainer exists, the desired player experience, priorities, initial limits, and first meaningful success. | Product direction; controller-first experience; portability |
| `docs/development.md` | Architecture and development guide | Defines shared recipe and backend boundaries, toolchain commands, current contract, planned modules, content rules, and local verification policy. | Rust CLI; Linux GameImage backend; Windows MSIX backend |
| `docs/manual-test-plan.md` | Agent-operable functional verification plan | Defines evidence requirements, current Windows/Linux scaffold scenarios, manifest checks, and planned native-package behavior scenarios. | Local builds; recipe validation; cross-platform behavior |
| `docs/mvp.md` | First product milestone | Defines the one-recipe Windows/Linux proof of concept, acceptance criteria, milestone sequence, deferrals, and go/no-go condition. | GameImage; MSIX; RetroArch/libretro; persistent saves |
| `docs/research/README.md` | Research index and conclusion | Summarizes feasibility findings, the cross-platform opportunity, recommended native backends, and links to the detailed research set. | Technical direction; prior art; platform strategy |
| `docs/research/cross-platform-gap-analysis.md` | GameImage platform-gap analysis | Distinguishes artifact and build portability, maps native packaging options, and identifies the shared contract work EmuContainer should own. | GameImage; FlatImage; MSIX; macOS; Android; web |
| `docs/research/prior-art.md` | Related-project survey | Evaluates GameImage and adjacent emulator, gaming-container, frontend, and game-data projects for reusable lessons and unresolved gaps. | GameImage evaluation; FlatImage; RetroDECK; Steam runtime |
| `docs/research/recommended-direction.md` | Recommended technical approach | Proposes a backend-neutral package contract, shared immutable and mutable data model, initial manifest shape, security posture, and staged proof of concept. | Package recipe; backend adapters; state; permissions |
| `docs/research/sources.md` | Research source catalog | Records the primary repositories and official documentation supporting the feasibility and platform analysis. | Research traceability; technology selection; licensing |
| `docs/research/technology-assessment.md` | Linux technology comparison | Compares candidate runtimes and package formats against controller, graphics, isolation, persistence, portability, and maintenance criteria. | Flatpak; FlatImage; GameImage; OCI; AppImage; Conty |
| `examples/contentless/emucontainer.toml` | Legally clean example recipe | Exercises both initial target declarations and the libretro contract without bundling a ROM, firmware, or BIOS. | Manifest validation; Windows/Linux targets; permissions |
| `rust-toolchain.toml` | Pinned Rust toolchain declaration | Selects Rust 1.97.1 with the minimal profile, Clippy, and rustfmt for consistent local environments. | Reproducible development; linting; formatting |
| `rustfmt.toml` | Rust formatting policy | Selects Edition 2024 formatting semantics, Unix newlines, and field-init shorthand. | Code style; cross-platform diffs |
| `scripts/bootstrap.ps1` | Windows environment bootstrap | Installs the pinned Rust toolchain components and fetches locked dependencies from PowerShell. | Windows development; local setup; dependency cache |
| `scripts/bootstrap.sh` | POSIX environment bootstrap | Installs the pinned Rust toolchain components and fetches locked dependencies on Linux-compatible shells. | Linux development; local setup; dependency cache |
| `scripts/check.ps1` | Windows local quality and build entry point | Runs formatting verification, warning-denied Clippy, and a locked debug build from PowerShell. | Windows development; local build gates |
| `scripts/check.sh` | POSIX local quality and build entry point | Runs formatting verification, warning-denied Clippy, and a locked debug build from a Linux-compatible shell. | Linux development; local build gates |
| `src/lib.rs` | Shared Rust library boundary | Exposes the package-manifest module and its public contract types to the CLI and future build backends. | Reusable core; manifest API; backend integration |
| `src/main.rs` | EmuContainer command-line entry point | Parses CLI arguments, implements the `validate` command, loads a recipe, and emits a concise success summary or actionable error. | Agent interface; recipe validation; diagnostics |
| `src/manifest.rs` | Package recipe model and validator | Deserializes the versioned TOML schema and enforces identifiers, targets, runner data, content modes, portable paths, and checksums. | Package contract; libretro runner; permissions; content integrity |
