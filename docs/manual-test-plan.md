# Agent-Operable Manual Test Plan

## Purpose and policy

This plan verifies the Windows Runtime+Image v1 proof of concept. Automated test suites, test harnesses, coverage, continuous integration, and GitHub Actions remain intentionally deferred. Formatting, warning-denied linting, compilation, and the manual scenarios below are the release gates.

Commands and machine-readable evidence are agent-operable wherever practical. Graphics, synchronized audio, fullscreen presentation, physical controller input, and recognizing a loaded in-game save are labeled human-observation steps.

Never publish the private ROMs or generated `.emuc` images used here. All fixtures, artifacts, logs copied for evidence, and temporary recipes belong under the ignored `.emucontainer/` tree.

## Evidence record

For every executed scenario, record:

- Scenario ID, date/time, operator, Windows edition/build, architecture, and NTFS volume.
- `rustc --version`, `cargo --version`, and `emucontainer --version` output.
- Exact commands and resolved input paths.
- Exit codes plus relevant stdout/stderr.
- Input and output SHA-256 hashes.
- Artifact, cache, instance, state, volume, and log paths inspected.
- Relevant `events.jsonl` records and process IDs.
- Pass, fail, or blocked result and reason.
- Cleanup performed or state intentionally retained.
- Each human observation, the person making it, and its result.

Do not report a scenario as passing if its required native Windows or human-observation step was not performed.

## PowerShell conventions

Run scenarios from the repository root in native Windows PowerShell. Initialize these variables in each new shell:

```powershell
$ErrorActionPreference = 'Stop'
$Repo = (Resolve-Path -LiteralPath '.').Path
$Cli = Join-Path $Repo 'target\debug\emucontainer.exe'
$FixtureRoot = Join-Path $Repo '.emucontainer\test-inputs'
$RomRoot = Join-Path $FixtureRoot 'roms'
$EmulatorRoot = Join-Path $FixtureRoot 'emulators'
$ArtifactRoot = Join-Path $Repo '.emucontainer\artifacts'
$EvidenceRoot = Join-Path $Repo '.emucontainer\evidence'
$RuntimeRoot = Join-Path $env:LOCALAPPDATA 'EmuContainer'
$InstallRoot = Join-Path $env:LOCALAPPDATA 'Programs\EmuContainer'

$MgbaRoot = Join-Path $EmulatorRoot 'mgba-0.10.5\mGBA-0.10.5-win64'
$MelonDsRoot = Join-Path $EmulatorRoot 'melonds-1.1'
$BlueRom = Join-Path $RomRoot 'gb\pokemon-blue.gb'
$CrystalRom = Join-Path $RomRoot 'gbc\pokemon-crystal.gbc'
$EmeraldRom = Join-Path $RomRoot 'gba\pokemon-emerald.gba'
$RenegadeRom = Join-Path $RomRoot 'nds\pokemon-renegade-platinum.nds'

$BlueImage = Join-Path $ArtifactRoot 'pokemon-blue.emuc'
$CrystalImage = Join-Path $ArtifactRoot 'pokemon-crystal.emuc'
$EmeraldImage = Join-Path $ArtifactRoot 'pokemon-emerald.emuc'
$RenegadeImage = Join-Path $ArtifactRoot 'pokemon-renegade-platinum.emuc'

New-Item -ItemType Directory -Force -Path $FixtureRoot, $ArtifactRoot, $EvidenceRoot | Out-Null
```

After a command whose failure is expected, capture `$LASTEXITCODE` before running another native command.

## Foundation gate

### MT-010: Bootstrap and build on Windows

```powershell
.\scripts\bootstrap.ps1
.\scripts\check.ps1
cargo build --release --locked
rustc --version
cargo --version
& $Cli --version
```

Pass when every build command exits `0`, formatting is unchanged, Clippy reports no warnings, and both `target\debug\emucontainer.exe` and `target\release\emucontainer.exe` exist. Record all reported versions and executable SHA-256 hashes.

### MT-020: Validate public recipes and missing-input diagnostics

```powershell
& $Cli validate (Join-Path $Repo 'examples\mgba-pokemon-blue\emucontainer.toml')
if ($LASTEXITCODE -ne 0) { throw 'mGBA example did not validate' }

& $Cli validate (Join-Path $Repo 'examples\melonds-pokemon-renegade-platinum\emucontainer.toml')
if ($LASTEXITCODE -ne 0) { throw 'melonDS example did not validate' }

& $Cli validate (Join-Path $Repo 'does-not-exist.toml')
$MissingExit = $LASTEXITCODE
if ($MissingExit -eq 0) { throw 'missing recipe unexpectedly validated' }
```

Pass when both schema-v2 examples validate, the missing file exits nonzero with its path in a concise diagnostic, and no obsolete contentless/schema-v1 example remains.

## Input and build scenarios

### MT-030: Review the repository for public sharing

Run the README source-setup commands and MT-020 without private inputs. Recipe validation must work without downloading games or launching an emulator. Confirm the README distinguishes this from building an image, which needs matching private inputs.

