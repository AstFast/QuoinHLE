Set-StrictMode -Version Latest

function Invoke-External {
    param(
        [Parameter(Mandatory)][string] $Executable,
        [string[]] $Arguments = @(),
        [Parameter(Mandatory)][string] $WorkingDirectory
    )
    Push-Location $WorkingDirectory
    try {
        & $Executable @Arguments
        if ($LASTEXITCODE -ne 0) {
            throw "$Executable exited with $LASTEXITCODE"
        }
    } finally {
        Pop-Location
    }
}

function Get-CargoOptions {
    param(
        [ValidateSet('debug', 'release')][string] $Profile = 'release',
        [string[]] $Features = @(),
        [switch] $AllFeatures,
        [switch] $NoDefaultFeatures,
        [int] $Jobs = 0,
        [string] $Target = '',
        [switch] $Locked,
        [switch] $Offline
    )
    $options = @()
    if ($Profile -eq 'release') {
        $options += '--release'
    } else {
        $options += @('--profile', 'dev')
    }
    if ($AllFeatures) {
        $options += '--all-features'
    } elseif ($Features.Count -gt 0) {
        $options += @('--features', ($Features -join ','))
    }
    if ($NoDefaultFeatures) {
        $options += '--no-default-features'
    }
    if ($Jobs -gt 0) {
        $options += @('--jobs', ([string]$Jobs))
    }
    if ($Target) {
        $options += @('--target', $Target)
    }
    if ($Locked) {
        $options += '--locked'
    }
    if ($Offline) {
        $options += '--offline'
    }
    return @($options)
}

function Invoke-RootBuild {
    param(
        [Parameter(Mandatory)][string] $Root,
        [ValidateSet('debug', 'release')][string] $Profile = 'release',
        [string[]] $Features = @(),
        [switch] $AllFeatures,
        [switch] $NoDefaultFeatures,
        [int] $Jobs = 0,
        [string] $Target = '',
        [switch] $Locked,
        [switch] $Offline
    )
    $arguments = @('build') + (Get-CargoOptions -Profile $Profile -Features $Features `
        -AllFeatures:$AllFeatures -NoDefaultFeatures:$NoDefaultFeatures -Jobs $Jobs `
        -Target $Target -Locked:$Locked -Offline:$Offline)
    Invoke-External -Executable 'cargo' -Arguments $arguments -WorkingDirectory $Root
}

function Invoke-CoreBuild {
    param(
        [Parameter(Mandatory)][string] $Root,
        [ValidateSet('debug', 'release')][string] $Profile = 'release',
        [int] $Jobs = 0,
        [string] $Target = '',
        [switch] $Locked,
        [switch] $Offline
    )
    # A workspace member, so `-p` is enough. Unlike the root crate, the core has
    # no features of its own; the root's feature switches must not be forwarded.
    $arguments = @('build', '-p', 'quoin_libretro') + (Get-CargoOptions -Profile $Profile -Jobs $Jobs `
        -Target $Target -Locked:$Locked -Offline:$Offline)
    Invoke-External -Executable 'cargo' -Arguments $arguments -WorkingDirectory $Root
}

function Invoke-WebBuild {
    param(
        [Parameter(Mandatory)] $Environment,
        [ValidateSet('debug', 'release')][string] $Profile = 'release',
        [string[]] $Features = @(),
        [switch] $NoDefaultFeatures,
        [switch] $Locked,
        [switch] $Offline
    )
    if (-not $Environment.WasmTarget) {
        Write-Warning 'wasm32-unknown-unknown is not installed; skipped the Web build'
        return 'SKIPPED (wasm32-unknown-unknown is not installed)'
    }
    if (-not $Environment.Trunk) {
        Write-Warning 'trunk is not installed; skipped the Web build'
        return 'SKIPPED (trunk is not installed)'
    }
    # A manual `trunk build` does not go through build.ps1, so the C toolchain bindings are
    # re-applied here rather than assuming Initialize-BuildEnvironment already ran. `ring`
    # needs CC/AR for wasm32, and the bindings are just environment variables (idempotent).
    if ($Environment.LlvmBin) {
        $env:PATH = "$($Environment.LlvmBin);$env:PATH"
    }
    if ($Environment.Clang) {
        $env:CC_wasm32_unknown_unknown = $Environment.Clang
    }
    if ($Environment.LlvmAr) {
        $env:AR_wasm32_unknown_unknown = $Environment.LlvmAr
    } else {
        Write-Warning 'llvm-ar is missing; a wasm32 build of ring may fail'
    }
    # Trunk takes cargo flags from `data-trunk` attributes on `<link rel="rust">`, not from
    # its CLI, so the root build's feature switches cannot be forwarded. Say so rather
    # than letting -Features silently mean nothing for the Web bundle.
    if ($Features.Count -gt 0 -or $NoDefaultFeatures -or $Locked -or $Offline) {
        Write-Warning 'the Web bundle takes its cargo features from platform/web/index.html; -Features/-NoDefaultFeatures/-Locked/-Offline do not apply to it'
    }
    $arguments = @('build')
    if ($Profile -eq 'release') {
        $arguments += '--release'
    }
    # Out-Host keeps trunk's own output visible without letting it join this
    # function's return value; the caller prints that value as the Web status.
    Invoke-External -Executable $Environment.Trunk -Arguments $arguments `
        -WorkingDirectory (Join-Path $Environment.Root 'platform/web') | Out-Host
    return 'BUILT'
}

function Invoke-ProjectCheck {
    param(
        [Parameter(Mandatory)][string] $Root,
        [ValidateSet('debug', 'release')][string] $Profile = 'release',
        [string[]] $Features = @(),
        [switch] $AllFeatures,
        [switch] $NoDefaultFeatures,
        [int] $Jobs = 0,
        [string] $Target = '',
        [switch] $Locked,
        [switch] $Offline
    )
    $arguments = @('check', '--all-targets') + (Get-CargoOptions -Profile $Profile -Features $Features `
        -AllFeatures:$AllFeatures -NoDefaultFeatures:$NoDefaultFeatures -Jobs $Jobs `
        -Target $Target -Locked:$Locked -Offline:$Offline)
    Invoke-External -Executable 'cargo' -Arguments $arguments -WorkingDirectory $Root
}

function Invoke-ProjectTests {
    param(
        [Parameter(Mandatory)][string] $Root,
        [ValidateSet('debug', 'release')][string] $Profile = 'release',
        [string[]] $Features = @(),
        [switch] $AllFeatures,
        [switch] $NoDefaultFeatures,
        [int] $Jobs = 0,
        [string] $Target = '',
        [switch] $Locked,
        [switch] $Offline
    )
    $arguments = @('test', '--lib', '--bins') + (Get-CargoOptions -Profile $Profile -Features $Features `
        -AllFeatures:$AllFeatures -NoDefaultFeatures:$NoDefaultFeatures -Jobs $Jobs `
        -Target $Target -Locked:$Locked -Offline:$Offline)
    Invoke-External -Executable 'cargo' -Arguments $arguments -WorkingDirectory $Root
}

Export-ModuleMember -Function Invoke-RootBuild, Invoke-CoreBuild, Invoke-WebBuild, Invoke-ProjectCheck, Invoke-ProjectTests
