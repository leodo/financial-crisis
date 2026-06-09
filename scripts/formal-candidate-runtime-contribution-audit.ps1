param(
    [Parameter(Mandatory = $true)]
    [string]$BaselineReleaseId,
    [Parameter(Mandatory = $true)]
    [string]$CandidateReleaseId,
    [string]$MarketScope = "financial_system",
    [string]$From = "",
    [string]$To = "",
    [ValidateSet("default", "strict_rebuild")]
    [string]$HistoryMode = "default",
    [int]$HistoryLimit = 5000,
    [int[]]$HorizonDays = @(5, 20, 60),
    [int]$TopCount = 12,
    [string]$OutputDir = "artifacts/research/runtime-contribution-audit",
    [switch]$ForceRebuild
)

$ErrorActionPreference = "Stop"
if ($PSVersionTable.PSVersion.Major -ge 7) {
    $PSNativeCommandUseErrorActionPreference = $false
}

$Root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Set-Location -LiteralPath $Root

if ([string]::IsNullOrWhiteSpace($From) -or [string]::IsNullOrWhiteSpace($To)) {
    $current = Invoke-RestMethod "http://127.0.0.1:18080/api/assessment/current"
    if ([string]::IsNullOrWhiteSpace($From)) {
        $From = [string]$current.as_of_date
    }
    if ([string]::IsNullOrWhiteSpace($To)) {
        $To = [string]$current.as_of_date
    }
}

function Find-ProbabilitySlicePath {
    param([string]$ReleaseId)

    $fileName = "$ReleaseId-$From-$To-$HistoryMode-probability-slice.json"
    $path = Join-Path $Root "artifacts/research/release-probability-slices/$fileName"
    if (Test-Path -LiteralPath $path) {
        return (Resolve-Path -LiteralPath $path).Path
    }

    return $null
}

function Invoke-ProbabilitySlice {
    param([string]$ReleaseId)

    $args = @(
        "run", "-p", "fc-worker", "--",
        "research", "release", "probability-slice",
        "--release-id", $ReleaseId,
        "--market-scope", $MarketScope,
        "--from", $From,
        "--to", $To,
        "--history-mode", $HistoryMode,
        "--history-limit", ([string]$HistoryLimit)
    )

    $output = & cargo @args 2>&1
    $exitCode = $LASTEXITCODE
    foreach ($line in $output) {
        Write-Host $line
    }
    if ($exitCode -ne 0) {
        throw "release probability-slice failed for $ReleaseId"
    }
}

function Ensure-ProbabilitySlice {
    param([string]$ReleaseId)

    $path = Find-ProbabilitySlicePath -ReleaseId $ReleaseId
    if ($ForceRebuild -or -not $path) {
        Write-Host "Generating runtime probability slice for $ReleaseId"
        Invoke-ProbabilitySlice -ReleaseId $ReleaseId
        $path = Find-ProbabilitySlicePath -ReleaseId $ReleaseId
    }
    if (-not $path) {
        throw "Could not find runtime probability slice artifact for $ReleaseId."
    }
    return $path
}

$baselinePath = Ensure-ProbabilitySlice -ReleaseId $BaselineReleaseId
$candidatePath = Ensure-ProbabilitySlice -ReleaseId $CandidateReleaseId

$outputDirectory = Join-Path $Root $OutputDir
New-Item -ItemType Directory -Path $outputDirectory -Force | Out-Null
$horizonCsv = ($HorizonDays | ForEach-Object { [string]$_ }) -join ","

$python = @'
import json
import os
import sys
from collections import defaultdict
from datetime import datetime, timezone

root = sys.argv[1]
baseline_path = sys.argv[2]
candidate_path = sys.argv[3]
output_dir = sys.argv[4]
top_count = int(sys.argv[5])
horizons = [int(item) for item in sys.argv[6].split(",") if item]

with open(baseline_path, "r", encoding="utf-8") as handle:
    baseline = json.load(handle)
with open(candidate_path, "r", encoding="utf-8") as handle:
    candidate = json.load(handle)

def round6(value):
    if value is None:
        return None
    return round(float(value), 6)

def pct(value):
    if value is None:
        return "-"
    return f"{value * 100:.4f}%"

def avg(values):
    values = [value for value in values if value is not None]
    if not values:
        return None
    return sum(values) / len(values)

