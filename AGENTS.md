# EmuContainer Agent Instructions

## Scope

These instructions apply to the entire repository. EmuContainer is expected to be built and maintained primarily by coding agents, so repository context, change discipline, and agent-operable verification are required parts of every change.

## Required session startup

Before planning work, searching the repository, editing files, or running a build in a new session:

1. Read `VISION.md` in full to understand the product goal and boundaries.
2. Read `FILEMAP.md` in full to understand the current repository structure and file ownership.

Do not rely on memory or an earlier summary in place of these files.

## Canonical file inventory

`FILEMAP.md` is the canonical inventory of every version-controlled file in the repository. Every row must state the file's purpose, function, and related features.

Updating `FILEMAP.md` is mandatory in the same change whenever a file is added, deleted, renamed, moved, or materially repurposed. Each tracked file must appear exactly once, and no nonexistent file may remain listed. A change is not finished and must not be committed while the file map is stale. Compare the file map with the actual repository inventory before handing off any change.

## Search before implementation

Before writing code, use repository-wide file and text searches to find existing implementations, callers, related types, configuration, and documentation. Read the relevant implementation and its consumers before deciding where the change belongs.

New code must not duplicate a function already performed elsewhere. When existing behavior is too narrow, refactor or generalize it into a reusable implementation and update its callers. Prefer one maintained source of truth over parallel implementations, including across platform adapters.

## No dead or superseded code

Dead, superseded, and unused code is forbidden. When functionality is replaced, delete the old implementation in the same change along with obsolete callers, exports, dependencies, configuration, documentation, and examples. Do not retain commented-out alternatives, speculative compatibility shims, empty placeholder modules, or unused branches for possible future use.

Treat compiler and linter warnings as errors. After removing or replacing behavior, search the repository for stale references before considering the work complete.

## Maintainable implementation standards

- Keep modules focused and interfaces explicit.
- Put platform-neutral behavior in reusable shared code; isolate operating-system packaging details in target adapters.
- Prefer declarative package recipes and shared planning logic over duplicated target scripts.
- Keep mutable player state separate from immutable package payloads.
- State effective capabilities honestly. The v1 Windows runtime is not a sandbox and cannot enforce network denial; future isolation work should default to the least capability required.
- Add dependencies only when they provide clear value that cannot be served by existing code or the standard library.
- Update user and developer documentation whenever behavior or workflows change.

## Current validation policy

Automated test suites, test harnesses, test-only dependencies, coverage systems, continuous-integration services, and GitHub Actions are intentionally deferred until EmuContainer has a working product whose behavior needs regression protection. Do not add them to the current implementation, roadmap, or architecture unless the user explicitly changes this policy.

For now, validate changes with local formatting, linting, compilation, and the agent-operable manual scenarios in `docs/manual-test-plan.md`. Update that plan whenever a feature adds or changes observable behavior.

## Agent-operable manual verification

Design commands and workflows so an agent can execute and evaluate them without relying on an interactive desktop whenever practical. Commands must have deterministic inputs, meaningful exit codes, concise diagnostics, and discoverable artifact or log paths.

For graphics, audio, controller, fullscreen, and launcher behavior, expose every machine-verifiable signal available: process state, structured logs, device enumeration, configuration, artifact metadata, save-file changes, and clean shutdown status. Isolate any unavoidable human observation as a clearly labeled step rather than making an entire scenario human-only.

Record manual verification using the evidence format in `docs/manual-test-plan.md`.

## Required change sequence

1. Read `VISION.md` and `FILEMAP.md` at session start.
2. Search for existing related functionality and references.
3. Refactor existing code or implement the smallest reusable change.
4. Remove all behavior and references made obsolete by the change.
5. Update documentation and manual verification scenarios.
6. Synchronize `FILEMAP.md` with the final file inventory.
7. Run the relevant local build and manual checks, then report concrete evidence.