```powershell
git diff --check
git ls-files
git log --all --format= --name-only
git check-ignore --no-index public-check.emuc public-check.gb public-check.gbc public-check.gba public-check.nds public-check.sav public-check.srm public-check.state .emucontainer/private-check.txt .env
cargo metadata --no-deps --format-version 1 --locked
```

Pass when the candidate file inventory matches `FILEMAP.md` exactly once per file, MIT license text and Cargo metadata agree, all listed private sample paths are ignored, and tracked files plus available history contain no private payloads or credentials. Review history contents as well as filenames; ignore rules do not remove files from older commits. Record the refs and scan scope, and do not describe a pattern-based credential scan as a guarantee.

Check the README's local links resolve and that its donation link and `.github/FUNDING.yml` both identify `dorely`. After pushing to the default branch, verify GitHub renders the README, detects the MIT license, and displays the Sponsor destination. Record that remote UI check separately if it has not yet been performed. No emulator launch or runtime installation is required for this scenario.

### MT-100: Stage private ROMs without tracking them

Create only ignored destination directories, then copy the four authorized local fixtures:

```powershell
$SourceRootSetting = $env:EMUCONTAINER_ROM_SOURCE_ROOT
if ([string]::IsNullOrWhiteSpace($SourceRootSetting)) {
    throw 'Set EMUCONTAINER_ROM_SOURCE_ROOT to the authorized local ROM source directory'
}
$SourceRoot = (Resolve-Path -LiteralPath $SourceRootSetting).Path
$Copies = @(
    @{ Source = Join-Path $SourceRoot 'gb\Pokemon - Blue Version (USA, Europe) (SGB Enhanced).gb'; Destination = $BlueRom; Size = 1048576; Hash = '2a951313c2640e8c2cb21f25d1db019ae6245d9c7121f754fa61afd7bee6452d' },
    @{ Source = Join-Path $SourceRoot 'gbc\Pokemon - Crystal Version (USA, Europe) (Rev 1).gbc'; Destination = $CrystalRom; Size = 2097152; Hash = 'fdcc3c8c43813cf8731fc037d2a6d191bac75439c34b24ba1c27526e6acdc8a2' },
    @{ Source = Join-Path $SourceRoot 'gba\Pokemon - Emerald Version (USA, Europe).gba'; Destination = $EmeraldRom; Size = 16777216; Hash = 'a9dec84dfe7f62ab2220bafaef7479da0929d066ece16a6885f6226db19085af' },
    @{ Source = Join-Path $SourceRoot 'nds\Pokemon - Renegade Platinum.nds'; Destination = $RenegadeRom; Size = 104923028; Hash = 'f11e52b42a76dcbe968a733d28d8dd4dedf7161ae21fc499e2738c3dc0475bb8' }
)

foreach ($copy in $Copies) {
    if (-not (Test-Path -LiteralPath $copy.Source -PathType Leaf)) { throw "Missing source: $($copy.Source)" }
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $copy.Destination) | Out-Null
    Copy-Item -LiteralPath $copy.Source -Destination $copy.Destination -Force

    $sourceInfo = Get-Item -LiteralPath $copy.Source
    $destinationInfo = Get-Item -LiteralPath $copy.Destination
    $sourceHash = (Get-FileHash -LiteralPath $copy.Source -Algorithm SHA256).Hash.ToLowerInvariant()
    $destinationHash = (Get-FileHash -LiteralPath $copy.Destination -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($sourceInfo.Length -ne $copy.Size -or $destinationInfo.Length -ne $copy.Size) { throw "Size mismatch: $($copy.Destination)" }
    if ($sourceHash -ne $copy.Hash -or $destinationHash -ne $copy.Hash) { throw "Hash mismatch: $($copy.Destination)" }

    git check-ignore -v --no-index -- $copy.Destination
    if ($LASTEXITCODE -ne 0) { throw "Fixture is not ignored: $($copy.Destination)" }
    git ls-files --error-unmatch -- $copy.Destination 2>$null
    if ($LASTEXITCODE -eq 0) { throw "Fixture is tracked: $($copy.Destination)" }
}

$fixtureStatus = git status --short -- .emucontainer
if ($fixtureStatus) { throw "Ignored fixture tree appeared in status: $fixtureStatus" }
```

Pass when every source/destination pair has the expected size and SHA-256, `git check-ignore` identifies the repository's `/.emucontainer/` rule, none is tracked, and status remains clean for the fixture tree. Record the four source/destination hashes and ignore-rule output.

Do not copy the existing DeSmuME directory. It contains personal battery/state/configuration data and is not a clean emulator input.

### MT-110: Intake clean emulator folders

Use official release assets and verify them before extraction. 7-Zip is required for the mGBA archive.