def evaluation_thresholds(release_id):
    candidates = [
        os.path.join(root, "artifacts", "research", "model-bundles", "generated", f"{release_id}-evaluation.json"),
        os.path.join(root, "config", "model-bundles", "generated", f"{release_id}-evaluation.json"),
    ]
    for path in candidates:
        if not os.path.exists(path):
            continue
        with open(path, "r", encoding="utf-8") as handle:
            doc = json.load(handle)
        thresholds = {}
        for horizon in doc.get("horizons") or []:
            thresholds[int(horizon.get("horizon_days"))] = horizon.get("decision_threshold")
        return thresholds, path
    return {}, None

def review_runtime_thresholds(baseline_release_id, candidate_release_id, history_mode):
    review_dir = os.path.join(root, "artifacts", "research", "release-review")
    if not os.path.isdir(review_dir):
        return {}, {}, None
    suffix = f"{baseline_release_id}-vs-{candidate_release_id}-{history_mode}-release-review.json"
    matches = [
        os.path.join(review_dir, name)
        for name in os.listdir(review_dir)
        if name.endswith(suffix)
    ]
    if not matches:
        return {}, {}, None
    matches.sort(key=lambda path: os.path.getmtime(path), reverse=True)
    path = matches[0]
    with open(path, "r", encoding="utf-8") as handle:
        doc = json.load(handle)
    baseline_thresholds = {}
    candidate_thresholds = {}
    for row in ((doc.get("comparison") or {}).get("runtime_separation_summary") or []):
        horizon = int(row.get("horizon_days", -1))
        if horizon <= 0:
            continue
        baseline_thresholds[horizon] = row.get("baseline_threshold")
        candidate_thresholds[horizon] = row.get("candidate_threshold")
    return baseline_thresholds, candidate_thresholds, path

def horizon_for(row, horizon_days):
    for horizon in (row.get("probability_diagnostics") or {}).get("horizon_overlays") or []:
        if int(horizon.get("horizon_days", -1)) == horizon_days:
            return horizon
    return None

def horizon_probability(row, horizon_days, kind):
    horizon = horizon_for(row, horizon_days)
    if kind == "raw":
        return (horizon or {}).get("raw_probability", row.get(f"raw_p_{horizon_days}d"))
    if kind == "calibrated":
        return (horizon or {}).get("calibrated_probability", row.get(f"calibrated_p_{horizon_days}d"))
    if kind == "runtime":
        return (horizon or {}).get(
            "runtime_final_probability",
            (horizon or {}).get("final_probability", row.get(f"calibrated_p_{horizon_days}d")),
        )
    return (horizon or {}).get("final_probability")

def base_contributions(row, horizon_days):
    horizon = horizon_for(row, horizon_days)
    return list((horizon or {}).get("base_contributions") or [])

def aggregate_feature_rows(rows, horizon_days):
    by_name = defaultdict(lambda: {
        "count": 0,
        "contribution": [],
        "weight": [],
        "raw_value": [],
        "normalized_value": [],
    })
    contributing_row_count = 0
    for row in rows:
        items = base_contributions(row, horizon_days)
        if items:
            contributing_row_count += 1
        for item in items:
            bucket = by_name[item.get("name", "unknown")]
            bucket["count"] += 1
            for key in ["contribution", "weight", "raw_value", "normalized_value"]:
                bucket[key].append(float(item.get(key) or 0.0))

    feature_rows = []
    for name, values in by_name.items():
        feature_rows.append({
            "name": name,
            "observed_count": values["count"],
            "row_coverage_ratio": round6(values["count"] / max(1, len(rows))),
            "mean_contribution": round6(avg(values["contribution"])),
            "mean_weight": round6(avg(values["weight"])),
            "mean_raw_value": round6(avg(values["raw_value"])),
            "mean_normalized_value": round6(avg(values["normalized_value"])),
        })
    return contributing_row_count, feature_rows

def feature_map(rows):
    return {row["name"]: row for row in rows}

def semantic_anomalies(feature_rows, horizon_days):
    anomalies = []
    by_name = feature_map(feature_rows)
    usdjpy_tail = by_name.get("tail_pos__us_usdjpy_level__145")
    if usdjpy_tail and (usdjpy_tail.get("mean_raw_value") or 0) > 0 and (usdjpy_tail.get("mean_contribution") or 0) < -1.0:
        anomalies.append({
            "code": "usdjpy_high_tail_negative",
            "horizon_days": horizon_days,
            "feature": "tail_pos__us_usdjpy_level__145",
            "mean_raw_value": usdjpy_tail.get("mean_raw_value"),
            "mean_contribution": usdjpy_tail.get("mean_contribution"),
            "message": "High USDJPY tail is strongly reducing runtime probability; treat current touchline distance as model-audit evidence, not proof of safety.",
        })
    usdjpy_change = by_name.get("us_usdjpy_change_20d")
    if usdjpy_change and (usdjpy_change.get("mean_raw_value") or 0) > 0 and (usdjpy_change.get("mean_contribution") or 0) < -0.25:
        anomalies.append({
            "code": "usdjpy_change_negative",
            "horizon_days": horizon_days,
            "feature": "us_usdjpy_change_20d",
            "mean_raw_value": usdjpy_change.get("mean_raw_value"),
            "mean_contribution": usdjpy_change.get("mean_contribution"),
            "message": "Positive 20d USDJPY change is reducing runtime probability in this window.",
        })
    return anomalies

