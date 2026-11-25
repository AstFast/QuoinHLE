[CmdletBinding()]
param(
    [string] $Version = "",
    [ValidateSet('debug', 'release')]
    [string] $Profile = 'release',
    [string[]] $Features = @(),
    [switch] $AllFeatures,
    [switch] $NoDefaultFeatures,
    [int] $Jobs = 0,
    [string] $Target = "",
    [switch] $Locked,
    [switch] $Offline,
    # Faces to put in the release archive, if one is asked for. Both are caller-supplied paths:
    # nothing is bundled by default, and the interface falls back to the system's own fonts.
    # `-Font` names files, `-FontDir` takes every .ttf/.otf/.ttc/.otc in a directory.
    [string[]] $Font = @(),
    [string[]] $FontDir = @(),
    [switch] $ConfigureOnly,
    [switch] $EnvironmentOnly,
    [switch] $NoCore,
    [switch] $NoWeb,
    [switch] $Check,
    [switch] $Test,
    [switch] $NoJit,
    [switch] $Package,
    [string] $PackageDir = ""
)

$ErrorActionPreference = 'Stop'
$root = $PSScriptRoot

Import-Module (Join-Path $PSScriptRoot 'build/Version.psm1') -Force
Import-Module (Join-Path $PSScriptRoot 'build/Environment.psm1') -Force
Import-Module (Join-Path $PSScriptRoot 'build/Targets.psm1') -Force
Import-Module (Join-Path $PSScriptRoot 'build/Package.psm1') -Force

if ($AllFeatures -and $Features.Count -gt 0) {
    throw '-AllFeatures and -Features are mutually exclusive'
}
if ($Jobs -lt 0) {
    throw '-Jobs must be zero or a positive integer'
}

if (-not $Version) {
    $versionFile = Join-Path $root 'VERSION'
    if (Test-Path -LiteralPath $versionFile) {
        $Version = (Get-Content -Raw -Path $versionFile).Trim()
    }
}
if (-not $Version) {
    throw 'no version: pass -Version X.Y.Z[-tag] or write it to VERSION'
}

$environment = Get-BuildEnvironment -Root $root
Show-BuildEnvironment -Environment $environment

if ($EnvironmentOnly) {
    exit 0
}

Update-ProjectVersion -Root $root -Version $Version
Write-Host "version: $Version"
Write-Host "profile: $Profile"

if ($ConfigureOnly) {
    Write-Host 'configuration only; no build was started'
    exit 0
}

Initialize-BuildEnvironment -Environment $environment

$buildOptions = @{
    Profile          = $Profile
    Features         = $Features
    AllFeatures      = $AllFeatures
    NoDefaultFeatures = $NoDefaultFeatures
    Jobs             = $Jobs
    Target           = $Target
    Locked           = $Locked
    Offline          = $Offline
}

# `jit` is one of the default features: the desktop binary arms Rynarmic unless the
# caller asks for an interpreter-only build. The libretro core and the Web bundle
# depend on this crate with `default-features = false`, so they never see it.
# `-NoJit` therefore restates the remaining default features explicitly, with
# `jit` left out. Those switches belong to the root crate only.
$rootBuildOptions = $buildOptions.Clone()
$jitSource = Join-Path $root 'third_party/rynarmic/rynarmic'
$targetIsX64 = if ($Target) {
    $Target -like '*x86_64*'
} else {
    [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture -eq 'X64'
}
if ($NoJit) {
    if ($AllFeatures) {
        throw '-NoJit and -AllFeatures are mutually exclusive'
    }
    $rootFeatures = @($Features | Where-Object { $_ -ne 'jit' })
    if (-not $NoDefaultFeatures) {
        # default = script, ui, device, jit; rebuild that set without jit
        $rootFeatures = @(($rootFeatures + @('script', 'ui', 'device')) | Select-Object -Unique)
        $rootBuildOptions.NoDefaultFeatures = $true
    }
    $rootBuildOptions.Features = $rootFeatures
    Write-Host 'jit: interpreter-only build (script, ui, device without jit)'
} elseif (-not $targetIsX64 -or -not (Test-Path -LiteralPath $jitSource)) {
    Write-Warning 'jit is a default feature: this build needs an x86-64 target and the initialized third_party/rynarmic submodule (git submodule update --init). Use -NoJit for an interpreter-only build.'
}

if ($Check) {
    Invoke-ProjectCheck -Root $root @rootBuildOptions
}
if ($Test) {
    Invoke-ProjectTests -Root $root @rootBuildOptions
}

Write-Host "building root crate ($Profile)"
Invoke-RootBuild -Root $root @rootBuildOptions

# The core is a workspace member: it shares the root's target directory and the
# caller's -Target/-Profile, but takes no feature switches of its own.
$coreBuildOptions = @{
    Profile = $Profile
    Jobs    = $Jobs
    Target  = $Target
    Locked  = $Locked
    Offline = $Offline
}

if (-not $NoCore) {
    Write-Host "building libretro core ($Profile)"
    Invoke-CoreBuild -Root $root @coreBuildOptions
}

$webStatus = 'not requested (-NoWeb)'
if (-not $NoWeb) {
    Write-Host "building Web bundle ($Profile)"
    $webStatus = Invoke-WebBuild -Environment $environment -Profile $Profile `
        -Features $Features -NoDefaultFeatures:$NoDefaultFeatures -Locked:$Locked -Offline:$Offline
}

if ($Package) {
    $packageDir = if ($PackageDir) { $PackageDir } else { Join-Path $root 'dist' }
    Write-Host "packaging into $packageDir"
    $null = New-ReleasePackage -Root $root -Version $Version -Profile $Profile -Target $Target `
        -OutputDir $packageDir -Font $Font -FontDir $FontDir
}

Write-Host "build complete: $Version ($Profile); Web: $webStatus"
