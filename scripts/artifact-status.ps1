$ErrorActionPreference = "Stop"

$VersionedArtifactRoots = @(
    "config/model-bundles/generated",
    "config/model-releases/generated",
    "reports/release-review",
    "reports/formal-dataset"
)

$IgnoredArtifactRoots = @(
    "artifacts/research"
)

function Invoke-GitLines {
    param([string[]]$Arguments)

    $output = & git @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "git $($Arguments -join ' ') failed"
    }

    @($output | Where-Object { $_ -and $_.Trim() -ne "" })
}

function Write-List {
    param(
        [string]$Title,
        [string[]]$Items
    )

    Write-Host $Title
    if ($Items.Count -eq 0) {
        Write-Host "  none"
        return
    }

    foreach ($item in $Items) {
        Write-Host "  $item"
    }
}

$stagedVersionedArtifacts = @()
$untrackedVersionedArtifacts = @()
$ignoredVersionedArtifacts = @()
foreach ($root in $VersionedArtifactRoots) {
    $stagedVersionedArtifacts += Invoke-GitLines @("diff", "--cached", "--name-only", "--", $root)
    $untrackedVersionedArtifacts += Invoke-GitLines @("ls-files", "--others", "--exclude-standard", "--", $root)
    $ignoredVersionedArtifacts += Invoke-GitLines @("ls-files", "--others", "-i", "--exclude-standard", "--", $root)
}

$ignoredResearchArtifacts = @()
foreach ($root in $IgnoredArtifactRoots) {
    $ignoredResearchArtifacts += Invoke-GitLines @("ls-files", "--others", "-i", "--exclude-standard", "--", $root)
}

Write-Host "Artifact status"
Write-Host "==============="
Write-List "Staged versioned artifacts:" $stagedVersionedArtifacts
Write-Host ""
Write-List "Untracked files in versioned artifact directories:" $untrackedVersionedArtifacts
Write-Host ""
Write-List "Ignored generated files in versioned artifact directories:" $ignoredVersionedArtifacts
Write-Host ""
Write-List "Ignored research artifacts:" $ignoredResearchArtifacts

if ($stagedVersionedArtifacts.Count -gt 0 -and $env:ALLOW_TRACKED_ARTIFACTS -ne "1") {
    Write-Host ""
    Write-Host "Refusing staged versioned artifacts without explicit curation."
    Write-Host "Before committing, document whether each artifact is a formal release artifact, baseline evidence, or a temporary research byproduct."
    Write-Host "If this is intentional, rerun with ALLOW_TRACKED_ARTIFACTS=1 after the curation note is in the commit."
    exit 1
}
