param(
    [Parameter(Mandatory = $true)]
    [string]$BaselineReleaseId,
    [Parameter(Mandatory = $true)]
    [string]$CandidateReleaseId,
    [string]$MarketScope = "financial_system",
    [string]$ScenarioId = "us_regional_banks_2023",
    [string]$From = "2023-02-01",
    [string]$To = "2023-05-15",
    [int[]]$HorizonDays = @(20, 60),
    [int]$TopCount = 12,
    [string]$OutputDir = "artifacts/research/regime-contribution-audit",
    [switch]$ForceRebuild
)

$ErrorActionPreference = "Stop"
if ($PSVersionTable.PSVersion.Major -ge 7) {
    $PSNativeCommandUseErrorActionPreference = $false
}

$Root = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location -LiteralPath $Root

function Find-ComparePath {
    $stem = "$BaselineReleaseId-vs-$CandidateReleaseId-$From-$To-formal-probability-compare"
    if (-not [string]::IsNullOrWhiteSpace($ScenarioId)) {
        $stem = "$stem-$ScenarioId"
    }
    $fileName = "$stem.json"
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

    $matches = $matches | Sort-Object LastWriteTime -Descending

    $match = $matches | Select-Object -First 1
    if ($match) {
        return $match.FullName
    }

    return $null
}

function Invoke-FormalCompare {
    $args = @(
        "run", "-p", "fc-worker", "--",
        "research", "release", "formal-probability-compare",
        "--market-scope", $MarketScope,
        "--baseline-release-id", $BaselineReleaseId,
        "--candidate-release-id", $CandidateReleaseId,
        "--from", $From,
        "--to", $To
    )

    if (-not [string]::IsNullOrWhiteSpace($ScenarioId)) {
        $args += @("--scenario-id", $ScenarioId)
    }

    & cargo @args
    if ($LASTEXITCODE -ne 0) {
        throw "formal-probability-compare failed for $BaselineReleaseId vs $CandidateReleaseId"
    }
}

$comparePath = Find-ComparePath
if ($ForceRebuild -or -not $comparePath) {
    Write-Host "Generating formal-probability-compare artifact"
    Invoke-FormalCompare
    $comparePath = Find-ComparePath
}

if (-not $comparePath) {
    throw "Could not find formal-probability-compare artifact after generation."
}

$outputDirectory = Join-Path $Root $OutputDir
New-Item -ItemType Directory -Path $outputDirectory -Force | Out-Null

$horizonCsv = ($HorizonDays | ForEach-Object { [string]$_ }) -join ","

$python = @'
import json
import os
import sys
from collections import defaultdict
from datetime import datetime, timezone

compare_path = sys.argv[1]
output_dir = sys.argv[2]
top_count = int(sys.argv[3])
horizons = [int(item) for item in sys.argv[4].split(",") if item]

with open(compare_path, "r", encoding="utf-8") as handle:
    doc = json.load(handle)

rows = doc.get("rows") or []
regime_order = [
    "normal",
    "pre_warning_buffer",
    "positive_window",
    "in_crisis",
    "post_crisis_cooldown",
]

def threshold_by_horizon(kind, horizon):
    for row in doc.get(f"{kind}_thresholds") or []:
        if int(row.get("horizon_days", -1)) == horizon:
            return row.get("decision_threshold")
    return None

def avg(values):
    values = [value for value in values if value is not None]
    if not values:
        return None
    return sum(values) / len(values)

def rate(values):
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
    return f"{value * 100:.2f}%"

def contribution_stats(feature_rows, row_count):
    by_name = defaultdict(lambda: {
        "count": 0,
        "baseline_contribution": [],
        "candidate_contribution": [],
        "delta_contribution": [],
        "baseline_weight": [],
        "candidate_weight": [],
    })
    for feature in feature_rows:
        item = by_name[feature.get("name", "unknown")]
        item["count"] += 1
        for key in [
            "baseline_contribution",
            "candidate_contribution",
            "delta_contribution",
            "baseline_weight",
            "candidate_weight",
        ]:
            item[key].append(float(feature.get(key) or 0.0))

    result = []
    for name, values in by_name.items():
        result.append({
            "name": name,
            "observed_count": values["count"],
            "coverage_ratio": round6(values["count"] / max(1, row_count)),
            "mean_baseline_contribution": round6(avg(values["baseline_contribution"])),
            "mean_candidate_contribution": round6(avg(values["candidate_contribution"])),
            "mean_delta_contribution": round6(avg(values["delta_contribution"])),
            "mean_baseline_weight": round6(avg(values["baseline_weight"])),
            "mean_candidate_weight": round6(avg(values["candidate_weight"])),
        })
    return result

def collect_regime_rows(horizon):
    key = f"regime_{horizon}d"
    seen = sorted({row.get(key) for row in rows if row.get(key)})
    return [regime for regime in regime_order if regime in seen] + [
        regime for regime in seen if regime not in regime_order
    ]

