#Requires -Version 5.1
<#
.SYNOPSIS
    Nexus - Windows Quick-Start Script
.DESCRIPTION
    Installs all dependencies (if needed), spins up infrastructure via Docker Desktop,
    builds the server with Cargo, and starts Nexus.
.NOTES
    Requires: Windows 10 2004+ or Windows 11 (for WSL2 / Docker Desktop)
    Run from PowerShell as Administrator for first-time setup.
    Subsequent runs do not need elevation.
#>

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..\..\..") | Select-Object -ExpandProperty Path
$EnvFile  = Join-Path $RepoRoot ".env"

function Write-Info($msg)    { Write-Host "[nexus] $msg" -ForegroundColor Cyan }
function Write-Ok($msg)      { Write-Host "[nexus] $msg" -ForegroundColor Green }
function Write-Warn($msg)    { Write-Host "[nexus] $msg" -ForegroundColor Yellow }
function Write-Err($msg)     { Write-Host "[nexus] ERROR: $msg" -ForegroundColor Red; exit 1 }

# -- Winget helper -------------------------------------------------------------
function Install-Via-Winget($id, $name) {
    Write-Info "Installing $name via winget ..."
    winget install --id $id --silent --accept-package-agreements --accept-source-agreements
}

# -- Check / install Docker Desktop -------------------------------------------
if (-not (Get-Command "docker" -ErrorAction SilentlyContinue)) {
    Write-Warn "Docker not found."
    if (Get-Command "winget" -ErrorAction SilentlyContinue) {
        Install-Via-Winget "Docker.DockerDesktop" "Docker Desktop"
        Write-Warn "Docker Desktop installed. Please restart your machine, then re-run this script."
        exit 0
    } else {
        Write-Err "Docker Desktop is required. Download from https://www.docker.com/products/docker-desktop/"
    }
}

# Wait for Docker daemon
Write-Info "Checking Docker daemon ..."
$attempts = 0
while ($attempts -lt 15) {
    try {
        $oldEap = $ErrorActionPreference
        $ErrorActionPreference = "Continue"
        docker info 1>$null 2>$null
        $ErrorActionPreference = $oldEap

        break
    } catch {
        if ($null -ne $oldEap) { $ErrorActionPreference = $oldEap }

        Write-Host "." -NoNewline
        Start-Sleep -Seconds 3
        $attempts++
    }
}
if ($attempts -eq 15) {
    Write-Info "Starting Docker Desktop ..."
    Start-Process "C:\Program Files\Docker\Docker\Docker Desktop.exe"
    Start-Sleep -Seconds 20

    $oldEap = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    docker info 1>$null 2>$null
    $ErrorActionPreference = $oldEap
}
Write-Ok "Docker is running."

# -- Check / install Rust ------------------------------------------------------
if (-not (Get-Command "cargo" -ErrorAction SilentlyContinue)) {
    Write-Warn "Rust not found."
    if (Get-Command "winget" -ErrorAction SilentlyContinue) {
        Install-Via-Winget "Rustlang.Rustup" "Rust (rustup)"
        # Reload PATH
        $env:PATH = [System.Environment]::GetEnvironmentVariable("PATH","Machine") + ";" + [System.Environment]::GetEnvironmentVariable("PATH","User")
    } else {
        Write-Err "Rust is required. Download rustup from https://rustup.rs/"
    }
}
Write-Info "Rust: $(rustc --version)"

# -- Environment setup ---------------------------------------------------------
if (-not (Test-Path $EnvFile)) {
    Write-Info "Creating .env from .env.example ..."
    Copy-Item (Join-Path $RepoRoot ".env.example") $EnvFile

    # Generate a random JWT secret using .NET
    $bytes  = New-Object byte[] 64
    [System.Security.Cryptography.RandomNumberGenerator]::Create().GetBytes($bytes)
    $secret = -join ($bytes | ForEach-Object { "{0:x2}" -f $_ })

    $content = Get-Content $EnvFile -Raw
    $content = $content -replace "CHANGE_ME_IN_PRODUCTION_use_openssl_rand_hex_64", $secret
    Set-Content $EnvFile $content -NoNewline
    Write-Ok ".env created with auto-generated JWT secret."
}

# -- Start infrastructure ------------------------------------------------------
Write-Info "Starting infrastructure (Postgres, Redis, ScyllaDB, MinIO, MeiliSearch) ..."
Set-Location $RepoRoot
docker compose up -d

Write-Info "Waiting for services to become healthy ..."
Start-Sleep -Seconds 15
$ready = $false
for ($i = 0; $i -lt 20; $i++) {
    $ps = docker compose ps 2>&1
    if ($ps -match "healthy") { $ready = $true; break }
    Start-Sleep -Seconds 3
    Write-Host "." -NoNewline
}
Write-Host ""
if (-not $ready) { Write-Warn "Some services may still be starting - continuing anyway." }

# -- Build ---------------------------------------------------------------------
Write-Info "Building Nexus server (first build: 2-10 min) ..."
cargo build --release --bin nexus

# -- Run -----------------------------------------------------------------------
Write-Ok "Starting Nexus ..."
Write-Host ""
Write-Host "  REST API:   http://localhost:8080"
Write-Host "  Gateway:    ws://localhost:8081"
Write-Host "  Voice:      ws://localhost:8082"
Write-Host "  Health:     http://localhost:8080/api/v1/health"
Write-Host ""
cargo run --release --bin nexus -- serve
