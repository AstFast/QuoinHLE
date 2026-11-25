Set-StrictMode -Version Latest

# What a release archive contains, and why each piece is there:
#
#   quoin.exe, quoin_libretro.dll   the two things a person runs;
#   quoin_libretro.info             the descriptor RetroArch reads from `info/`;
#   README.md, LICENSE              the terms the binaries are given under;
#   docs/                           credit and disclaimer, so a recipient has them
#                                   beside the binary rather than only online;
#   data/ui-fonts/                  optional, see below.
#
# **No font is bundled unless one is named.** The interface already has three ways to find a
# face at run time — the manager's font page, `program_data_dir()/fonts`, and the operating
# system's own paths — so a build that names nothing still draws text, and an archive that
# carries nothing is a complete release. `-Font` and `-FontDir` are for the case where a
# particular face should travel with the binary: they copy caller-supplied files into the
# archive, which puts the licence question on whoever chose them. Nothing in this repository is
# a font, so a checkout has nothing to fall back to.
function Get-FontSelection {
    param(
        [string[]] $Font = @(),
        [string[]] $FontDir = @()
    )
    $files = [System.Collections.Generic.List[string]]::new()
    foreach ($path in $Font) {
        if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
            throw "no such font file: $path"
        }
        $files.Add((Resolve-Path -LiteralPath $path).Path)
    }
    foreach ($dir in $FontDir) {
        if (-not (Test-Path -LiteralPath $dir -PathType Container)) {
            throw "no such font directory: $dir"
        }
        Get-ChildItem -LiteralPath $dir -File |
            Where-Object { $_.Extension -match '(?i)^\.(ttf|otf|ttc|otc)$' } |
            ForEach-Object { $files.Add($_.FullName) }
    }
    return @($files | Select-Object -Unique)
}

function Copy-FontLicences {
    param(
        [string[]] $Files,
        [string] $Destination
    )
    # A redistributed face has to carry its notice and terms, and the file is nearly always
    # sitting beside it under one of a handful of names. Take whatever is there rather than
    # inventing a convention; the caller warns when nothing was found.
    $copied = @()
    $directories = $Files | ForEach-Object { Split-Path -Parent $_ } | Select-Object -Unique
    foreach ($dir in $directories) {
        Get-ChildItem -LiteralPath $dir -File |
            Where-Object { $_.Name -match '(?i)^(licen[cs]e|ofl|copying)' -and $_.Extension -match '(?i)^($|\.(txt|md|html?))$' } |
            ForEach-Object {
                # Two families can both ship a file called `license.txt`; the second one keeps
                # its directory's name so neither licence is silently dropped.
                $target = Join-Path $Destination $_.Name
                if (Test-Path -LiteralPath $target) {
                    $target = Join-Path $Destination ((Split-Path -Leaf $dir) + '-' + $_.Name)
                }
                Copy-Item -LiteralPath $_.FullName -Destination $target -Force
                $copied += (Split-Path -Leaf $target)
            }
    }
    return @($copied)
}

function New-ReleasePackage {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)][string] $Root,
        [Parameter(Mandatory)][string] $Version,
        [string] $Profile = 'release',
        [string] $Target = '',
        [Parameter(Mandatory)][string] $OutputDir,
        [string[]] $Font = @(),
        [string[]] $FontDir = @()
    )

    $targetRoot = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $Root 'target' }
    if ($Target) { $targetRoot = Join-Path $targetRoot $Target }
    $artifacts = Join-Path $targetRoot $Profile

    $exe = Join-Path $artifacts 'quoin.exe'
    $core = Join-Path $artifacts 'quoin_libretro.dll'
    foreach ($required in @($exe, $core)) {
        if (-not (Test-Path -LiteralPath $required)) {
            throw "missing build artifact: $required (run the build first, and without -NoCore)"
        }
    }

    $name = "quoin-$Version-win64"
    $stage = Join-Path $OutputDir $name
    if (Test-Path -LiteralPath $stage) { Remove-Item -LiteralPath $stage -Recurse -Force }
    New-Item -ItemType Directory -Force -Path (Join-Path $stage 'docs') | Out-Null

    Copy-Item -LiteralPath $exe -Destination $stage
    Copy-Item -LiteralPath $core -Destination $stage
    Copy-Item -LiteralPath (Join-Path $Root 'platform/libretro/quoin_libretro.info') -Destination $stage
    Copy-Item -LiteralPath (Join-Path $Root 'README.md') -Destination $stage
    Copy-Item -LiteralPath (Join-Path $Root 'LICENSE') -Destination $stage
    Copy-Item -LiteralPath (Join-Path $Root 'docs/Credit.md') -Destination (Join-Path $stage 'docs')
    Copy-Item -LiteralPath (Join-Path $Root 'docs/DISCLAIMER.txt') -Destination (Join-Path $stage 'docs')

    $selected = @(Get-FontSelection -Font $Font -FontDir $FontDir)
    $fontNote = 'font: none, the interface uses the system font'
    if ($selected.Count -gt 0) {
        $fontsOut = Join-Path $stage 'data/ui-fonts'
        New-Item -ItemType Directory -Force -Path $fontsOut | Out-Null
        foreach ($file in $selected) { Copy-Item -LiteralPath $file -Destination $fontsOut -Force }
        $licences = @(Copy-FontLicences -Files $selected -Destination $fontsOut)
        $fontNote = "font: $($selected.Count) face(s)"
        if ($licences.Count -gt 0) {
            $fontNote = "$fontNote + $($licences -join ', ')"
        } else {
            $fontNote = "$fontNote, no licence file found"
            Write-Warning 'the archive carries a font with no licence file beside it: check that its terms allow redistribution'
        }
    }

    $zip = Join-Path $OutputDir "$name.zip"
    if (Test-Path -LiteralPath $zip) { Remove-Item -LiteralPath $zip -Force }
    Compress-Archive -Path $stage -DestinationPath $zip -CompressionLevel Optimal

    $hash = (Get-FileHash -LiteralPath $zip -Algorithm SHA256).Hash.ToLower()
    "$hash  $name.zip" | Set-Content -LiteralPath (Join-Path $OutputDir 'SHA256SUMS.txt') -Encoding ascii

    Write-Host "package: $zip"
    Write-Host $fontNote
    Write-Host ("sha256: {0}" -f $hash)
    return $zip
}

Export-ModuleMember -Function New-ReleasePackage