def gap_row(horizon, feature_name, stats_by_regime):
    def candidate(regime):
        return (stats_by_regime.get(regime, {}).get(feature_name) or {}).get(
            "mean_candidate_contribution"
        )

    positive = candidate("positive_window")
    cooldown = candidate("post_crisis_cooldown")
    normal = candidate("normal")
    return {
        "name": feature_name,
        "positive_window_candidate_contribution": positive,
        "cooldown_candidate_contribution": cooldown,
        "normal_candidate_contribution": normal,
        "positive_minus_cooldown": round6(
            None if positive is None or cooldown is None else positive - cooldown
        ),
        "positive_minus_normal": round6(
            None if positive is None or normal is None else positive - normal
        ),
        "horizon_days": horizon,
    }

audit = {
    "generated_at": datetime.now(timezone.utc).isoformat(),
    "source_compare_path": compare_path,
    "market_scope": doc.get("market_scope"),
    "baseline_release_id": doc.get("baseline_release_id"),
    "candidate_release_id": doc.get("candidate_release_id"),
    "dataset_key": doc.get("dataset_key"),
    "scenario_id": doc.get("scenario_id"),
    "from_date": doc.get("from_date"),
    "to_date": doc.get("to_date"),
    "row_count": len(rows),
    "methodology_limitations": [
        "This audit is built from formal-probability-compare top_feature_deltas, currently top 8 absolute deltas per row, not the full feature contribution vector.",
        "Use this report for triage of suspicious regime separation and train/runtime transfer issues; final release decisions still require release-review guardrails.",
    ],
    "horizons": [],
    "takeaways": [],
}

for horizon in horizons:
    regimes = collect_regime_rows(horizon)
    horizon_report = {
        "horizon_days": horizon,
        "baseline_decision_threshold": round6(threshold_by_horizon("baseline", horizon)),
        "candidate_decision_threshold": round6(threshold_by_horizon("candidate", horizon)),
        "regime_summary": [],
        "top_features_by_regime": [],
        "candidate_feature_gap_risks": [],
    }
    stats_by_regime = {}
    probability_by_regime = {}

    for regime in regimes:
        selected = [row for row in rows if row.get(f"regime_{horizon}d") == regime]
        if not selected:
            continue

        baseline_values = [float(row.get(f"baseline_final_p_{horizon}d") or 0.0) for row in selected]
        candidate_values = [float(row.get(f"candidate_final_p_{horizon}d") or 0.0) for row in selected]
        deltas = [float(row.get(f"delta_final_p_{horizon}d") or 0.0) for row in selected]
        baseline_linear = [float(row.get(f"baseline_base_linear_{horizon}d") or 0.0) for row in selected]
        candidate_linear = [float(row.get(f"candidate_base_linear_{horizon}d") or 0.0) for row in selected]
        baseline_hits = [bool(row.get(f"baseline_hit_{horizon}d")) for row in selected]
        candidate_hits = [bool(row.get(f"candidate_hit_{horizon}d")) for row in selected]

        probability_by_regime[regime] = {
            "baseline_avg_probability": avg(baseline_values),
            "candidate_avg_probability": avg(candidate_values),
        }
        horizon_report["regime_summary"].append({
            "regime": regime,
            "row_count": len(selected),
            "baseline_avg_probability": round6(avg(baseline_values)),
            "candidate_avg_probability": round6(avg(candidate_values)),
            "delta_avg_probability": round6(avg(deltas)),
            "baseline_hit_rate": round6(rate(baseline_hits)),
            "candidate_hit_rate": round6(rate(candidate_hits)),
            "baseline_avg_base_linear": round6(avg(baseline_linear)),
            "candidate_avg_base_linear": round6(avg(candidate_linear)),
        })

        feature_rows = []
        for row in selected:
            feature_rows.extend(row.get(f"top_feature_deltas_{horizon}d") or [])
        stats = contribution_stats(feature_rows, len(selected))
        stats_by_regime[regime] = {item["name"]: item for item in stats}
        stats.sort(
            key=lambda item: abs(item["mean_candidate_contribution"] or 0.0),
            reverse=True,
        )
        horizon_report["top_features_by_regime"].append({
            "regime": regime,
            "feature_count": len(stats),
            "top_features": stats[:top_count],
        })

    all_feature_names = sorted({
        feature_name
        for feature_map in stats_by_regime.values()
        for feature_name in feature_map.keys()
    })
    gap_rows = [gap_row(horizon, feature_name, stats_by_regime) for feature_name in all_feature_names]
    gap_rows = [
        item for item in gap_rows
        if item["positive_minus_cooldown"] is not None or item["positive_minus_normal"] is not None
    ]
    gap_rows.sort(
        key=lambda item: min(
            value for value in [
                item["positive_minus_cooldown"],
                item["positive_minus_normal"],
            ]
            if value is not None
        )
    )
    horizon_report["candidate_feature_gap_risks"] = gap_rows[:top_count]

    positive = probability_by_regime.get("positive_window", {}).get("candidate_avg_probability")
    cooldown = probability_by_regime.get("post_crisis_cooldown", {}).get("candidate_avg_probability")
    normal = probability_by_regime.get("normal", {}).get("candidate_avg_probability")
    baseline_positive = probability_by_regime.get("positive_window", {}).get("baseline_avg_probability")
    candidate_threshold = threshold_by_horizon("candidate", horizon)
    if positive is not None and cooldown is not None and positive <= cooldown:
        audit["takeaways"].append(
            f"{horizon}d candidate positive-window avg {pct(positive)} is not above cooldown {pct(cooldown)}; this points to cooldown bleed."
        )
    if positive is not None and normal is not None and positive <= normal:
        audit["takeaways"].append(
            f"{horizon}d candidate positive-window avg {pct(positive)} is not above normal {pct(normal)}; this points to cold or misordered regime separation."
        )
    if positive is not None and baseline_positive is not None and baseline_positive > 0 and positive < baseline_positive * 0.75:
        audit["takeaways"].append(
            f"{horizon}d candidate retains only {positive / baseline_positive * 100:.1f}% of baseline positive-window avg probability."
        )
    if positive is not None and candidate_threshold is not None and positive < candidate_threshold:
        audit["takeaways"].append(
            f"{horizon}d candidate positive-window avg {pct(positive)} remains below its decision threshold {pct(candidate_threshold)}; this can produce zero threshold hits even when probability looks high."
        )
    if normal is not None and candidate_threshold is not None and candidate_threshold > 0 and normal >= candidate_threshold * 0.5:
        audit["takeaways"].append(
            f"{horizon}d candidate normal avg {pct(normal)} already consumes {normal / candidate_threshold * 100:.1f}% of the decision threshold; lowering the threshold without better feature separation can reintroduce false positives."
        )
    available_regimes = set(probability_by_regime.keys())
    if "positive_window" in available_regimes and not (
        "normal" in available_regimes or "post_crisis_cooldown" in available_regimes
    ):
        audit["takeaways"].append(
            f"{horizon}d selected formal compare has positive-window rows but no normal/cooldown rows; use a separate false-positive window or runtime release-review evidence for background bleed."
        )
    if "positive_window" not in available_regimes and "normal" in available_regimes:
        audit["takeaways"].append(
            f"{horizon}d selected formal compare only has background normal rows; use it as a false-positive check, not as positive-window continuity evidence."
        )

    audit["horizons"].append(horizon_report)

