[CmdletBinding()]
param(
    [string] $IdentityName = $env:MSIX_IDENTITY_NAME,
    [string] $Publisher = $env:MSIX_PUBLISHER,
    [string] $PublisherDisplayName = $env:MSIX_PUBLISHER_DISPLAY_NAME,
    [string] $PackageVersion = $env:MSIX_PACKAGE_VERSION,
    [string] $OutputDirectory = "target/release/msix",
    [switch] $RunWindowsAppCertificationKit
)

$ErrorActionPreference = "Stop"

function Require-Value([string] $Name, [string] $Value) {
    if ([string]::IsNullOrWhiteSpace($Value)) {
        throw "$Name is required. Copy the exact value from Partner Center > Product management > Product identity."
    }
}

function Find-WindowsSdkTool([string] $Name) {
    if ($Name -eq "appcert.exe") {
        $appCertDirectory = Join-Path ${env:ProgramFiles(x86)} "Windows Kits\\10\\App Certification Kit"
        $appCert = Get-ChildItem -Path $appCertDirectory -Filter $Name -Recurse -ErrorAction SilentlyContinue |
            Select-Object -First 1
        if ($null -ne $appCert) {
            return $appCert.FullName
        }
    }
    $sdkBin = Join-Path ${env:ProgramFiles(x86)} "Windows Kits\\10\\bin"
    $tool = Get-ChildItem -Path $sdkBin -Filter $Name -Recurse |
        Where-Object { $_.FullName -match '\\x64\\' } |
        Sort-Object FullName -Descending |
        Select-Object -First 1
    if ($null -eq $tool) {
        throw "$Name was not found in the Windows SDK. Install the Windows 10/11 SDK including MSIX Packaging Tools."
    }
    return $tool.FullName
}

Require-Value "MSIX_IDENTITY_NAME" $IdentityName
Require-Value "MSIX_PUBLISHER" $Publisher
Require-Value "MSIX_PUBLISHER_DISPLAY_NAME" $PublisherDisplayName
Require-Value "MSIX_PACKAGE_VERSION" $PackageVersion
if ($PackageVersion -notmatch '^[1-9]\d{0,4}(\.\d{1,5}){3}$') {
    throw "MSIX_PACKAGE_VERSION must have four numeric components and a non-zero first component, for example 1.0.0.0."
}
foreach ($component in $PackageVersion.Split('.')) {
    if ([int]$component -gt 65535) {
        throw "MSIX_PACKAGE_VERSION components must be between 0 and 65535."
    }
}

$packageRoot = Resolve-Path (Join-Path $PSScriptRoot "../..")
Push-Location $packageRoot
try {
    $executable = Join-Path $packageRoot "target/release/Tempo.exe"
    if (-not (Test-Path $executable -PathType Leaf)) {
        throw "Tempo.exe is missing. Run cargo build --release for Windows before packaging."
    }

    $outputDirectory = Join-Path $packageRoot $OutputDirectory
    $stagingDirectory = Join-Path $outputDirectory "staging"
    $packagePath = Join-Path $outputDirectory "Tempo-Windows.msix"
    Remove-Item -Recurse -Force $stagingDirectory -ErrorAction SilentlyContinue
    Remove-Item -Force $packagePath -ErrorAction SilentlyContinue
    New-Item -ItemType Directory -Force (Join-Path $stagingDirectory "Assets") | Out-Null

    Copy-Item $executable $stagingDirectory
    Copy-Item "packaging/windows/Assets/*" (Join-Path $stagingDirectory "Assets")

    $manifest = Get-Content -Raw "packaging/windows/AppxManifest.xml.in"
    $replacements = @{
        "__MSIX_IDENTITY_NAME__" = [System.Security.SecurityElement]::Escape($IdentityName)
        "__MSIX_PUBLISHER__" = [System.Security.SecurityElement]::Escape($Publisher)
        "__MSIX_PUBLISHER_DISPLAY_NAME__" = [System.Security.SecurityElement]::Escape($PublisherDisplayName)
        "__MSIX_VERSION__" = $PackageVersion
    }
    foreach ($token in $replacements.Keys) {
        $manifest = $manifest.Replace($token, $replacements[$token])
    }
    Set-Content -Path (Join-Path $stagingDirectory "AppxManifest.xml") -Value $manifest -Encoding utf8NoBOM

    $makeAppx = Find-WindowsSdkTool "MakeAppx.exe"
    & $makeAppx pack /o /d $stagingDirectory /p $packagePath
    if ($LASTEXITCODE -ne 0) { throw "MakeAppx pack failed with exit code $LASTEXITCODE." }
    & $makeAppx validate /p $packagePath
    if ($LASTEXITCODE -ne 0) { throw "MakeAppx validate failed with exit code $LASTEXITCODE." }

    if ($RunWindowsAppCertificationKit) {
        $appCert = Find-WindowsSdkTool "appcert.exe"
        $reportPath = Join-Path $outputDirectory "Tempo-Windows-wack.xml"
        & $appCert reset
        if ($LASTEXITCODE -ne 0) { throw "Windows App Certification Kit reset failed with exit code $LASTEXITCODE." }
        & $appCert test -appxpackagepath $packagePath -reportoutputpath $reportPath
        if ($LASTEXITCODE -ne 0) { throw "Windows App Certification Kit failed with exit code $LASTEXITCODE." }
    }

    Write-Output "Created $packagePath"
}
finally {
    Pop-Location
}
