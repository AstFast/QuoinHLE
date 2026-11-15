Set-StrictMode -Version Latest

function Test-ProjectVersion {
    param([Parameter(Mandatory)][string] $Version)
    return $Version -match '^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$'
}

function Write-Utf8 {
    param([string] $Path, [string] $Text)
    $encoding = [System.Text.UTF8Encoding]::new($false)
    [System.IO.File]::WriteAllText($Path, $Text, $encoding)
}

function Replace-CargoVersion {
    param([string] $Path, [string] $Version)
    if (-not (Test-Path -LiteralPath $Path)) { return }
    $text = [System.IO.File]::ReadAllText($Path)
    $placement = 'version = "' + $Version + '"'
    $text = [regex]::Replace($text, '(?m)^version = "[^"]+"', $placement)
    Write-Utf8 $Path $text
}

function Replace-LockVersion {
    param([string] $Path, [string[]] $Names, [string] $Version)
    if (-not (Test-Path -LiteralPath $Path)) { return }
    $text = [System.IO.File]::ReadAllText($Path)
    foreach ($name in $Names) {
        $pattern = '(?s)(\[\[package\]\]\s*\r?\nname = "' + [regex]::Escape($name) + '"\s*\r?\nversion = ")[^"]*(")'
        $replacement = '${1}' + $Version + '${2}'
        $text = [regex]::Replace($text, $pattern, $replacement)
    }
    Write-Utf8 $Path $text
}

function Update-ProjectVersion {
    [CmdletBinding()]
    param([Parameter(Mandatory)][string] $Root, [Parameter(Mandatory)][string] $Version)

    if (-not (Test-ProjectVersion $Version)) {
        throw "not a version Cargo accepts: $Version"
    }

    # The workspace version lives in `[workspace.package]`; `quoin` and
    # `quoin_libretro` inherit it, so those manifests carry no literal to rewrite.
    # `platform/web` is excluded from the workspace and keeps its own.
    Replace-CargoVersion (Join-Path $Root 'Cargo.toml') $Version
    Replace-CargoVersion (Join-Path $Root 'platform/web/Cargo.toml') $Version

    # One workspace lock file covers both `quoin` and the libretro core.
    Replace-LockVersion (Join-Path $Root 'Cargo.lock') @('quoin', 'quoin_libretro') $Version
    Replace-LockVersion (Join-Path $Root 'platform/web/Cargo.lock') @('quoin', 'quoin-web') $Version

    # What RetroArch shows in its core list. It is a plain text field, so nothing
    # else in the build would notice it going stale.
    $info = Join-Path $Root 'platform/libretro/quoin_libretro.info'
    if (Test-Path -LiteralPath $info) {
        $text = [System.IO.File]::ReadAllText($info)
        $text = $text -replace '(?m)^display_version = "[^"]*"', ('display_version = "' + $Version + '"')
        Write-Utf8 $info $text
    }

    $plist = Join-Path $Root 'platform/ios/Info.plist'
    if (Test-Path -LiteralPath $plist) {
        $text = [System.IO.File]::ReadAllText($plist)
        $text = $text -replace '(?s)(<key>CFBundleShortVersionString</key>\s*<string>)[^<]*(</string>)', ('${1}' + $Version + '${2}')
        Write-Utf8 $plist $text
    }

    Write-Utf8 (Join-Path $Root 'VERSION') ($Version + "`n")
}

Export-ModuleMember -Function Test-ProjectVersion, Update-ProjectVersion
