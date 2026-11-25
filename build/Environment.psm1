Set-StrictMode -Version Latest

function Get-CommandPath {
    param([string] $Name)
    $command = Get-Command $Name -ErrorAction SilentlyContinue
    if ($null -eq $command) { return $null }
    return $command.Source
}

function Get-BuildEnvironment {
    [CmdletBinding()]
    param([Parameter(Mandatory)][string] $Root)

    $cargo = Get-CommandPath 'cargo'
    $rustc = Get-CommandPath 'rustc'
    $rustup = Get-CommandPath 'rustup'
    $trunk = Get-CommandPath 'trunk'
    $wasmBindgen = Get-CommandPath 'wasm-bindgen'
    $cargoApk = Get-CommandPath 'cargo-apk'
    $xcodebuild = Get-CommandPath 'xcodebuild'

    $rustVersion = $null
    $hostTarget = $null
    if ($rustc) {
        $rustVersion = (& $rustc --version 2>$null | Out-String).Trim()
        $details = (& $rustc -vV 2>$null | Out-String)
        if ($details -match '(?m)^host: (.+)$') { $hostTarget = $matches[1].Trim() }
    }

    $installedTargets = @()
    if ($rustup) {
        $installedTargets = @(& $rustup target list --installed 2>$null)
    }
    $wasmTarget = $installedTargets -contains 'wasm32-unknown-unknown'

    $llvmCandidates = @()
    if ($env:ProgramFiles) { $llvmCandidates += (Join-Path $env:ProgramFiles 'LLVM\bin') }
    if (${env:ProgramFiles(x86)}) { $llvmCandidates += (Join-Path ${env:ProgramFiles(x86)} 'LLVM\bin') }
    $llvmBin = $llvmCandidates | Where-Object { Test-Path (Join-Path $_ 'clang.exe') } | Select-Object -First 1
    $clang = if ($llvmBin) { Join-Path $llvmBin 'clang.exe' } else { Get-CommandPath 'clang' }
    $llvmAr = if ($llvmBin -and (Test-Path (Join-Path $llvmBin 'llvm-ar.exe'))) {
        Join-Path $llvmBin 'llvm-ar.exe'
    } else {
        Get-CommandPath 'llvm-ar'
    }

    return [pscustomobject]@{
        Root = $Root
        OS = [System.Environment]::OSVersion.Platform.ToString()
        IsWindows = ([System.Environment]::OSVersion.Platform -eq 'Win32NT')
        Cargo = $cargo
        Rustc = $rustc
        RustcVersion = $rustVersion
        HostTarget = $hostTarget
        Rustup = $rustup
        WasmTarget = $wasmTarget
        Trunk = $trunk
        WasmBindgen = $wasmBindgen
        CargoApk = $cargoApk
        Xcodebuild = $xcodebuild
        LlvmBin = $llvmBin
        Clang = $clang
        LlvmAr = $llvmAr
    }
}

function Show-BuildEnvironment {
    param([Parameter(Mandatory)] $Environment)
    Write-Host "host target      : $($Environment.HostTarget)"
    Write-Host "rustc            : $($Environment.RustcVersion)"
    Write-Host "cargo            : $($Environment.Cargo)"
    Write-Host "wasm32 target    : $(if ($Environment.WasmTarget) { 'installed' } else { 'missing' })"
    Write-Host "trunk            : $(if ($Environment.Trunk) { $Environment.Trunk } else { 'missing (Web build will be skipped)' })"
    Write-Host "wasm-bindgen     : $(if ($Environment.WasmBindgen) { $Environment.WasmBindgen } else { 'missing' })"
    Write-Host "cargo-apk        : $(if ($Environment.CargoApk) { $Environment.CargoApk } else { 'missing' })"
    Write-Host "xcodebuild       : $(if ($Environment.Xcodebuild) { $Environment.Xcodebuild } else { 'missing' })"
    Write-Host "LLVM clang       : $(if ($Environment.Clang) { $Environment.Clang } else { 'missing (wasm/ring may fail)' })"
    Write-Host "LLVM ar          : $(if ($Environment.LlvmAr) { $Environment.LlvmAr } else { 'missing' })"
}

function Initialize-BuildEnvironment {
    [CmdletBinding()]
    param([Parameter(Mandatory)] $Environment)

    if (-not $Environment.Cargo -or -not $Environment.Rustc) {
        throw 'Rust toolchain not found: install rustup/cargo/rustc and reopen the shell'
    }

    if ($Environment.LlvmBin) {
        $env:PATH = "$($Environment.LlvmBin);$env:PATH"
    }
    if ($Environment.Clang) {
        $env:CC_wasm32_unknown_unknown = $Environment.Clang
    }
    if ($Environment.LlvmAr) {
        $env:AR_wasm32_unknown_unknown = $Environment.LlvmAr
    }
}

Export-ModuleMember -Function Get-BuildEnvironment, Show-BuildEnvironment, Initialize-BuildEnvironment