```powershell
$DownloadRoot = Join-Path $FixtureRoot 'archives'
New-Item -ItemType Directory -Force -Path $DownloadRoot, $EmulatorRoot | Out-Null

$MgbaArchive = Join-Path $DownloadRoot 'mGBA-0.10.5-win64.7z'
$MelonDsArchive = Join-Path $DownloadRoot 'melonDS-1.1-windows-x86_64.zip'

Invoke-WebRequest -Uri 'https://github.com/mgba-emu/mgba/releases/download/0.10.5/mGBA-0.10.5-win64.7z' -OutFile $MgbaArchive
Invoke-WebRequest -Uri 'https://github.com/melonDS-emu/melonDS/releases/download/1.1/melonDS-windows-x86_64.zip' -OutFile $MelonDsArchive

$ExpectedArchives = @{
    $MgbaArchive = 'b497a57c7d9093834dadc64f33a90f7c411439c21fdb8a0143255a45ea37563a'
    $MelonDsArchive = '9f3f8a244103be20b5b657af5b0ed1b2a66bb20a7181476a6d294c9a53d4f8c8'
}
foreach ($path in $ExpectedArchives.Keys) {
    $actual = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($actual -ne $ExpectedArchives[$path]) { throw "Archive hash mismatch: $path" }
}

$MgbaExtractRoot = Split-Path -Parent $MgbaRoot
$allowedEmulatorRoot = [IO.Path]::GetFullPath($EmulatorRoot).TrimEnd('\') + '\'
foreach ($destination in @($MgbaExtractRoot, $MelonDsRoot)) {
    $fullDestination = [IO.Path]::GetFullPath($destination)
    if (-not $fullDestination.StartsWith($allowedEmulatorRoot, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Refusing to clean extraction destination outside the fixture emulator root: $fullDestination"
    }
    if (Test-Path -LiteralPath $fullDestination) {
        Remove-Item -LiteralPath $fullDestination -Recurse -Force
    }
}
New-Item -ItemType Directory -Force -Path $MgbaExtractRoot, $MelonDsRoot | Out-Null
& 7z x $MgbaArchive "-o$MgbaExtractRoot" -y
if ($LASTEXITCODE -ne 0) { throw 'mGBA extraction failed' }
Expand-Archive -LiteralPath $MelonDsArchive -DestinationPath $MelonDsRoot -Force

$MgbaExe = Join-Path $MgbaRoot 'mgba-sdl.exe'
$MelonDsExe = Join-Path $MelonDsRoot 'melonDS.exe'
if (-not (Test-Path -LiteralPath $MgbaExe -PathType Leaf)) { throw 'Normalize the mGBA archive so mgba-sdl.exe is at the fixture root' }
if (-not (Test-Path -LiteralPath $MelonDsExe -PathType Leaf)) { throw 'Normalize the melonDS archive so melonDS.exe is at the fixture root' }

$ExpectedExecutables = @{
    $MgbaExe = 'e08d9771033acd9a518175ef528ad3a54b6d2a2e532c7aaf3af54b7ed5af420f'
    $MelonDsExe = '04738277ba1d7ea0b7408608755d746193a68a9bfa628e2759140a9f2d5ab109'
}
foreach ($path in $ExpectedExecutables.Keys) {
    $actual = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($actual -ne $ExpectedExecutables[$path]) { throw "Executable hash mismatch: $path" }
}

$MgbaLicense = Join-Path $MgbaRoot 'LICENSE.txt'
if (-not (Test-Path -LiteralPath $MgbaLicense -PathType Leaf)) { throw 'mGBA LICENSE.txt is missing from the clean input root' }

$MelonDsLicense = Join-Path $MelonDsRoot 'LICENSE'
if (-not (Test-Path -LiteralPath $MelonDsLicense -PathType Leaf)) {
    Invoke-WebRequest -Uri 'https://raw.githubusercontent.com/melonDS-emu/melonDS/1.1/LICENSE' -OutFile $MelonDsLicense
}

$MelonDsPortable = Join-Path $MelonDsRoot 'portable'
New-Item -ItemType Directory -Force -Path $MelonDsPortable | Out-Null
$MelonDsConfig = @'
SaveFilePath = "portable"
SavestatePath = "portable"

[Emu]
DirectBoot = true
ExternalBIOSEnable = false
ConsoleType = 0
'@
$Utf8NoBom = New-Object System.Text.UTF8Encoding -ArgumentList $false
[IO.File]::WriteAllText((Join-Path $MelonDsPortable 'melonDS.toml'), $MelonDsConfig, $Utf8NoBom)

Get-ChildItem -LiteralPath $MgbaRoot -Recurse -Force | Select-Object FullName, Length
Get-ChildItem -LiteralPath $MelonDsRoot -Recurse -Force | Select-Object FullName, Length
```

Pass when archives and executables match their expected hashes, each entrypoint is at its declared root, required DLL/resources/notices are retained, `portable.ini` is present for mGBA, and melonDS has the seeded `portable\melonDS.toml`. Record recursive input inventories and hashes. No BIOS is needed; do not copy the unrelated PSX `scph1001.bin`.

### MT-120: Build four deterministic images

Build Blue and Renegade Platinum from recipes:

