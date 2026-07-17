$ErrorActionPreference = "Stop"

$cargoBin = Join-Path $env:USERPROFILE ".cargo\bin"
if (Test-Path -LiteralPath $cargoBin) {
    $env:PATH = "$cargoBin;$env:PATH"
}

$repositoryRoot = Split-Path -Parent $PSScriptRoot
Push-Location $repositoryRoot

try {
    if (-not (Get-Command rustup -ErrorAction SilentlyContinue)) {
        throw "rustup is required. Install it from https://rustup.rs/ and open a new terminal."
    }

    rustup toolchain install 1.97.1 --profile minimal --component rustfmt --component clippy
    if ($LASTEXITCODE -ne 0) {
        exit $LASTEXITCODE
    }

    cargo fetch --locked
    if ($LASTEXITCODE -ne 0) {
        exit $LASTEXITCODE
    }

    Write-Host "EmuContainer development dependencies are ready."
}
finally {
    Pop-Location
}
