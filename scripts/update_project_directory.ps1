Param(
    [string]$Root = (Get-Location).Path,
    [string]$Output = "project_directory.md",
    [string]$MapFile = "project_directory_map.toml",
    [string[]]$Roots = @("crates"),
    [string]$SourceGlob = "/src/",
    [switch]$UpdateTouchedOnly
)

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
        if ($trim -match '^(?<key>[^=]+?)\\s*=\\s*\"(?<value>.*)\"\\s*$') {
            $key = $Matches['key'].Trim()
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
        if ($line -match '^##\\s+(.+)$') {
            $current = $Matches[1].Trim()
            continue
        }
        if ($current -and $line -match '^Description:\\s*(.+)$') {
            $map[$current] = $Matches[1].Trim()
        }
    }
    return $map
}

function Get-RustFunctions {
    Param([string]$Path)
    $funcs = New-Object System.Collections.Generic.List[string]
    foreach ($line in Get-Content $Path) {
        if ($line -match '^\\s*(pub(\\([^)]*\\))?\\s+)?(async\\s+)?fn\\s+([A-Za-z0-9_]+)') {
            $funcs.Add($Matches[4])
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
        if ($SourceGlob -and ($item.FullName -notmatch [Regex]::Escape($SourceGlob))) {
            continue
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

function Normalize-RelPath {
    Param([string]$Path)
    $rel = Resolve-Path -Relative $Path
    if ($rel.StartsWith('.\\') -or $rel.StartsWith('./')) {
        $rel = $rel.Substring(2)
    }
    return ($rel -replace '\\\\', '/')
}

$entries = @()
foreach ($file in ($files | Sort-Object FullName)) {
    $rel = Normalize-RelPath -Path $file.FullName
    if ($UpdateTouchedOnly -and $touched.Count -gt 0) {
        if ($touched -notcontains $rel) {
            continue
        }
    }
    $desc = $descMap[$rel]
    if (-not $desc) {
        $desc = $existingMap[$rel]
    }
    if (-not $desc) {
        $desc = "TODO: add description."
    }
    $funcs = Get-RustFunctions -Path $file.FullName
    if ($funcs.Count -gt 0) {
        $funcText = ($funcs | ForEach-Object { "``$($_)``" }) -join ', '
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
        if ($line -match '^##\\s+(.+)$') {
            if ($currentName) {
                $existing[$currentName] = $block.ToArray()
            }
            $currentName = $Matches[1].Trim()
            $block = New-Object System.Collections.Generic.List[string]
            $block.Add($line)
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
        $name = $header.Substring(3).Trim()
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