```powershell
New-Item -ItemType Directory -Force -Path $ArtifactRoot | Out-Null
$InitialOutputs = @($BlueImage, $CrystalImage, $EmeraldImage, $RenegadeImage, (Join-Path $ArtifactRoot 'pokemon-blue-rebuild.emuc'))
$ExistingOutputs = $InitialOutputs | Where-Object { Test-Path -LiteralPath $_ }
if ($ExistingOutputs) {
    throw "Move or remove only these prior ignored test artifacts before a clean MT-120 run: $($ExistingOutputs -join ', ')"
}

& $Cli build --recipe (Join-Path $Repo 'examples\mgba-pokemon-blue\emucontainer.toml') --emulator-root $MgbaRoot --rom $BlueRom --output $BlueImage
if ($LASTEXITCODE -ne 0) { throw 'Blue build failed' }

& $Cli build --recipe (Join-Path $Repo 'examples\melonds-pokemon-renegade-platinum\emucontainer.toml') --emulator-root $MelonDsRoot --rom $RenegadeRom --output $RenegadeImage
if ($LASTEXITCODE -ne 0) { throw 'Renegade Platinum build failed' }
```

Exercise the all-flags form with Crystal and Emerald:

```powershell
$MgbaSource = 'https://github.com/mgba-emu/mgba/releases/tag/0.10.5'
$CommonMgba = @(
    '--target', 'windows-x86_64',
    '--emulator-id', 'mgba',
    '--emulator-version', '0.10.5',
    '--emulator-source', $MgbaSource,
    '--emulator-root', $MgbaRoot,
    '--entrypoint', 'mgba-sdl.exe',
    '--working-directory', '.',
    '--arg=-f', '--arg=-C', '--arg=savegamePath={state}/saves',
    '--arg=-C', '--arg=savestatePath={state}/states', '--arg={rom}',
    '--notice', 'LICENSE.txt=mgba-LICENSE.txt'
)

& $Cli build --id local.pokemon-crystal --title 'Pokemon Crystal' --system gbc @CommonMgba --rom $CrystalRom --rom-name pokemon-crystal.gbc --rom-sha256 fdcc3c8c43813cf8731fc037d2a6d191bac75439c34b24ba1c27526e6acdc8a2 --output $CrystalImage
if ($LASTEXITCODE -ne 0) { throw 'Crystal build failed' }

& $Cli build --id local.pokemon-emerald --title 'Pokemon Emerald' --system gba @CommonMgba --rom $EmeraldRom --rom-name pokemon-emerald.gba --rom-sha256 a9dec84dfe7f62ab2220bafaef7479da0929d066ece16a6885f6226db19085af --output $EmeraldImage
if ($LASTEXITCODE -ne 0) { throw 'Emerald build failed' }
```

Prove deterministic output and overwrite protection:

```powershell
$BlueRebuild = Join-Path $ArtifactRoot 'pokemon-blue-rebuild.emuc'
& $Cli build --recipe (Join-Path $Repo 'examples\mgba-pokemon-blue\emucontainer.toml') --emulator-root $MgbaRoot --rom $BlueRom --output $BlueRebuild
if ($LASTEXITCODE -ne 0) { throw 'determinism rebuild failed' }

$blueHash = (Get-FileHash -LiteralPath $BlueImage -Algorithm SHA256).Hash
$rebuildHash = (Get-FileHash -LiteralPath $BlueRebuild -Algorithm SHA256).Hash
if ($blueHash -ne $rebuildHash) { throw 'identical inputs did not produce identical images' }

& $Cli build --recipe (Join-Path $Repo 'examples\mgba-pokemon-blue\emucontainer.toml') --emulator-root $MgbaRoot --rom $BlueRom --output $BlueImage
$OverwriteExit = $LASTEXITCODE
if ($OverwriteExit -eq 0) { throw 'existing output was overwritten without --force' }

& $Cli build --recipe (Join-Path $Repo 'examples\mgba-pokemon-blue\emucontainer.toml') --emulator-root $MgbaRoot --rom $BlueRom --output $BlueImage --force
if ($LASTEXITCODE -ne 0) { throw '--force rebuild failed' }
if ((Get-FileHash -LiteralPath $BlueImage -Algorithm SHA256).Hash -ne $blueHash) { throw '--force changed deterministic output' }
```

Pass when all four images exist, recipe and all-flags modes succeed, identical Blue inputs produce identical outer hashes, overwrite without `--force` fails without changing the prior artifact, and `--force` atomically reproduces it. Record image sizes and SHA-256 hashes.

### MT-130: Inspect images and reject invalid inputs

Verify every image and retain JSON output:

```powershell
$Images = @($BlueImage, $CrystalImage, $EmeraldImage, $RenegadeImage)
foreach ($image in $Images) {
    $name = [IO.Path]::GetFileNameWithoutExtension($image)
    $jsonPath = Join-Path $EvidenceRoot "$name-inspect.json"
    & $Cli inspect $image --json | Set-Content -LiteralPath $jsonPath -Encoding UTF8
    if ($LASTEXITCODE -ne 0) { throw "Inspection failed: $image" }
    Get-Content -LiteralPath $jsonPath -Raw | ConvertFrom-Json | Out-Null
}
```

Inspect the JSON records and confirm target, identities, ROM path/size/hash, launch vector, notices, mounts, `isolation = "none"`, `network_enforced = false`, and complete sorted inventory. `image.json` itself is structurally verified but is not in its own inventory because a self-hash is impossible.

