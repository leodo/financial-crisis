param(
    [Parameter(Mandatory = $true)]
    [string]$BaselineReleaseId,
    [Parameter(Mandatory = $true)]
    [string]$CandidateReleaseId,
    [string]$MarketScope = "financial_system",
    [string]$ScenarioId = "us_regional_banks_2023",
    [int]$TopCount = 14,
    [string]$OutputDir = "artifacts/research/separation-audit",
    [switch]$ForceRebuild
)

$ErrorActionPreference = "Stop"
if ($PSVersionTable.PSVersion.Major -ge 7) {
    $PSNativeCommandUseErrorActionPreference = $false
}

$Root = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location -LiteralPath $Root

function Get-CompareStem {
    param(
        [string]$From,
        [string]$To,
        [string]$Scenario
    )

    $stem = "$BaselineReleaseId-vs-$CandidateReleaseId-$From-$To-formal-probability-compare"
    if (-not [string]::IsNullOrWhiteSpace($Scenario)) {
        $stem = "$stem-$Scenario"
    }
    return $stem
}

function Find-ComparePath {
    param(
        [string]$From,
        [string]$To,
        [string]$Scenario
    )

    $fileName = "$(Get-CompareStem -From $From -To $To -Scenario $Scenario).json"
    $compareDirectories = @(
        "artifacts/research/formal-probability-compares",
        "artifacts/research/spa/cmp"
    )
    $matches = @()
    foreach ($relativeDirectory in $compareDirectories) {
        $compareDirectory = Join-Path $Root $relativeDirectory
        if (-not (Test-Path -LiteralPath $compareDirectory)) {
            continue
        }

        $path = Join-Path $compareDirectory $fileName
        if (Test-Path -LiteralPath $path) {
            $matches += Get-Item -LiteralPath $path
        }
    }

    return ($matches | Sort-Object LastWriteTime -Descending | Select-Object -First 1).FullName
}

function Invoke-FormalCompare {
    param(
        [string]$From,
        [string]$To,
        [string]$Scenario
    )

    $args = @(
        "run", "-p", "fc-worker", "--",
        "research", "release", "formal-probability-compare",
        "--market-scope", $MarketScope,
        "--baseline-release-id", $BaselineReleaseId,
        "--candidate-release-id", $CandidateReleaseId,
        "--from", $From,
        "--to", $To
    )

    if (-not [string]::IsNullOrWhiteSpace($Scenario)) {
        $args += @("--scenario-id", $Scenario)
    }

    & cargo @args
    if ($LASTEXITCODE -ne 0) {
        throw "formal-probability-compare failed for $From -> $To"
    }
}

function Resolve-ComparePath {
    param(
        [string]$From,
        [string]$To,
        [string]$Scenario
    )

    $path = Find-ComparePath -From $From -To $To -Scenario $Scenario
    if ($ForceRebuild -or -not $path) {
        Invoke-FormalCompare -From $From -To $To -Scenario $Scenario
        $path = Find-ComparePath -From $From -To $To -Scenario $Scenario
    }
    if (-not $path) {
        throw "Could not find compare artifact for $From -> $To scenario=$Scenario"
    }
    return $path
}

Write-Host "Formal candidate separation audit"
Write-Host "  baseline : $BaselineReleaseId"
Write-Host "  candidate: $CandidateReleaseId"
Write-Host "  scope    : $MarketScope"
Write-Host ""

$windows = @(
    [pscustomobject]@{
        id = "regional_positive"
        role = "true_positive"
        label = "Regional banks positive-window"
        from = "2022-12-01"
        to = "2023-03-15"
        scenario = $ScenarioId
        regime_20d = "positive_window"
    },
    [pscustomobject]@{
        id = "february_false_positive"
        role = "false_positive_pressure"
        label = "February pre-warning false-positive window"
        from = "2023-02-01"
        to = "2023-02-15"
        scenario = ""
        regime_20d = ""
    },
    [pscustomobject]@{
        id = "july_false_positive"
        role = "false_positive_pressure"
        label = "July normal false-positive window"
        from = "2023-07-01"
        to = "2023-07-20"
        scenario = ""
        regime_20d = ""
    }
)

$windowSpecs = foreach ($window in $windows) {
    $path = Resolve-ComparePath -From $window.from -To $window.to -Scenario $window.scenario
    [pscustomobject]@{
        id = $window.id
        role = $window.role
        label = $window.label
        from = $window.from
        to = $window.to
        scenario = $window.scenario
        regime_20d = $window.regime_20d
        path = $path
    }
}

