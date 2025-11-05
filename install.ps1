#!/usr/bin/env pwsh
# Lemma installer script for Windows
# Based on rustup-init.ps1

param(
    [string]$LemmaHome = "$env:USERPROFILE\.lemma"
)

$ErrorActionPreference = 'Stop'

$LemmaBinDir = Join-Path $LemmaHome "bin"
$BaseUrl = "https://lemma.puqing.work"

function Get-Platform {
    $arch = $env:PROCESSOR_ARCHITECTURE

    switch ($arch) {
        "AMD64" { return "x86_64-pc-windows-gnu" }
        "x86_64" { return "x86_64-pc-windows-gnu" }
        default {
            Write-Error "Unsupported architecture: $arch"
            exit 1
        }
    }
}

function Get-LatestVersion {
    param([string]$ManifestUrl)

    Write-Host "   Checking latest version..." -ForegroundColor Gray

    try {
        $manifest = Invoke-RestMethod -Uri $ManifestUrl -UseBasicParsing

        # Parse TOML manually (simple version field)
        $versionLine = $manifest -split "`n" | Where-Object { $_ -match '^version\s*=\s*"([^"]+)"' }
        if ($versionLine -match '"([^"]+)"') {
            return $matches[1]
        }

        throw "Failed to parse version from manifest"
    }
    catch {
        Write-Error "Failed to fetch version manifest: $_"
        exit 1
    }
}

function Install-Lemma {
    Write-Host ""
    Write-Host "=> Installing lemma..." -ForegroundColor Cyan
    Write-Host ""

    # Detect platform
    $platform = Get-Platform
    Write-Host "   Platform: $platform" -ForegroundColor Gray

    # Fetch latest version
    $manifestUrl = "$BaseUrl/manifests/stable.toml"
    $version = Get-LatestVersion -ManifestUrl $manifestUrl
    Write-Host "   Version: $version" -ForegroundColor Gray

    # Construct download URL
    $archiveName = "lemma-$platform.zip"
    $downloadUrl = "$BaseUrl/releases/v$version/$archiveName"
    Write-Host "   Download URL: $downloadUrl" -ForegroundColor Gray
    Write-Host ""

    # Create temp directory
    $tempDir = Join-Path $env:TEMP "lemma-install-$(New-Guid)"
    New-Item -ItemType Directory -Path $tempDir -Force | Out-Null

    try {
        # Download archive
        Write-Host "=> Downloading lemma..." -ForegroundColor Cyan
        $archivePath = Join-Path $tempDir $archiveName

        try {
            Invoke-WebRequest -Uri $downloadUrl -OutFile $archivePath -UseBasicParsing
        }
        catch {
            Write-Error "Failed to download lemma: $_"
            exit 1
        }

        # Extract archive
        Write-Host "=> Extracting..." -ForegroundColor Cyan
        $extractDir = Join-Path $tempDir "extracted"
        Expand-Archive -Path $archivePath -DestinationPath $extractDir -Force

        # Install binary
        Write-Host "=> Installing to $LemmaBinDir..." -ForegroundColor Cyan
        New-Item -ItemType Directory -Path $LemmaBinDir -Force | Out-Null

        $sourceExe = Join-Path $extractDir "lemma.exe"
        $destExe = Join-Path $LemmaBinDir "lemma.exe"

        # Remove old binary if it exists
        if (Test-Path $destExe) {
            Remove-Item $destExe -Force -ErrorAction SilentlyContinue
        }

        Copy-Item -Path $sourceExe -Destination $destExe -Force

        # Create proxy binaries
        Write-Host "=> Creating proxy binaries..." -ForegroundColor Cyan
        foreach ($binary in @("lean", "lake", "leanc")) {
            $proxyPath = Join-Path $LemmaBinDir "$binary.exe"
            if (Test-Path $proxyPath) {
                Remove-Item $proxyPath -Force -ErrorAction SilentlyContinue
            }
            Copy-Item -Path $destExe -Destination $proxyPath -Force
        }

        Write-Host ""
        Write-Host "✓ lemma installed successfully!" -ForegroundColor Green
        Write-Host ""

        # Check if lemma is in PATH
        $pathParts = $env:PATH -split ';'
        $inPath = $pathParts -contains $LemmaBinDir

        if (-not $inPath) {
            Write-Host "To get started, add lemma to your PATH:" -ForegroundColor Yellow
            Write-Host ""
            Write-Host "  Run the following command in PowerShell (Admin):" -ForegroundColor White
            Write-Host "  [System.Environment]::SetEnvironmentVariable('Path', `$env:Path + ';$LemmaBinDir', 'User')" -ForegroundColor Cyan
            Write-Host ""
            Write-Host "  Or add it manually via:" -ForegroundColor White
            Write-Host "  - System Properties > Environment Variables > User Variables > Path" -ForegroundColor Gray
            Write-Host "  - Add: $LemmaBinDir" -ForegroundColor Gray
            Write-Host ""
            Write-Host "After updating PATH, restart your terminal." -ForegroundColor Yellow
        }
        else {
            Write-Host "lemma is already in your PATH." -ForegroundColor Green
        }

        Write-Host ""
        Write-Host "Then install a Lean toolchain:" -ForegroundColor White
        Write-Host ""
        Write-Host "  lemma toolchain install stable" -ForegroundColor Cyan
        Write-Host ""
    }
    finally {
        # Clean up temp directory
        if (Test-Path $tempDir) {
            Remove-Item -Path $tempDir -Recurse -Force -ErrorAction SilentlyContinue
        }
    }
}

# Main execution
try {
    Install-Lemma
}
catch {
    Write-Error "Installation failed: $_"
    exit 1
}