Reject a wrong declared ROM hash:

```powershell
$BadRecipe = Join-Path $FixtureRoot 'bad-hash.toml'
(Get-Content -LiteralPath (Join-Path $Repo 'examples\mgba-pokemon-blue\emucontainer.toml') -Raw) `
    -replace '2a951313c2640e8c2cb21f25d1db019ae6245d9c7121f754fa61afd7bee6452d', ('0' * 64) |
    Set-Content -LiteralPath $BadRecipe -Encoding UTF8
& $Cli build --recipe $BadRecipe --emulator-root $MgbaRoot --rom $BlueRom --output (Join-Path $ArtifactRoot 'bad-hash.emuc')
$BadHashExit = $LASTEXITCODE
if ($BadHashExit -eq 0) { throw 'wrong ROM hash unexpectedly built' }
```

Reject a one-byte-corrupted image:

```powershell
$Tampered = Join-Path $ArtifactRoot 'pokemon-blue-tampered.emuc'
Copy-Item -LiteralPath $BlueImage -Destination $Tampered -Force
$bytes = [IO.File]::ReadAllBytes($Tampered)
$offset = [Math]::Floor($bytes.Length / 2)
$bytes[$offset] = $bytes[$offset] -bxor 1
[IO.File]::WriteAllBytes($Tampered, $bytes)
& $Cli inspect $Tampered --json
$TamperExit = $LASTEXITCODE
if ($TamperExit -eq 0) { throw 'tampered image unexpectedly verified' }

$Appended = Join-Path $ArtifactRoot 'pokemon-blue-appended.emuc'
Copy-Item -LiteralPath $BlueImage -Destination $Appended -Force
$stream = [IO.File]::Open($Appended, [IO.FileMode]::Append, [IO.FileAccess]::Write, [IO.FileShare]::None)
try { $stream.WriteByte(0) } finally { $stream.Dispose() }
& $Cli inspect $Appended --json
$AppendedExit = $LASTEXITCODE
if ($AppendedExit -eq 0) { throw 'image with undeclared trailing data unexpectedly verified' }
```

Search retained inspection JSON for private host roots:

```powershell
$privateRoots = @($Repo, $FixtureRoot)
if ($env:EMUCONTAINER_ROM_SOURCE_ROOT) {
    $privateRoots += [IO.Path]::GetFullPath($env:EMUCONTAINER_ROM_SOURCE_ROOT)
}
foreach ($jsonPath in Get-ChildItem -LiteralPath $EvidenceRoot -Filter '*-inspect.json') {
    $manifestJson = ((Get-Content -LiteralPath $jsonPath.FullName -Raw | ConvertFrom-Json).manifest | ConvertTo-Json -Depth 100 -Compress)
    foreach ($root in $privateRoots) {
        if ($manifestJson.IndexOf($root, [StringComparison]::OrdinalIgnoreCase) -ge 0) {
            throw "Host source path leaked into image metadata: $root"
        }
    }
}
```

Pass when all original images fully verify, the JSON is parseable and complete, wrong-hash, modified-entry, and trailing-data artifacts exit nonzero with actionable diagnostics, no emulator process starts, and no private source path appears in the manifest.

## Runtime scenarios

### MT-140: Install and launch directly

**Interactive/human-observation scenario:** the launch commands below open emulator windows. Never run them unattended or as a headless agent check; stop after installation/status unless a human explicitly takes over the launch portion.

Install and verify the user-scoped runtime:

```powershell
& $Cli runtime install
if ($LASTEXITCODE -ne 0) { throw 'runtime install failed' }
& $Cli runtime status --json | Set-Content -LiteralPath (Join-Path $EvidenceRoot 'runtime-status.json') -Encoding UTF8
if ($LASTEXITCODE -ne 0) { throw 'runtime status failed' }

$association = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey('Software\Classes\EmuContainer.Image\shell\open\command').GetValue('')
$extension = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey('Software\Classes\.emuc').GetValue('')
$association
$extension
```

Run each image from the CLI. A human exits the emulator normally after confirming the expected game opens directly:

```powershell
foreach ($image in @($BlueImage, $CrystalImage, $EmeraldImage, $RenegadeImage)) {
    & $Cli run $image --json
    if ($LASTEXITCODE -ne 0) { throw "Launch failed: $image" }
}
```

Launch Blue once through the registered file association, then exit it normally:

```powershell
Start-Process -FilePath $BlueImage
```

After every exit, locate the newest run log:

```powershell
Get-ChildItem -LiteralPath (Join-Path $RuntimeRoot 'games') -Filter events.jsonl -Recurse |
    Sort-Object LastWriteTimeUtc -Descending |
    Select-Object -First 5 FullName, LastWriteTimeUtc