$outputDirectory = Join-Path $Root $OutputDir
New-Item -ItemType Directory -Path $outputDirectory -Force | Out-Null

$windowJson = $windowSpecs | ConvertTo-Json -Depth 5 -Compress
$windowJsonBase64 = [Convert]::ToBase64String([System.Text.Encoding]::UTF8.GetBytes($windowJson))

$python = @'
import base64
import json
import os
import sys
from collections import defaultdict
from datetime import datetime, timezone

windows = json.loads(base64.b64decode(sys.argv[1]).decode("utf-8"))
output_dir = sys.argv[2]
top_count = int(sys.argv[3])

def avg(values):
    values = [float(value) for value in values if value is not None]
    if not values:
        return None
    return sum(values) / len(values)

def rate(values):
    values = list(values)
    if not values:
        return None
    return sum(1 for value in values if value) / len(values)

def round6(value):
    if value is None:
        return None
    return round(float(value), 6)

def pct(value):
    if value is None:
        return "-"
    return f"{float(value) * 100:.2f}%"

def select_rows(doc, regime):
    rows = doc.get("rows") or []
    if regime:
        rows = [row for row in rows if row.get("regime_20d") == regime]
    return rows

def threshold(doc, kind):
    for row in doc.get(f"{kind}_thresholds") or []:
        if int(row.get("horizon_days", -1)) == 20:
            return row.get("decision_threshold")
    return None

def feature_stats(rows):
    by_name = defaultdict(lambda: {
        "observed_count": 0,
        "baseline_contribution": [],
        "candidate_contribution": [],
        "delta_contribution": [],
        "baseline_weight": [],
        "candidate_weight": [],
    })
    for row in rows:
        for feature in row.get("top_feature_deltas_20d") or []:
            name = feature.get("name") or "unknown"
            item = by_name[name]
            item["observed_count"] += 1
            for key in [
                "baseline_contribution",
                "candidate_contribution",
                "delta_contribution",
                "baseline_weight",
                "candidate_weight",
            ]:
                item[key].append(float(feature.get(key) or 0.0))

    result = {}
    row_count = max(1, len(rows))
    for name, values in by_name.items():
        result[name] = {
            "name": name,
            "observed_count": values["observed_count"],
            "coverage_ratio": round6(values["observed_count"] / row_count),
            "mean_baseline_contribution": round6(avg(values["baseline_contribution"])),
            "mean_candidate_contribution": round6(avg(values["candidate_contribution"])),
            "mean_delta_contribution": round6(avg(values["delta_contribution"])),
            "mean_baseline_weight": round6(avg(values["baseline_weight"])),
            "mean_candidate_weight": round6(avg(values["candidate_weight"])),
        }
    return result

window_reports = []
feature_by_window = {}

for spec in windows:
    with open(spec["path"], "r", encoding="utf-8") as handle:
        doc = json.load(handle)
    rows = select_rows(doc, spec.get("regime_20d"))
    feature_map = feature_stats(rows)
    feature_by_window[spec["id"]] = feature_map
    baseline_values = [row.get("baseline_final_p_20d") for row in rows]
    candidate_values = [row.get("candidate_final_p_20d") for row in rows]
    delta_values = [row.get("delta_final_p_20d") for row in rows]

    window_reports.append({
        "id": spec["id"],
        "role": spec["role"],
        "label": spec["label"],
        "from_date": spec["from"],
        "to_date": spec["to"],
        "scenario_id": spec.get("scenario") or None,
        "regime_filter_20d": spec.get("regime_20d") or None,
        "source_compare_path": spec["path"],
        "row_count": len(rows),
        "baseline_decision_threshold": round6(threshold(doc, "baseline")),
        "candidate_decision_threshold": round6(threshold(doc, "candidate")),
        "baseline_avg_p20d": round6(avg(baseline_values)),
        "candidate_avg_p20d": round6(avg(candidate_values)),
        "delta_avg_p20d": round6(avg(delta_values)),
        "baseline_max_p20d": round6(max([float(value) for value in baseline_values], default=0.0)),
        "candidate_max_p20d": round6(max([float(value) for value in candidate_values], default=0.0)),
        "baseline_hit_rate_20d": round6(rate(row.get("baseline_hit_20d") for row in rows)),
        "candidate_hit_rate_20d": round6(rate(row.get("candidate_hit_20d") for row in rows)),
        "top_candidate_contributors": sorted(
            feature_map.values(),
            key=lambda item: abs(item.get("mean_candidate_contribution") or 0.0),
            reverse=True,
        )[:top_count],
        "top_delta_contributors": sorted(
            feature_map.values(),
            key=lambda item: abs(item.get("mean_delta_contribution") or 0.0),
            reverse=True,
        )[:top_count],
    })

