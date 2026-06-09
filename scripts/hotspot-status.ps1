param(
    [int]$Top = 15,
    [int]$Threshold = 500
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Normalize-RepoPath {
    param([string]$Path)

    return $Path.Replace("\", "/").TrimStart("./")
}

$repoRoot = (& git rev-parse --show-toplevel).Trim()
if (-not $repoRoot) {
    throw "Unable to determine repository root."
}

Push-Location $repoRoot
try {
    $roots = @("apps", "crates", "scripts")
    $trackedExtensions = @(".rs", ".ts", ".tsx", ".js", ".jsx", ".css", ".ps1", ".toml")
    $excludeSegments = @("\node_modules\", "\target\", "\dist\", "\.git\")

    $files = Get-ChildItem -Recurse -File $roots | Where-Object {
        $fullName = $_.FullName
        $extension = $_.Extension.ToLowerInvariant()
        $trackedExtensions -contains $extension -and -not ($excludeSegments | Where-Object { $fullName.Contains($_) })
    }

    $rows = foreach ($file in $files) {
        $lineCount = (Get-Content $file.FullName | Measure-Object -Line).Lines
        [pscustomobject]@{
            Path  = Normalize-RepoPath ($file.FullName.Substring($repoRoot.Length + 1))
            Lines = $lineCount
        }
    }

    $sorted = $rows | Sort-Object Lines -Descending
    $hotspots = $sorted | Where-Object { $_.Lines -ge $Threshold } | Select-Object -First $Top
    if (-not $hotspots) {
        $hotspots = $sorted | Select-Object -First $Top
    }

    Write-Host "Code Hotspots"
    Write-Host "============="
    $hotspots | Format-Table -AutoSize

    $changedPaths = @(
        (& git diff --name-only --cached --diff-filter=ACMR)
        (& git diff --name-only --diff-filter=ACMR)
    ) | Where-Object { $_ } | ForEach-Object { Normalize-RepoPath $_ } | Sort-Object -Unique

    $hotspotIndex = @{}
    foreach ($hotspot in $hotspots) {
        $hotspotIndex[$hotspot.Path] = $hotspot
    }

    $touchedHotspots = foreach ($path in $changedPaths) {
        if ($hotspotIndex.ContainsKey($path)) {
            $hotspotIndex[$path]
        }
    }

    if ($touchedHotspots) {
        Write-Host ""
        Write-Host "Touched hotspot files"
        Write-Host "====================="
        $touchedHotspots | Sort-Object Lines -Descending | Format-Table -AutoSize

        if ($env:ALLOW_HOTSPOT_TOUCH -ne "1") {
            throw "Hotspot guard: touched top-size source files. Split the module first, or rerun with ALLOW_HOTSPOT_TOUCH=1 after documenting why the direct edit is still justified."
        }

        Write-Host ""
        Write-Host "ALLOW_HOTSPOT_TOUCH=1 detected; bypassing hotspot guard."
    } else {
        Write-Host ""
        Write-Host "Touched hotspot files: none"
    }
}
finally {
    Pop-Location
}