if not audit["takeaways"]:
    audit["takeaways"].append(
        "No obvious candidate regime-order blocker was found in this formal compare artifact; continue with runtime release-review evidence."
    )

stem_parts = [
    doc.get("baseline_release_id", "baseline"),
    "vs",
    doc.get("candidate_release_id", "candidate"),
    str(doc.get("scenario_id") or "all"),
    str(doc.get("from_date")),
    str(doc.get("to_date")),
    "regime-contribution-audit",
]
stem = "-".join(part.replace(":", "_").replace("/", "_").replace("\\", "_") for part in stem_parts)
output_path = os.path.join(output_dir, f"{stem}.json")
with open(output_path, "w", encoding="utf-8") as handle:
    json.dump(audit, handle, ensure_ascii=False, indent=2)

print("Formal candidate regime contribution audit")
print(f"  baseline : {audit['baseline_release_id']}")
print(f"  candidate: {audit['candidate_release_id']}")
print(f"  scenario : {audit['scenario_id'] or 'all'}")
print(f"  rows     : {audit['row_count']}")
print(f"  source   : {compare_path}")
print(f"  output   : {output_path}")
print("")

for horizon in audit["horizons"]:
    print(f"{horizon['horizon_days']}d regime probability summary")
    print(
        "  thresholds baseline={base} candidate={cand}".format(
            base=pct(horizon["baseline_decision_threshold"]),
            cand=pct(horizon["candidate_decision_threshold"]),
        )
    )
    print("  regime                 rows  baseline   candidate  delta      hit(base/cand)")
    for row in horizon["regime_summary"]:
        print(
            "  {regime:<22} {row_count:>4}  {base:>9}  {cand:>9}  {delta:>8}  {bhit:>5}/{chit:<5}".format(
                regime=row["regime"],
                row_count=row["row_count"],
                base=pct(row["baseline_avg_probability"]),
                cand=pct(row["candidate_avg_probability"]),
                delta=pct(row["delta_avg_probability"]),
                bhit=pct(row["baseline_hit_rate"]),
                chit=pct(row["candidate_hit_rate"]),
            )
        )
    print("")
    print(f"{horizon['horizon_days']}d candidate feature gap risks")
    for item in horizon["candidate_feature_gap_risks"][: min(8, top_count)]:
        print(
            "  - {name}: pos-cooldown={pc}, pos-normal={pn}, pos={pos}, cooldown={cool}, normal={normal}".format(
                name=item["name"],
                pc=item["positive_minus_cooldown"],
                pn=item["positive_minus_normal"],
                pos=item["positive_window_candidate_contribution"],
                cool=item["cooldown_candidate_contribution"],
                normal=item["normal_candidate_contribution"],
            )
        )
    print("")

print("Takeaways")
for takeaway in audit["takeaways"]:
    print(f"  - {takeaway}")
'@

$python | python - $comparePath $outputDirectory $TopCount $horizonCsv
if ($LASTEXITCODE -ne 0) {
    throw "regime contribution audit failed"
}