all_feature_names = sorted({
    name
    for feature_map in feature_by_window.values()
    for name in feature_map.keys()
})

def metric(feature_name, window_id, key):
    return (feature_by_window.get(window_id, {}).get(feature_name) or {}).get(key)

def classify_feature(row):
    regional_delta = row["regional_delta_contribution"] or 0.0
    false_avg = row["false_positive_avg_delta_contribution"] or 0.0
    regional_candidate = row["regional_candidate_contribution"] or 0.0
    false_candidate = row["false_positive_avg_candidate_contribution"] or 0.0
    false_ratio = row["false_to_regional_delta_ratio"]

    if false_avg >= 0.25 and regional_delta <= 0.10:
        return "false_positive_only_lift"
    if (
        regional_delta > 0.0
        and false_avg >= 0.25
        and false_ratio is not None
        and false_ratio >= 0.55
    ):
        return "false_positive_coupled_lift"
    if regional_delta >= 0.50 and false_avg <= max(0.15, regional_delta * 0.30):
        return "regional_preferential_lift"
    if regional_candidate <= -0.25 and false_candidate > regional_candidate:
        return "regional_suppression"
    return "mixed_or_low_signal"

feature_rows = []
for name in all_feature_names:
    regional_delta = metric(name, "regional_positive", "mean_delta_contribution")
    feb_delta = metric(name, "february_false_positive", "mean_delta_contribution")
    july_delta = metric(name, "july_false_positive", "mean_delta_contribution")
    false_deltas = [value for value in [feb_delta, july_delta] if value is not None]
    false_avg_delta = avg(false_deltas)
    regional_candidate = metric(name, "regional_positive", "mean_candidate_contribution")
    feb_candidate = metric(name, "february_false_positive", "mean_candidate_contribution")
    july_candidate = metric(name, "july_false_positive", "mean_candidate_contribution")
    false_candidates = [value for value in [feb_candidate, july_candidate] if value is not None]
    false_avg_candidate = avg(false_candidates)
    false_to_regional_delta_ratio = None
    if regional_delta is not None and regional_delta > 0 and false_avg_delta is not None:
        false_to_regional_delta_ratio = false_avg_delta / regional_delta

    row = {
        "name": name,
        "regional_delta_contribution": round6(regional_delta),
        "february_false_delta_contribution": round6(feb_delta),
        "july_false_delta_contribution": round6(july_delta),
        "false_positive_avg_delta_contribution": round6(false_avg_delta),
        "false_to_regional_delta_ratio": round6(false_to_regional_delta_ratio),
        "regional_candidate_contribution": round6(regional_candidate),
        "february_false_candidate_contribution": round6(feb_candidate),
        "july_false_candidate_contribution": round6(july_candidate),
        "false_positive_avg_candidate_contribution": round6(false_avg_candidate),
        "regional_coverage_ratio": metric(name, "regional_positive", "coverage_ratio"),
        "february_coverage_ratio": metric(name, "february_false_positive", "coverage_ratio"),
        "july_coverage_ratio": metric(name, "july_false_positive", "coverage_ratio"),
    }
    row["classification"] = classify_feature(row)
    row["risk_score"] = round6(
        max(0.0, false_avg_delta or 0.0)
        + max(0.0, (false_to_regional_delta_ratio or 0.0) - 0.55)
        - max(0.0, regional_delta or 0.0) * 0.05
    )
    feature_rows.append(row)

feature_rows.sort(
    key=lambda row: (
        0 if row["classification"] in {
            "false_positive_only_lift",
            "false_positive_coupled_lift",
        } else 1,
        -(row["risk_score"] or 0.0),
        row["name"],
    )
)