def compare_feature_rows(baseline_rows, candidate_rows):
    baseline_map = feature_map(baseline_rows)
    candidate_map = feature_map(candidate_rows)
    names = sorted(set(baseline_map.keys()) | set(candidate_map.keys()))
    rows = []
    for name in names:
        left = baseline_map.get(name, {})
        right = candidate_map.get(name, {})
        baseline_contribution = left.get("mean_contribution")
        candidate_contribution = right.get("mean_contribution")
        rows.append({
            "name": name,
            "baseline_mean_contribution": baseline_contribution,
            "candidate_mean_contribution": candidate_contribution,
            "delta_mean_contribution": round6((candidate_contribution or 0.0) - (baseline_contribution or 0.0)),
            "baseline_observed_count": left.get("observed_count", 0),
            "candidate_observed_count": right.get("observed_count", 0),
        })
    rows.sort(key=lambda row: abs(row["delta_mean_contribution"] or 0.0), reverse=True)
    return rows

def threshold_state(probability, threshold):
    if probability is None or threshold is None or threshold <= 0:
        return "unconfigured"
    ratio = probability / threshold
    if probability >= threshold:
        return "above_floor"
    if ratio >= 0.8:
        return "near_floor"
    if ratio >= 0.2:
        return "building"
    return "cold"

def runtime_group_key(row, horizon_days, threshold):
    probability = horizon_probability(row, horizon_days, "runtime")
    bucket = row.get("time_to_risk_bucket") or "unknown_bucket"
    posture = row.get("posture") or "unknown_posture"
    state = threshold_state(probability, threshold)
    return f"bucket={bucket}|posture={posture}|floor={state}"

def build_date_rows(dates, horizon_days, baseline_rows_by_date, candidate_rows_by_date, baseline_threshold, candidate_threshold):
    rows = []
    for date in dates:
        baseline_row = baseline_rows_by_date[date]
        candidate_row = candidate_rows_by_date[date]
        baseline_probability = horizon_probability(baseline_row, horizon_days, "runtime")
        candidate_probability = horizon_probability(candidate_row, horizon_days, "runtime")
        rows.append({
            "as_of_date": date,
            "baseline_runtime_probability": round6(baseline_probability),
            "candidate_runtime_probability": round6(candidate_probability),
            "baseline_touchline_ratio": round6(None if not baseline_threshold else (baseline_probability or 0.0) / baseline_threshold),
            "candidate_touchline_ratio": round6(None if not candidate_threshold else (candidate_probability or 0.0) / candidate_threshold),
            "baseline_time_to_risk_bucket": baseline_row.get("time_to_risk_bucket"),
            "candidate_time_to_risk_bucket": candidate_row.get("time_to_risk_bucket"),
            "baseline_posture": baseline_row.get("posture"),
            "candidate_posture": candidate_row.get("posture"),
            "candidate_runtime_group": runtime_group_key(candidate_row, horizon_days, candidate_threshold),
        })
    return rows

