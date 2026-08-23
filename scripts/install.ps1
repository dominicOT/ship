# PowerShell script to install ship for Windows
# Usage: irm https://raw.githubusercontent.com/dominicOT/ship/master/scripts/install.ps1 | iex
# or:    powershell -ExecutionPolicy Bypass -Command "& { iwr https://raw.githubusercontent.com/dominicOT/ship/master/scripts/install.ps1 | iex }"

param(
    [string]$InstallDir = "$env:LOCALAPPDATA\ship\bin"
)

$ErrorActionPreference = "Stop"
$VerbosePreference = "Continue"

$Repo = "dominicOT/ship"
$Prog = "ship"
$DownloadBase = "https://github.com/$Repo/releases/latest/download"

Write-Host "Installing $Prog for Windows`n" -ForegroundColor Cyan

# Detect architecture
$arch = $env:PROCESSOR_ARCHITECTURE
switch ($arch) {
    "AMD64" { $arch = "x86_64"; break }
    "ARM64" { $arch = "aarch64"; break }
    default {
        Write-Error "Unsupported architecture: $arch. Supported: x86_64, aarch64"
        exit 1
    }
}

$AssetName = "$Prog-windows-$arch.zip"
$AssetUrl = "$DownloadBase/$AssetName"

Write-Host "Detected architecture: $arch"
Write-Host "Downloading from: $AssetUrl`n"

# Create temporary directory
$TmpDir = New-TemporaryDirectory
trap {
    Remove-Item -Path $TmpDir -Recurse -Force -ErrorAction SilentlyContinue
}

$AssetPath = Join-Path $TmpDir $AssetName

try {
    # Download the release asset
    Write-Host "Downloading $AssetName..."
    $ProgressPreference = "SilentlyContinue"
    Invoke-WebRequest -Uri $AssetUrl -OutFile $AssetPath -ErrorAction Stop
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
        exit 1
    }

    # Copy to install directory
    $InstallPath = Join-Path $InstallDir "$Prog.exe"
    Copy-Item -Path $BinarySource -Destination $InstallPath -Force
    Write-Host "✓ Installed to: $InstallPath" -ForegroundColor Green

    # Check if install directory is in PATH
    $PathEnv = [Environment]::GetEnvironmentVariable("PATH", "User")
    if ($PathEnv -split ";" -contains $InstallDir) {
        Write-Host "✓ $InstallDir is already in PATH" -ForegroundColor Green
    } else {
        Write-Host "`n⚠ Adding $InstallDir to your user PATH..." -ForegroundColor Yellow
        
        if ($PathEnv) {
            $NewPath = "$PathEnv;$InstallDir"
        } else {
            $NewPath = $InstallDir
        }
        
        [Environment]::SetEnvironmentVariable("PATH", $NewPath, "User")
        Write-Host "✓ PATH updated (you may need to restart your terminal)" -ForegroundColor Green
    }

    Write-Host "`n✓ Installation complete!" -ForegroundColor Green
    Write-Host "Try: & '$InstallPath' --help`n" -ForegroundColor Cyan

} catch {
    Write-Error "Installation failed: $_"
    exit 1
} finally {
    Remove-Item -Path $TmpDir -Recurse -Force -ErrorAction SilentlyContinue
}