takeaways = []
regional_window = next((row for row in window_reports if row["id"] == "regional_positive"), None)
false_windows = [row for row in window_reports if row["role"] == "false_positive_pressure"]
if regional_window:
    candidate_threshold = regional_window["candidate_decision_threshold"]
    regional_avg = regional_window["candidate_avg_p20d"]
    if candidate_threshold is not None and regional_avg is not None and regional_avg < candidate_threshold:
        takeaways.append(
            "Regional positive-window candidate avg p20d "
            f"{pct(regional_avg)} remains below candidate threshold {pct(candidate_threshold)}; "
            "threshold remains binding even when the true-positive window is lifted."
        )
    for false_window in false_windows:
        false_max = false_window["candidate_max_p20d"]
        if false_max is not None and regional_avg is not None and false_max >= regional_avg * 0.90:
            takeaways.append(
                f"{false_window['label']} candidate max p20d {pct(false_max)} is close to or above "
                f"regional positive-window avg {pct(regional_avg)}; lowering the 20d threshold alone can reintroduce false positives."
            )

for row in feature_rows[:top_count]:
    if row["classification"] == "false_positive_coupled_lift":
        takeaways.append(
            f"{row['name']} lifts false-positive windows almost as much as the regional positive-window "
            f"(false/regional delta ratio {row['false_to_regional_delta_ratio']}); inspect gating or context constraints before retraining."
        )
    elif row["classification"] == "false_positive_only_lift":
        takeaways.append(
            f"{row['name']} mainly lifts false-positive windows without comparable regional positive-window lift; consider suppressing or gating this path."
        )

if not takeaways:
    takeaways.append(
        "No dominant cross-window false-positive feature lift was found in top_feature_deltas; inspect full contribution exports or runtime review next."
    )

audit = {
    "generated_at": datetime.now(timezone.utc).isoformat(),
    "baseline_release_id": windows[0].get("baseline_release_id"),
    "candidate_release_id": windows[0].get("candidate_release_id"),
    "window_reports": window_reports,
    "cross_window_feature_rows": feature_rows,
    "top_false_positive_coupled_features": [
        row for row in feature_rows
        if row["classification"] in {"false_positive_only_lift", "false_positive_coupled_lift"}
    ][:top_count],
    "methodology_limitations": [
        "This audit is based on formal-probability-compare top_feature_deltas_20d, not the full contribution vector.",
        "February and July windows are diagnostic false-positive pressure windows, not formal crisis labels.",
        "Use this report to decide what to test next; release activation still requires candidate-screen and release-review gates.",
    ],
    "takeaways": takeaways,
}

baseline = None
candidate = None
for spec in windows:
    with open(spec["path"], "r", encoding="utf-8") as handle:
        doc = json.load(handle)
    baseline = baseline or doc.get("baseline_release_id")
    candidate = candidate or doc.get("candidate_release_id")
audit["baseline_release_id"] = baseline
audit["candidate_release_id"] = candidate

stem = f"{baseline}-vs-{candidate}-20d-separation-audit"
safe_stem = "".join(ch if ch.isalnum() or ch in "._-" else "_" for ch in stem)
output_path = os.path.join(output_dir, f"{safe_stem}.json")
with open(output_path, "w", encoding="utf-8") as handle:
    json.dump(audit, handle, ensure_ascii=False, indent=2)

print("Formal candidate separation audit")
print(f"  baseline : {baseline}")
print(f"  candidate: {candidate}")
print(f"  output   : {output_path}")
print("")
print("20d window probability summary")
print("  window                         rows  candidate avg  candidate max  threshold  hit")
for row in window_reports:
    print(
        "  {label:<30} {rows:>4}  {avg:>13}  {maxv:>13}  {thr:>9}  {hit:>6}".format(
            label=row["id"],
            rows=row["row_count"],
            avg=pct(row["candidate_avg_p20d"]),
            maxv=pct(row["candidate_max_p20d"]),
            thr=pct(row["candidate_decision_threshold"]),
            hit=pct(row["candidate_hit_rate_20d"]),
        )
    )
print("")
print("Top cross-window feature risks")
for row in audit["top_false_positive_coupled_features"][:top_count]:
    print(
        "  - {name}: {cls}; regional_delta={regional}, false_avg_delta={false}, false/regional={ratio}".format(
            name=row["name"],
            cls=row["classification"],
            regional=row["regional_delta_contribution"],
            false=row["false_positive_avg_delta_contribution"],
            ratio=row["false_to_regional_delta_ratio"],
        )
    )
print("")
print("Takeaways")
for takeaway in takeaways:
    print(f"  - {takeaway}")
'@

$python | python - $windowJsonBase64 $outputDirectory $TopCount
if ($LASTEXITCODE -ne 0) {
    throw "formal candidate separation audit failed"
}
