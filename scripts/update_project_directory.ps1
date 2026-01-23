Param(
    [string]$Root = (Get-Location).Path,
    [string]$Output = "project_directory.md",
    [string]$MapFile = "project_directory_map.toml",
    [string[]]$Roots = @("crates"),
    [string]$SourceGlob = "/src/",
    [switch]$UpdateTouchedOnly
)

function Normalize-PathKey {
    Param([string]$Path)
    if (-not $Path) {
        return $Path
    }
    $normalized = $Path.Replace('\', '/')
    if ($normalized.StartsWith('./')) {
        $normalized = $normalized.Substring(2)
    }
    if ($normalized.StartsWith('.\\')) {
        $normalized = $normalized.Substring(2)
    }
    return $normalized
}

function Read-DescriptionMap {
    Param([string]$Path)
    $map = @{}
    if (-not (Test-Path $Path)) {
        return $map
    }
    foreach ($line in Get-Content $Path) {
        $trim = $line.Trim()
        if (-not $trim -or $trim.StartsWith("#")) {
            continue
        }
        if ($trim -match '^(?<key>[^=]+?)\s*=\s*"(?<value>.*)"\s*$') {
            $key = Normalize-PathKey -Path $Matches['key'].Trim()
            $value = $Matches['value'].Trim()
            if ($key) {
                $map[$key] = $value
            }
        }
    }
    return $map
}

function Read-ExistingDescriptions {
    Param([string]$Path)
    $map = @{}
    if (-not (Test-Path $Path)) {
        return $map
    }
    $current = $null
    foreach ($line in Get-Content $Path) {
        if ($line -match '^##\s+(.+)$') {
            $current = Normalize-PathKey -Path $Matches[1].Trim()
            continue
        }
        if ($current -and $line -match '^Description:\s*(.+)$') {
            $map[$current] = $Matches[1].Trim()
        }
    }
    return $map
}

function Get-RustFunctions {
    Param([string]$Path)
    $lines = Get-Content $Path
    $funcs = New-Object System.Collections.Generic.List[object]
    $lineCount = $lines.Count
    for ($i = 0; $i -lt $lineCount; $i++) {
        $line = $lines[$i]
        $trimmed = $line.Trim()
        if ($trimmed.StartsWith('//')) {
            continue
        }
        if ($line -match '^\s*(pub(\([^)]*\))?\s+)?(async\s+)?fn\s+([A-Za-z0-9_]+)') {
            $name = $Matches[4]
            $startLine = $i + 1
            $endLine = $startLine
            $depth = 0
            $foundBody = $false
            $foundSigEnd = $false
            for ($j = $i; $j -lt $lineCount; $j++) {
                $text = $lines[$j]
                $text = $text -replace '"([^"\\]|\\.)*"', '""'
                $text = $text -replace '//.*$', ''
                if (-not $foundBody) {
                    if ($text -match ';') {
                        $endLine = $j + 1
                        $foundSigEnd = $true
                        break
                    }
                    if ($text -match '\{') {
                        $foundBody = $true
                        $openCount = ([regex]::Matches($text, '\{')).Count
                        $closeCount = ([regex]::Matches($text, '\}')).Count
                        $depth = $openCount - $closeCount
                        if ($depth -le 0) {
                            $endLine = $j + 1
                            break
                        }
                        continue
                    }
                } else {
                    $openCount = ([regex]::Matches($text, '\{')).Count
                    $closeCount = ([regex]::Matches($text, '\}')).Count
                    $depth += ($openCount - $closeCount)
                    if ($depth -le 0) {
                        $endLine = $j + 1
                        break
                    }
                }
            }
            if (-not $foundBody -and -not $foundSigEnd) {
                $endLine = $startLine
            }
            $funcs.Add([pscustomobject]@{
                Name = $name
                Start = $startLine
                End = $endLine
            })
        }
    }
    return $funcs
}

$rootPath = (Resolve-Path $Root).Path
$outputPath = Join-Path $rootPath $Output
$mapPath = Join-Path $rootPath $MapFile

$descMap = Read-DescriptionMap -Path $mapPath
$existingMap = Read-ExistingDescriptions -Path $outputPath

$files = New-Object System.Collections.Generic.List[System.IO.FileInfo]
foreach ($root in $Roots) {
    $rootDir = Join-Path $rootPath $root
    if (-not (Test-Path $rootDir)) {
        continue
    }
    $items = Get-ChildItem -Path $rootDir -Recurse -File | Where-Object {
        $_.Extension -eq '.rs' -and $_.FullName -notmatch '\\\\target\\\\'
    }
    foreach ($item in $items) {
        if ($SourceGlob) {
            $normalized = $item.FullName.Replace('\', '/')
            if ($normalized -notmatch [Regex]::Escape($SourceGlob)) {
                continue
            }
        }
        $files.Add($item)
    }
}

$touched = @()
if ($UpdateTouchedOnly) {
    try {
        $touched = git diff --name-only | Where-Object { $_ }
    } catch {
        $touched = @()
    }
}

$touchedSet = @{}
if ($UpdateTouchedOnly -and $touched.Count -gt 0) {
    foreach ($entry in $touched) {
        $normalized = Normalize-PathKey -Path $entry
        $touchedSet[$normalized] = $true
    }
}

function Normalize-RelPath {
    Param([string]$Path, [string]$Base)
    $rel = $Path
    $prev = Get-Location
    try {
        Set-Location -Path $Base
        $rel = Resolve-Path -Relative $Path
    } finally {
        Set-Location -Path $prev
    }
    $rel = Normalize-PathKey -Path $rel
    return $rel
}

function Guess-DescriptionFromPath {
    Param([string]$RelPath)
    $stem = [System.IO.Path]::GetFileNameWithoutExtension($RelPath)
    if ($stem -eq 'mod') {
        $parent = Split-Path -Path $RelPath -Parent
        $stem = Split-Path -Path $parent -Leaf
    }
    if (-not $stem) {
        return $null
    }
    $parts = $stem -split '[-_]'
    $title = ($parts | ForEach-Object {
        if ($_.Length -le 1) { $_.ToUpper() } else { $_.Substring(0,1).ToUpper() + $_.Substring(1) }
    }) -join ' '
    return "$title module."
}

function Get-DescriptionFromSource {
    Param([string]$Path)
    $lines = Get-Content $Path
    $desc = New-Object System.Collections.Generic.List[string]
    $started = $false
    foreach ($line in $lines) {
        $trim = $line.Trim()
        if (-not $trim) {
            if ($desc.Count -gt 0) {
                break
            }
            continue
        }
        if ($trim -match '^(//!|///)\s*(.+)?$') {
            $text = $Matches[2].Trim()
            if ($text) {
                $desc.Add($text)
            }
            $started = $true
            continue
        }
        if (-not $started -and $trim -match '^//\s*(.+)$') {
            $text = $Matches[1].Trim()
            if ($text) {
                $desc.Add($text)
            }
            continue
        }
        break
    }
    if ($desc.Count -gt 0) {
        return ($desc -join ' ').Trim()
    }
    return $null
}

$entries = @()
foreach ($file in ($files | Sort-Object FullName)) {
    $rel = Normalize-RelPath -Path $file.FullName -Base $rootPath
    if ($UpdateTouchedOnly -and $touchedSet.Count -gt 0) {
        if (-not $touchedSet.ContainsKey($rel)) {
            continue
        }
    }
    $desc = $descMap[$rel]
    if (-not $desc) {
        $desc = $existingMap[$rel]
    }
    if (-not $desc -or $desc -eq "TODO: add description.") {
        $desc = Get-DescriptionFromSource -Path $file.FullName
    }
    if (-not $desc) {
        $desc = Guess-DescriptionFromPath -RelPath $rel
    }
    if (-not $desc) {
        $desc = "TODO: add description."
    }
    $funcs = @(Get-RustFunctions -Path $file.FullName)
    if ($funcs.Count -gt 0) {
        $funcText = ($funcs | ForEach-Object { "``$($_.Name)`` (L$($_.Start)-L$($_.End))" }) -join ', '
    } else {
        $funcText = 'None'
    }
    $entries += @(
        "## $rel",
        "Description: $desc",
        "Functions: $funcText",
        ''
    )
}

if ($UpdateTouchedOnly -and (Test-Path $outputPath)) {
    $current = Get-Content $outputPath
    $output = New-Object System.Collections.Generic.List[string]
    $output.Add('# Project Directory')
    $output.Add('')

    $existing = @{}
    $block = New-Object System.Collections.Generic.List[string]
    $currentName = $null
    foreach ($line in $current) {
        if ($line -match '^##\s+(.+)$') {
            if ($currentName) {
                $existing[$currentName] = $block.ToArray()
            }
            $currentName = Normalize-PathKey -Path $Matches[1].Trim()
            $block = New-Object System.Collections.Generic.List[string]
            $block.Add("## $currentName")
            continue
        }
        if ($currentName) {
            $block.Add($line)
        }
    }
    if ($currentName) {
        $existing[$currentName] = $block.ToArray()
    }

    $updated = @{}
    for ($i = 0; $i -lt $entries.Count; $i += 4) {
        $header = $entries[$i]
        $name = Normalize-PathKey -Path $header.Substring(3).Trim()
        $updated[$name] = @($entries[$i], $entries[$i + 1], $entries[$i + 2], $entries[$i + 3])
    }

    $keys = New-Object System.Collections.Generic.List[string]
    foreach ($key in $existing.Keys) { $keys.Add($key) }
    foreach ($key in $updated.Keys) { if (-not $keys.Contains($key)) { $keys.Add($key) } }
    $keys = $keys | Sort-Object

    foreach ($key in $keys) {
        $blockLines = $updated[$key]
        if (-not $blockLines) {
            $blockLines = $existing[$key]
        }
        foreach ($line in $blockLines) {
            $output.Add($line)
        }
    }

    Set-Content -Path $outputPath -Value $output -Encoding utf8
    exit 0
}

$headerLines = @('# Project Directory', '')
Set-Content -Path $outputPath -Value ($headerLines + $entries) -Encoding utf8
