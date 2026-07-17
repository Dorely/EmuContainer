$ErrorActionPreference = "Stop"

$cargoBin = Join-Path $env:USERPROFILE ".cargo\bin"
if (Test-Path -LiteralPath $cargoBin) {
    $env:PATH = "$cargoBin;$env:PATH"
}

function Invoke-CargoStep {
    param(
        [Parameter(Mandatory = $true)]
        [string[]] $CargoArguments
    )

    cargo @CargoArguments
    if ($LASTEXITCODE -ne 0) {
        exit $LASTEXITCODE
    }
}

$repositoryRoot = Split-Path -Parent $PSScriptRoot
Push-Location $repositoryRoot

try {
    Invoke-CargoStep -CargoArguments @("fmt", "--all", "--check")
    Invoke-CargoStep -CargoArguments @(
        "clippy",
        "--all-targets",
        "--all-features",
        "--locked",
        "--",
        "-D",
        "warnings"
    )
    Invoke-CargoStep -CargoArguments @("build", "--locked")

    Write-Host "EmuContainer formatting, linting, and local build checks passed."
}
finally {
    Pop-Location
}