def build_horizon_aggregate(label, dates, horizon_days, baseline_rows_by_date, candidate_rows_by_date, baseline_threshold, candidate_threshold):
    baseline_selected = [baseline_rows_by_date[date] for date in dates]
    candidate_selected = [candidate_rows_by_date[date] for date in dates]
    baseline_contributing_count, baseline_features = aggregate_feature_rows(baseline_selected, horizon_days)
    candidate_contributing_count, candidate_features = aggregate_feature_rows(candidate_selected, horizon_days)
    feature_deltas = compare_feature_rows(baseline_features, candidate_features)
    baseline_runtime = [horizon_probability(row, horizon_days, "runtime") for row in baseline_selected]
    candidate_runtime = [horizon_probability(row, horizon_days, "runtime") for row in candidate_selected]
    return {
        "label": label,
        "date_count": len(dates),
        "baseline_decision_threshold": round6(baseline_threshold),
        "candidate_decision_threshold": round6(candidate_threshold),
        "baseline_avg_runtime_probability": round6(avg(baseline_runtime)),
        "candidate_avg_runtime_probability": round6(avg(candidate_runtime)),
        "delta_avg_runtime_probability": round6((avg(candidate_runtime) or 0.0) - (avg(baseline_runtime) or 0.0)),
        "baseline_touchline_ratio": round6(None if not baseline_threshold else (avg(baseline_runtime) or 0.0) / baseline_threshold),
        "candidate_touchline_ratio": round6(None if not candidate_threshold else (avg(candidate_runtime) or 0.0) / candidate_threshold),
        "baseline_rows_with_base_contributions": baseline_contributing_count,
        "candidate_rows_with_base_contributions": candidate_contributing_count,
        "baseline_top_negative_base_contributions": sorted(baseline_features, key=lambda row: row.get("mean_contribution") or 0.0)[:top_count],
        "candidate_top_negative_base_contributions": sorted(candidate_features, key=lambda row: row.get("mean_contribution") or 0.0)[:top_count],
        "top_abs_delta_base_contributions": feature_deltas[:top_count],
        "baseline_semantic_anomalies": semantic_anomalies(baseline_features, horizon_days),
        "candidate_semantic_anomalies": semantic_anomalies(candidate_features, horizon_days),
    }

baseline_rows_by_date = {row.get("as_of_date"): row for row in baseline.get("rows") or []}
candidate_rows_by_date = {row.get("as_of_date"): row for row in candidate.get("rows") or []}
common_dates = sorted(set(baseline_rows_by_date.keys()) & set(candidate_rows_by_date.keys()))
baseline_thresholds, baseline_threshold_source = evaluation_thresholds(baseline.get("release_id"))
candidate_thresholds, candidate_threshold_source = evaluation_thresholds(candidate.get("release_id"))
review_baseline_thresholds, review_candidate_thresholds, review_threshold_source = review_runtime_thresholds(
    baseline.get("release_id"),
    candidate.get("release_id"),
    baseline.get("history_mode"),
)
baseline_thresholds = {**baseline_thresholds, **review_baseline_thresholds}
candidate_thresholds = {**candidate_thresholds, **review_candidate_thresholds}

audit = {
    "generated_at": datetime.now(timezone.utc).isoformat(),
    "market_scope": baseline.get("market_scope"),
    "history_mode": baseline.get("history_mode"),
    "from_date": baseline.get("from_date"),
    "to_date": baseline.get("to_date"),
    "baseline_release_id": baseline.get("release_id"),
    "candidate_release_id": candidate.get("release_id"),
    "baseline_slice_path": baseline_path,
    "candidate_slice_path": candidate_path,
    "baseline_replay_run_id": baseline.get("replay_run_id"),
    "candidate_replay_run_id": candidate.get("replay_run_id"),
    "baseline_threshold_source": baseline_threshold_source,
    "candidate_threshold_source": candidate_threshold_source,
    "runtime_threshold_source": review_threshold_source,
    "common_date_count": len(common_dates),
    "methodology_limitations": [
        "Runtime probability slices expose top base contributions per horizon, not a full feature vector.",
        "This audit compares dates present in both slices and is suitable for explaining current/runtime readings; release promotion still requires release-review guardrails.",
    ],
    "horizons": [],
    "takeaways": [],
}