```

Pass when install and status agree on the installed executable/hash and owned HKCU association; every image verifies and launches directly; logs record image verification, cache/instance readiness, mounts, expanded executable/arguments, PID, root exit code, final status, and zero remaining descendants. Human-observe correct game, fullscreen, graphics, and audio for each image.

### MT-150: Prove the image is self-contained

**Interactive/human-observation scenario:** cold-launching these images opens emulator windows. Never run the launch loop unattended.

Move the only four `.emuc` files into a separate ignored transport directory, make all build inputs unavailable, prune disposable runtime data, and cold-launch:

```powershell
$TransportRoot = Join-Path $Repo '.emucontainer\transport-only'
$OfflineRoot = Join-Path $Repo '.emucontainer\test-inputs.offline'
New-Item -ItemType Directory -Force -Path $TransportRoot | Out-Null
foreach ($image in @($BlueImage, $CrystalImage, $EmeraldImage, $RenegadeImage)) {
    Copy-Item -LiteralPath $image -Destination $TransportRoot -Force
}

Move-Item -LiteralPath $FixtureRoot -Destination $OfflineRoot
try {
    & $Cli runtime prune --json
    if ($LASTEXITCODE -ne 0) { throw 'runtime prune failed' }
    foreach ($image in Get-ChildItem -LiteralPath $TransportRoot -Filter '*.emuc') {
        & $Cli run $image.FullName --json
        if ($LASTEXITCODE -ne 0) { throw "Cold launch failed: $($image.FullName)" }
    }
}
finally {
    Move-Item -LiteralPath $OfflineRoot -Destination $FixtureRoot
}
```

Pass when all four transport copies launch after cache/instance pruning while the emulator and ROM source tree is unavailable. Record transport hashes, prune output, rebuilt cache/instance paths, logs, and confirmation that no path under the offline fixture tree was accessed.

### MT-160: Verify gameplay and persistent state

**Interactive/human-observation scenario:** every launch in this scenario requires a human to operate and close the emulator. Never run it unattended.

For each game, run its image and perform these narrow human steps:

1. Confirm direct fullscreen startup and expected game identity.
2. Confirm video remains responsive and audio is synchronized.
3. Create a new in-game battery save.
4. Create a save state using Shift+F1.
5. Exit normally.

For Emerald additionally connect an XInput controller before launch and confirm directional, face-button, Start, and Select input.

Record state artifacts before pruning:

```powershell
$GameIds = @('local.pokemon-blue', 'local.pokemon-crystal', 'local.pokemon-emerald', 'local.pokemon-renegade-platinum')
$BeforeState = foreach ($id in $GameIds) {
    $gameRoot = Join-Path $RuntimeRoot "games\$id"
    Get-ChildItem -LiteralPath $gameRoot -Recurse -File | Where-Object {
        $_.FullName -match '\\state\\|\\volumes\\'
    } | ForEach-Object {
        [pscustomobject]@{
            Game = $id
            Path = $_.FullName
            Length = $_.Length
            SHA256 = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash
        }
    }
}
$BeforeState | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath (Join-Path $EvidenceRoot 'state-before-prune.json') -Encoding UTF8
if (($BeforeState | Group-Object Game).Count -ne 4) { throw 'one or more games produced no persistent state files' }

& $Cli runtime prune --json
if ($LASTEXITCODE -ne 0) { throw 'runtime prune failed' }

$AfterState = foreach ($entry in $BeforeState) {
    if (-not (Test-Path -LiteralPath $entry.Path -PathType Leaf)) { throw "State disappeared: $($entry.Path)" }
    [pscustomobject]@{
        Game = $entry.Game
        Path = $entry.Path
        Length = (Get-Item -LiteralPath $entry.Path).Length
        SHA256 = (Get-FileHash -LiteralPath $entry.Path -Algorithm SHA256).Hash
    }
}
$AfterState | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath (Join-Path $EvidenceRoot 'state-after-prune.json') -Encoding UTF8
for ($index = 0; $index -lt $BeforeState.Count; $index++) {
    if ($BeforeState[$index].Length -ne $AfterState[$index].Length -or
        $BeforeState[$index].SHA256 -ne $AfterState[$index].SHA256) {
        throw "Prune changed persistent state: $($BeforeState[$index].Path)"
    }
}
```

Relaunch all four images and human-confirm that the battery save and Shift+F1 state load. Pass when hashes remain unchanged across prune until the games intentionally modify them, all state reloads, each game's files remain under only its own ID, and Emerald accepts the XInput controller.

### MT-170: Verify identity guards, compatible updates, isolation, and locking

**Interactive/human-observation scenario:** the compatible-update and concurrency checks open an emulator. Never run them unattended; the rejection-only commands may be run headlessly after the identity already exists.

Create ignored derivative recipes from the Blue example:

```powershell
$BlueRecipe = Get-Content -LiteralPath (Join-Path $Repo 'examples\mgba-pokemon-blue\emucontainer.toml') -Raw
$UpdatedRecipe = Join-Path $FixtureRoot 'blue-emulator-update.toml'
$ChangedRomRecipe = Join-Path $FixtureRoot 'blue-changed-rom.toml'
$ChangedEmulatorRecipe = Join-Path $FixtureRoot 'blue-changed-emulator.toml'

($BlueRecipe -replace '(?m)^version = "0\.10\.5"$', 'version = "0.10.5-poc"') |
    Set-Content -LiteralPath $UpdatedRecipe -Encoding UTF8

