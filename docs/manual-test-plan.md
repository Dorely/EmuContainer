# Agent-Operable Manual Test Plan

## Purpose

This is the current functional verification plan for EmuContainer. Automated regression suites and continuous integration are deferred until a working product exists. Formatting, linting, and compilation remain local development gates; observable behavior is checked through the scenarios below.

The scenarios are designed so a coding agent can run commands, evaluate exit status and output, inspect produced artifacts, and report evidence. A scenario may identify a narrow human-observation step when graphics, audio, or physical-controller behavior cannot yet be proven through a machine-readable signal.

## Evidence record

For every executed scenario, record:

- Scenario ID and date.
- Operating system and architecture.
- Relevant tool and application versions.
- Exact commands and inputs.
- Exit code and material output.
- Artifact, log, or state paths inspected.
- Pass, fail, or blocked result with the reason.
- Cleanup performed or state intentionally retained.

Do not report a scenario as passing on an operating system that was simulated or cross-compiled. Native graphics, audio, controller, launcher, and packaging behavior must ultimately be observed on the target operating system.

## Current foundation scenarios

### MT-001: Bootstrap a Windows development host

Run from the repository root in PowerShell:

```powershell
.\scripts\bootstrap.ps1
```

Pass when the command exits with code `0`, the pinned Rust toolchain and required components are available, and the script reports that the environment is ready. Record the reported Rust and Cargo versions.

### MT-002: Build the scaffold on Windows

Run from the repository root in PowerShell:

```powershell
.\scripts\check.ps1
```

Pass when formatting and Clippy complete without warnings, the locked debug build succeeds, and `target/debug/emucontainer.exe` exists.

### MT-003: Bootstrap a Linux development host

Run from the repository root in a POSIX shell on native Linux:

```sh
chmod +x scripts/bootstrap.sh scripts/check.sh
./scripts/bootstrap.sh
```

Pass when the command exits with code `0`, the pinned Rust toolchain and required components are available, and the script reports that the environment is ready. Record the reported Rust and Cargo versions.

### MT-004: Build the scaffold on Linux

Run from the repository root on native Linux:

```sh
./scripts/check.sh
```

Pass when formatting and Clippy complete without warnings, the locked debug build succeeds, and `target/debug/emucontainer` exists.

### MT-005: Validate the checked-in contentless recipe

Run from the repository root:

```text
cargo run --locked -- validate examples/contentless/emucontainer.toml
```

Pass when the command exits with code `0` and prints:

```text
valid: org.emucontainer.smoke-test (EmuContainer Smoke Test, targets: linux-x86_64, windows-x86_64)
```

### MT-006: Reject a missing recipe

Run from the repository root:

```text
cargo run --locked -- validate does-not-exist.toml
```

Pass when the command exits nonzero and reports that `does-not-exist.toml` could not be read. Retain the exact exit code and diagnostic in the evidence record.

## Planned product scenarios

These scenarios define the shape of future manual verification. Add exact commands, expected structured output, logs, and artifact paths as each capability is implemented.

### MT-100: Assemble an unpackaged reference payload

Resolve one platform-neutral recipe into pinned frontend, core, content, configuration, and state-layout components on native Windows and Linux. Verify hashes, record the resolved plan, launch without target-specific commands in the recipe, and confirm that mutable state is outside the payload.

### MT-200: Build and inspect a Linux GameImage

Build through the noninteractive Linux adapter on native Linux. Record the artifact hash and metadata; verify launch, device visibility, fullscreen request, external save persistence, disabled network access, and clean return to the invoking launcher.

### MT-300: Build and inspect a Windows MSIX

Build and development-sign through the noninteractive Windows adapter. Record the package hash and manifest; verify install, direct launch, controller discovery, external save persistence across package replacement, disabled network access, clean exit, and uninstall behavior.

### MT-400: Compare shared gameplay behavior

Run the same recipe's native packages on Windows and Linux. Compare declared runner versions, content hash, launch result, controller mapping, fullscreen behavior, audio status, save changes, shutdown status, and network policy. Capture machine-readable evidence first and label only the remaining visual, audible, or physical-controller observations for a human operator.
