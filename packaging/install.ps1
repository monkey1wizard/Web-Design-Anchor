# WDA installer (Windows)
# Usage: irm https://raw.githubusercontent.com/monkey1wizard/web-design-anchor/main/packaging/install.ps1 | iex
#
# SHA-256 verification is mandatory. Cosign verification is best effort when
# cosign or its signature files are unavailable, but a completed failed
# verification aborts the installation.

#Requires -Version 5.1
[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'

$Repo = 'monkey1wizard/web-design-anchor'
$RepoCanonical = 'monkey1wizard/web-design-anchor'
$InstallDir = Join-Path $env:LOCALAPPDATA 'Programs\wda'

function Write-Info {
    param([string]$Message)
    Write-Host "wda-install: $Message"
}

function Write-Warn {
    param([string]$Message)
    Write-Error "wda-install: warning: $Message" -ErrorAction Continue
}

function Write-Fatal {
    param([string]$Message)
    Write-Error "wda-install: error: $Message" -ErrorAction Stop
    exit 1
}

function Get-WdaArch {
    switch ($env:PROCESSOR_ARCHITECTURE) {
        'AMD64' { return 'x64' }
        'ARM64' { return 'arm64' }
        default {
            Write-Fatal "Unsupported architecture: $($env:PROCESSOR_ARCHITECTURE). Supported: AMD64, ARM64."
        }
    }
}

function Get-LatestVersion {
    try {
        $response = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest" -UseBasicParsing
        if (-not $response.tag_name) {
            throw 'empty tag_name'
        }
        return $response.tag_name
    } catch {
        Write-Fatal "Failed to fetch latest release version from GitHub API: $_"
    }
}

function Get-FileSha256 {
    param([string]$Path)
    return (Get-FileHash -Path $Path -Algorithm SHA256).Hash.ToLower()
}

$Arch = Get-WdaArch
$Platform = 'windows'
Write-Info "Detected platform: $Platform-$Arch"

$Version = if ($env:WDA_VERSION) { $env:WDA_VERSION } else { Get-LatestVersion }
Write-Info "Installing WDA $Version"

$ArchiveName = "wda-$Version-$Platform-$Arch.zip"
$BaseUrl = if ($env:WDA_RELEASE_BASE_URL) {
    $env:WDA_RELEASE_BASE_URL.TrimEnd('/')
} else {
    "https://github.com/$Repo/releases/download/$Version"
}

$TmpDir = Join-Path $env:TEMP "wda-install-$(New-Guid)"
New-Item -ItemType Directory -Path $TmpDir -Force | Out-Null

try {
    Write-Info "Downloading $ArchiveName..."
    $ArchivePath = Join-Path $TmpDir $ArchiveName
    try {
        Invoke-WebRequest -Uri "$BaseUrl/$ArchiveName" -OutFile $ArchivePath -UseBasicParsing
    } catch {
        Write-Fatal "Download failed: $BaseUrl/$ArchiveName`n$_"
    }

    Write-Info 'Downloading checksums.txt...'
    $ChecksumsPath = Join-Path $TmpDir 'checksums.txt'
    try {
        Invoke-WebRequest -Uri "$BaseUrl/checksums.txt" -OutFile $ChecksumsPath -UseBasicParsing
    } catch {
        Write-Fatal "Download failed: $BaseUrl/checksums.txt`n$_"
    }

    Write-Info 'Verifying SHA-256 integrity...'
    $ExpectedLine = Get-Content $ChecksumsPath |
        Where-Object { $_ -match ("(^|\s)\*?" + [regex]::Escape($ArchiveName) + "$") } |
        Select-Object -First 1
    if (-not $ExpectedLine) {
        Write-Fatal "Checksum entry not found for '$ArchiveName' in checksums.txt."
    }

    $Expected = ($ExpectedLine -split '\s+')[0].ToLower()
    $Actual = Get-FileSha256 $ArchivePath
    if ($Expected -ne $Actual) {
        Write-Fatal "SHA-256 mismatch - download may be corrupted or tampered.`n  Expected: $Expected`n  Actual:   $Actual`nAborting installation."
    }
    Write-Info "SHA-256 OK ($($Actual.Substring(0, 16))...)"

    $CosignExe = Get-Command 'cosign' -ErrorAction SilentlyContinue
    if ($CosignExe) {
        Write-Info 'cosign found - downloading signature files...'
        $SigPath = Join-Path $TmpDir 'checksums.txt.sig'
        $CertPath = Join-Path $TmpDir 'checksums.txt.pem'
        $SignatureFilesDownloaded = $false
        try {
            Invoke-WebRequest -Uri "$BaseUrl/checksums.txt.sig" -OutFile $SigPath -UseBasicParsing
            Invoke-WebRequest -Uri "$BaseUrl/checksums.txt.pem" -OutFile $CertPath -UseBasicParsing
            $SignatureFilesDownloaded = $true
        } catch {
            Write-Warn 'Could not download cosign signature files - skipping (SHA-256 passed).'
        }

        # Keep completed verification outside the download catch so failures stay fatal.
        if ($SignatureFilesDownloaded) {
            & cosign verify-blob `
                --signature $SigPath `
                --certificate $CertPath `
                --certificate-identity-regexp "(?i)https://github.com/$RepoCanonical/\.github/workflows/release\.yml@.*" `
                --certificate-oidc-issuer 'https://token.actions.githubusercontent.com' `
                $ChecksumsPath
            if ($LASTEXITCODE -eq 0) {
                Write-Info 'cosign signature verified.'
            } else {
                Write-Fatal 'cosign signature verification FAILED - checksums.txt does not match a trusted signature. Aborting installation.'
            }
        }
    } else {
        Write-Warn 'cosign not found in PATH - skipping cosign verification (SHA-256 passed).'
    }

    Write-Info 'Extracting archive...'
    $ExtractDir = Join-Path $TmpDir 'extract'
    Expand-Archive -Path $ArchivePath -DestinationPath $ExtractDir -Force
    $BinaryName = 'wda.exe'
    $Binary = Get-ChildItem -Path $ExtractDir -Filter $BinaryName -Recurse | Select-Object -First 1
    if (-not $Binary) {
        Write-Fatal "Binary '$BinaryName' not found in archive."
    }

    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
    Copy-Item -Path $Binary.FullName -Destination (Join-Path $InstallDir $BinaryName) -Force

    Write-Host ''
    Write-Host 'WDA installed successfully.'
    Write-Host "  Location: $(Join-Path $InstallDir $BinaryName)"
    Write-Host ''

    $UserPath = [Environment]::GetEnvironmentVariable('PATH', 'User')
    if ($UserPath -notlike "*$InstallDir*") {
        Write-Host "Add $InstallDir to your PATH by adding this line to your User PATH:"
        Write-Host "  [Environment]::SetEnvironmentVariable('PATH', [Environment]::GetEnvironmentVariable('PATH','User') + ';$InstallDir', 'User')"
        Write-Host ''
    }

    Write-Host 'WDA requires deno and git on your PATH.'
    Write-Host 'The Skill ships at the root of the release archive.'
} finally {
    Remove-Item -Recurse -Force $TmpDir -ErrorAction SilentlyContinue
}
