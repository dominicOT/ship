# PowerShell script to install ship for Windows
# Usage: irm https://raw.githubusercontent.com/dominicOT/ship/master/scripts/install.ps1 | iex
# or:    powershell -ExecutionPolicy Bypass -Command "& { iwr https://raw.githubusercontent.com/dominicOT/ship/master/scripts/install.ps1 | iex }"

param(
    [string]$InstallDir = ""
)

$ErrorActionPreference = "Stop"

# Ensure TLS 1.2 is enabled
[Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12

$Repo = "dominicOT/ship"
$Prog = "ship"
$DownloadBase = "https://github.com/$Repo/releases/latest/download"

Write-Host "Installing $Prog for Windows`n" -ForegroundColor Cyan

# Detect architecture (checking 64-bit architecture even in 32-bit PowerShell process)
$rawArch = if ($env:PROCESSOR_ARCHITEW6432) { $env:PROCESSOR_ARCHITEW6432 } else { $env:PROCESSOR_ARCHITECTURE }
$arch = ""
switch ($rawArch) {
    "AMD64" { $arch = "x86_64"; break }
    "ARM64" { $arch = "aarch64"; break }
    default {
        Write-Error "Unsupported architecture: $rawArch. Supported: x86_64, aarch64"
        return
    }
}

if (-not $InstallDir) {
    if ($env:LOCALAPPDATA) {
        $InstallDir = Join-Path $env:LOCALAPPDATA "ship\bin"
    } elseif ($env:USERPROFILE) {
        $InstallDir = Join-Path $env:USERPROFILE ".ship\bin"
    } else {
        $InstallDir = "C:\ship\bin"
    }
}

$AssetName = "$Prog-windows-$arch.zip"
$AssetUrl = "$DownloadBase/$AssetName"

Write-Host "Detected architecture: $arch"
Write-Host "Downloading from: $AssetUrl`n"

# Create a unique temporary directory
$TmpDir = Join-Path ([System.IO.Path]::GetTempPath()) ([System.Guid]::NewGuid().ToString())
New-Item -ItemType Directory -Path $TmpDir -Force | Out-Null

$AssetPath = Join-Path $TmpDir $AssetName

try {
    # Download the release asset
    Write-Host "Downloading $AssetName..."
    $ProgressPreference = "SilentlyContinue"
    Invoke-WebRequest -Uri $AssetUrl -OutFile $AssetPath -UseBasicParsing -ErrorAction Stop
    $ProgressPreference = "Continue"
    Write-Host "✓ Downloaded" -ForegroundColor Green

    # Create install directory if it doesn't exist
    if (-not (Test-Path $InstallDir)) {
        Write-Host "Creating directory: $InstallDir"
        New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
    }

    # Extract the zip file
    Write-Host "Extracting $Prog.exe..."
    Expand-Archive -Path $AssetPath -DestinationPath $TmpDir -Force
    $BinarySource = Join-Path $TmpDir "$Prog.exe"

    if (-not (Test-Path $BinarySource)) {
        Write-Error "Downloaded archive does not contain $Prog.exe"
        return
    }

    # Copy to install directory
    $InstallPath = Join-Path $InstallDir "$Prog.exe"
    Copy-Item -Path $BinarySource -Destination $InstallPath -Force
    Write-Host "✓ Installed to: $InstallPath" -ForegroundColor Green

    # Check if install directory is in User PATH
    $PathEnv = [Environment]::GetEnvironmentVariable("PATH", "User")
    $CurrentPathParts = if ($PathEnv) { $PathEnv -split ";" } else { @() }

    if ($CurrentPathParts -contains $InstallDir) {
        Write-Host "✓ $InstallDir is already in User PATH" -ForegroundColor Green
    } else {
        Write-Host "`n⚠ Adding $InstallDir to your user PATH..." -ForegroundColor Yellow
        $NewPath = if ($PathEnv) { "$PathEnv;$InstallDir" } else { $InstallDir }
        [Environment]::SetEnvironmentVariable("PATH", $NewPath, "User")
        Write-Host "✓ User PATH updated" -ForegroundColor Green
    }

    # Add to current process PATH so it's usable right away
    if (($env:PATH -split ";") -notcontains $InstallDir) {
        $env:PATH = "$InstallDir;$env:PATH"
    }

    Write-Host "`n✓ Installation complete!" -ForegroundColor Green
    Write-Host "Try: ship --help`n" -ForegroundColor Cyan

} catch {
    Write-Error "Installation failed: $_"
} finally {
    if (Test-Path $TmpDir) {
        Remove-Item -Path $TmpDir -Recurse -Force -ErrorAction SilentlyContinue
    }
}