($BlueRecipe `
    -replace '(?m)^system = "gb"$', 'system = "gbc"' `
    -replace '(?m)^filename = "pokemon-blue\.gb"$', 'filename = "pokemon-crystal.gbc"' `
    -replace '2a951313c2640e8c2cb21f25d1db019ae6245d9c7121f754fa61afd7bee6452d', 'fdcc3c8c43813cf8731fc037d2a6d191bac75439c34b24ba1c27526e6acdc8a2') |
    Set-Content -LiteralPath $ChangedRomRecipe -Encoding UTF8

($BlueRecipe -replace '(?m)^id = "mgba"$', 'id = "mgba-alias"') |
    Set-Content -LiteralPath $ChangedEmulatorRecipe -Encoding UTF8
```

Build the three valid images:

```powershell
$UpdatedImage = Join-Path $ArtifactRoot 'pokemon-blue-emulator-update.emuc'
$ChangedRomImage = Join-Path $ArtifactRoot 'pokemon-blue-changed-rom.emuc'
$ChangedEmulatorImage = Join-Path $ArtifactRoot 'pokemon-blue-changed-emulator.emuc'

& $Cli build --recipe $UpdatedRecipe --emulator-root $MgbaRoot --rom $BlueRom --output $UpdatedImage --force
if ($LASTEXITCODE -ne 0) { throw 'compatible update image build failed' }
& $Cli build --recipe $ChangedRomRecipe --emulator-root $MgbaRoot --rom $CrystalRom --output $ChangedRomImage --force
if ($LASTEXITCODE -ne 0) { throw 'changed-ROM image build failed' }
& $Cli build --recipe $ChangedEmulatorRecipe --emulator-root $MgbaRoot --rom $BlueRom --output $ChangedEmulatorImage --force
if ($LASTEXITCODE -ne 0) { throw 'changed-emulator image build failed' }
```

Run the compatible update and human-confirm the existing Blue save loads. Then require both identity violations to fail before emulator launch:

```powershell
$OtherGameIds = @('local.pokemon-crystal', 'local.pokemon-emerald', 'local.pokemon-renegade-platinum')
$OtherStateBefore = @{}
foreach ($id in $OtherGameIds) {
    Get-ChildItem -LiteralPath (Join-Path $RuntimeRoot "games\$id") -Recurse -File | Where-Object {
        $_.FullName -match '\\state\\|\\volumes\\'
    } | ForEach-Object {
        $OtherStateBefore[$_.FullName] = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash
    }
}

& $Cli run $UpdatedImage --json
if ($LASTEXITCODE -ne 0) { throw 'compatible emulator-version update was rejected' }

& $Cli run $ChangedRomImage --json
$ChangedRomExit = $LASTEXITCODE
if ($ChangedRomExit -eq 0) { throw 'changed ROM reused the existing game identity' }

& $Cli run $ChangedEmulatorImage --json
$ChangedEmulatorExit = $LASTEXITCODE
if ($ChangedEmulatorExit -eq 0) { throw 'changed emulator ID reused the existing game identity' }
```

While Blue is running, attempt a second Blue launch:

```powershell
$FirstStdout = Join-Path $EvidenceRoot 'concurrent-first.stdout.log'
$FirstStderr = Join-Path $EvidenceRoot 'concurrent-first.stderr.log'
$FirstRun = Start-Process -FilePath $Cli -ArgumentList @('run', $BlueImage) -WindowStyle Hidden -RedirectStandardOutput $FirstStdout -RedirectStandardError $FirstStderr -PassThru
Start-Sleep -Seconds 3
& $Cli run $BlueImage --json
$ConcurrentExit = $LASTEXITCODE
if ($ConcurrentExit -eq 0) { throw 'concurrent launch unexpectedly succeeded' }
# Exit the first emulator normally, then:
$FirstRun.WaitForExit()

foreach ($entry in $OtherStateBefore.GetEnumerator()) {
    if (-not (Test-Path -LiteralPath $entry.Key -PathType Leaf)) { throw "Other game state disappeared: $($entry.Key)" }
    $afterHash = (Get-FileHash -LiteralPath $entry.Key -Algorithm SHA256).Hash
    if ($afterHash -ne $entry.Value) { throw "Blue checks changed another game's state: $($entry.Key)" }
}
```

Compare the other three games' pre/post state hashes. Pass when the compatible update reuses Blue state, different ROM/system and emulator ID fail before mounting or starting a process, the concurrent launch fails immediately, and no other game's state changes.

### MT-180: Repair cache and preserve state through lifecycle operations

**Interactive/human-observation scenario:** cache creation, repair, and the final save check launch Blue. Never run those launch commands unattended. The prune/install/status/uninstall portions are headless.

Corrupt one disposable cached file for Blue and require automatic reconstruction:

```powershell
$BlueDigest = (Get-FileHash -LiteralPath $BlueImage -Algorithm SHA256).Hash.ToLowerInvariant()
$BlueCache = Join-Path $RuntimeRoot "cache\images\$BlueDigest"
if (-not (Test-Path -LiteralPath $BlueCache -PathType Container)) {
    & $Cli run $BlueImage --json
    if ($LASTEXITCODE -ne 0) { throw 'could not create Blue cache' }
}
$CacheVictim = Get-ChildItem -LiteralPath (Join-Path $BlueCache 'rootfs') -Recurse -File | Where-Object Length -gt 0 | Select-Object -First 1
if (-not $CacheVictim) { throw 'cache contains no nonempty file to corrupt' }
$OriginalCacheHash = (Get-FileHash -LiteralPath $CacheVictim.FullName -Algorithm SHA256).Hash
$cacheBytes = [IO.File]::ReadAllBytes($CacheVictim.FullName)
$cacheBytes[0] = $cacheBytes[0] -bxor 1
[IO.File]::WriteAllBytes($CacheVictim.FullName, $cacheBytes)

$RepairEvents = Join-Path $EvidenceRoot 'cache-repair-events.jsonl'
& $Cli run $BlueImage --json | Tee-Object -FilePath $RepairEvents
if ($LASTEXITCODE -ne 0) { throw 'runtime did not repair disposable cache' }
if ((Get-FileHash -LiteralPath $CacheVictim.FullName -Algorithm SHA256).Hash -ne $OriginalCacheHash) {
    throw 'repaired cache file does not match its original verified hash'
}
if (-not (Select-String -LiteralPath $RepairEvents -SimpleMatch '"disposition":"repaired"')) {
    throw 'runtime did not record a repaired cache disposition'
}
```

Snapshot durable files, prune, and prove only disposable roots changed:

```powershell
$DurableBefore = Get-ChildItem -LiteralPath (Join-Path $RuntimeRoot 'games') -Recurse -File | ForEach-Object {
    [pscustomobject]@{ Path = $_.FullName; Hash = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash }
}
& $Cli runtime prune --json
if ($LASTEXITCODE -ne 0) { throw 'runtime prune failed' }
foreach ($entry in $DurableBefore) {
    if (-not (Test-Path -LiteralPath $entry.Path -PathType Leaf)) { throw "prune deleted durable file: $($entry.Path)" }
    if ((Get-FileHash -LiteralPath $entry.Path -Algorithm SHA256).Hash -ne $entry.Hash) { throw "prune changed durable file: $($entry.Path)" }
}
```

Uninstall, verify association/runtime removal and durable-data retention, then reinstall:

```powershell
$InstalledCli = Join-Path $InstallRoot 'emucontainer.exe'
if (-not (Test-Path -LiteralPath $InstalledCli -PathType Leaf)) {
    & $Cli runtime install
    if ($LASTEXITCODE -ne 0) { throw 'runtime install prerequisite failed' }
}

& $InstalledCli runtime uninstall
if ($LASTEXITCODE -ne 0) { throw 'runtime uninstall failed' }
for ($attempt = 0; $attempt -lt 100; $attempt++) {
    $extensionKey = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey('Software\Classes\.emuc')
    $programKey = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey('Software\Classes\EmuContainer.Image')
    if (-not (Test-Path -LiteralPath $InstalledCli) -and $null -eq $extensionKey -and $null -eq $programKey) { break }
    if ($extensionKey) { $extensionKey.Dispose() }
    if ($programKey) { $programKey.Dispose() }
    Start-Sleep -Milliseconds 100
}
if (Test-Path -LiteralPath $InstalledCli) { throw 'installed runtime remains after bounded uninstall wait' }
if ([Microsoft.Win32.Registry]::CurrentUser.OpenSubKey('Software\Classes\.emuc')) { throw 'owned .emuc association remains after uninstall' }
if ([Microsoft.Win32.Registry]::CurrentUser.OpenSubKey('Software\Classes\EmuContainer.Image')) { throw 'owned ProgID remains after uninstall' }
if (-not (Test-Path -LiteralPath (Join-Path $RuntimeRoot 'games') -PathType Container)) { throw 'uninstall deleted durable game data' }

& $Cli runtime install
if ($LASTEXITCODE -ne 0) { throw 'runtime reinstall failed' }
& $Cli runtime status --json
if ($LASTEXITCODE -ne 0) { throw 'reinstalled runtime status failed' }
& $Cli run $BlueImage --json
if ($LASTEXITCODE -ne 0) { throw 'reinstalled runtime could not launch Blue' }
```

Human-confirm the prior Blue save loads. Pass when corruption is detected and rebuilt from the image, cache/instance pruning preserves all durable files byte-for-byte, uninstall removes only the owned binary/association, reinstall restores status, and prior state remains playable. Record repair events, prune output, registry state, and before/after durable hashes.

## Final handoff checklist

Before declaring the v1 POC complete:

```powershell
.\scripts\check.ps1
cargo build --release --locked
git status --short
git ls-files
```

- Record which MT scenarios ran and link their ignored evidence files by absolute path.
- List blocked human-observation steps separately from machine-verifiable results.
- Confirm no ROM, emulator binary, `.emuc`, save, log, evidence output, or credential is tracked.
- Compare `git ls-files` against `FILEMAP.md`: every tracked file appears exactly once and no nonexistent path remains.
- Report release executable and four image hashes, plus installed runtime/status information.