for horizon in horizons:
    candidate_threshold = candidate_thresholds.get(horizon)
    baseline_threshold = baseline_thresholds.get(horizon)
    horizon_report = build_horizon_aggregate(
        "overall",
        common_dates,
        horizon,
        baseline_rows_by_date,
        candidate_rows_by_date,
        baseline_threshold,
        candidate_threshold,
    )
    horizon_report["horizon_days"] = horizon
    horizon_report["date_rows"] = build_date_rows(
        common_dates,
        horizon,
        baseline_rows_by_date,
        candidate_rows_by_date,
        baseline_threshold,
        candidate_threshold,
    )
    runtime_groups = defaultdict(list)
    for date in common_dates:
        runtime_groups[runtime_group_key(candidate_rows_by_date[date], horizon, candidate_threshold)].append(date)
    horizon_report["runtime_group_summaries"] = [
        {
            "group": group,
            **build_horizon_aggregate(
                group,
                dates,
                horizon,
                baseline_rows_by_date,
                candidate_rows_by_date,
                baseline_threshold,
                candidate_threshold,
            ),
        }
        for group, dates in sorted(runtime_groups.items(), key=lambda item: (-len(item[1]), item[0]))
    ]
    audit["horizons"].append(horizon_report)

    if horizon_report["baseline_rows_with_base_contributions"] == 0 or horizon_report["candidate_rows_with_base_contributions"] == 0:
        audit["takeaways"].append(
            f"{horizon}d runtime slice is missing base contributions for baseline or candidate; regenerate replay before using this horizon for attribution."
        )
    for anomaly in horizon_report["baseline_semantic_anomalies"]:
        audit["takeaways"].append(
            f"baseline {horizon}d {anomaly['code']}: {anomaly['feature']} contribution={anomaly['mean_contribution']} raw={anomaly['mean_raw_value']}"
        )
    for anomaly in horizon_report["candidate_semantic_anomalies"]:
        audit["takeaways"].append(
            f"candidate {horizon}d {anomaly['code']}: {anomaly['feature']} contribution={anomaly['mean_contribution']} raw={anomaly['mean_raw_value']}"
        )
    if candidate_threshold and horizon_report["candidate_avg_runtime_probability"] is not None and horizon_report["candidate_avg_runtime_probability"] < candidate_threshold:
        audit["takeaways"].append(
            f"candidate {horizon}d runtime avg {pct(horizon_report['candidate_avg_runtime_probability'])} is below threshold {pct(candidate_threshold)}; touchline ratio={horizon_report['candidate_touchline_ratio']}"
        )

stem = "-".join([
    audit["baseline_release_id"],
    "vs",
    audit["candidate_release_id"],
    str(audit["from_date"]),
    str(audit["to_date"]),
    str(audit["history_mode"]),
    "runtime-contribution-audit",
])
output_path = os.path.join(output_dir, stem + ".json")
with open(output_path, "w", encoding="utf-8") as handle:
    json.dump(audit, handle, ensure_ascii=False, indent=2)

print("Formal candidate runtime contribution audit")
print(f"  baseline : {audit['baseline_release_id']} ({audit['baseline_replay_run_id']})")
print(f"  candidate: {audit['candidate_release_id']} ({audit['candidate_replay_run_id']})")
print(f"  range    : {audit['from_date']} -> {audit['to_date']} mode={audit['history_mode']}")
print(f"  dates    : {audit['common_date_count']}")
print(f"  output   : {output_path}")
print("")
for horizon in audit["horizons"]:
    print(f"{horizon['horizon_days']}d runtime summary")
    print(f"  baseline avg={pct(horizon['baseline_avg_runtime_probability'])} threshold={pct(horizon['baseline_decision_threshold'])} touch={horizon['baseline_touchline_ratio']}")
    print(f"  candidate avg={pct(horizon['candidate_avg_runtime_probability'])} threshold={pct(horizon['candidate_decision_threshold'])} touch={horizon['candidate_touchline_ratio']}")
    print(f"  base contribution rows baseline={horizon['baseline_rows_with_base_contributions']} candidate={horizon['candidate_rows_with_base_contributions']}")
    print("  baseline semantic anomalies:")
    if horizon["baseline_semantic_anomalies"]:
        for anomaly in horizon["baseline_semantic_anomalies"]:
            print(f"    - {anomaly['code']} {anomaly['feature']} contribution={anomaly['mean_contribution']} raw={anomaly['mean_raw_value']}")
    else:
        print("    - none")
    print("  candidate semantic anomalies:")
    if horizon["candidate_semantic_anomalies"]:
        for anomaly in horizon["candidate_semantic_anomalies"]:
            print(f"    - {anomaly['code']} {anomaly['feature']} contribution={anomaly['mean_contribution']} raw={anomaly['mean_raw_value']}")
    else:
        print("    - none")
    print("  top candidate-negative feature deltas:")
    for item in horizon["top_abs_delta_base_contributions"][:5]:
        print(f"    - {item['name']}: baseline={item['baseline_mean_contribution']} candidate={item['candidate_mean_contribution']} delta={item['delta_mean_contribution']}")
    print("  candidate runtime groups:")
    for group in horizon.get("runtime_group_summaries", [])[:6]:
        print(
            f"    - {group['group']}: dates={group['date_count']} "
            f"candidate_avg={pct(group['candidate_avg_runtime_probability'])} "
            f"touch={group['candidate_touchline_ratio']}"
        )
    print("")
print("Takeaways")
for takeaway in audit["takeaways"]:
    print(f"  - {takeaway}")
'@

$python | python - $Root $baselinePath $candidatePath $outputDirectory $TopCount $horizonCsv
if ($LASTEXITCODE -ne 0) {
    throw "runtime contribution audit failed"
}
