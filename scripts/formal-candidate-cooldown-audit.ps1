param(
    [Parameter(Mandatory = $true)]
    [string]$BaselineReleaseId,
    [Parameter(Mandatory = $true)]
    [string]$CandidateReleaseId,
    [string]$MarketScope = "financial_system",
    [string]$HistoryMode = "default",
    [int]$HistoryLimit = 5000
)

$ErrorActionPreference = "Stop"
if ($PSVersionTable.PSVersion.Major -ge 7) {
    $PSNativeCommandUseErrorActionPreference = $false
}

$Root = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location -LiteralPath $Root

function Invoke-ReleaseReview {
    $args = @(
        "run", "-p", "fc-worker", "--",
        "research", "release", "review",
        "--market-scope", $MarketScope,
        "--baseline-release-id", $BaselineReleaseId,
        "--candidate-release-id", $CandidateReleaseId,
        "--history-mode", $HistoryMode,
        "--history-limit", "$HistoryLimit"
    )

    & cargo @args
    if ($LASTEXITCODE -ne 0) {
        throw "release review failed for baseline=$BaselineReleaseId candidate=$CandidateReleaseId"
    }
}

function Resolve-ReleaseReviewPath {
    param(
        [string]$Baseline,
        [string]$Candidate,
        [string]$Mode
    )

    $reportDirectory = Join-Path $Root "artifacts/research/release-review"
    $pattern = "*$Baseline-vs-$Candidate-$Mode-release-review.json"
    $match = Get-ChildItem -LiteralPath $reportDirectory -Filter $pattern -ErrorAction SilentlyContinue |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First 1

    if ($match) {
        return $match.FullName
    }

    return $null
}

$reviewPath = Resolve-ReleaseReviewPath -Baseline $BaselineReleaseId -Candidate $CandidateReleaseId -Mode $HistoryMode
if (-not $reviewPath) {
    Write-Host "No $HistoryMode release-review artifact found; running release review first."
    Invoke-ReleaseReview
    $reviewPath = Resolve-ReleaseReviewPath -Baseline $BaselineReleaseId -Candidate $CandidateReleaseId -Mode $HistoryMode
}

if (-not $reviewPath) {
    throw "Expected release-review artifact was not found after review run."
}

$outputDirectory = Join-Path $Root "artifacts/research/cooldown-audit"
New-Item -ItemType Directory -Force -Path $outputDirectory | Out-Null
$stamp = Get-Date -Format "yyyy-MM-dd"
$outputPath = Join-Path $outputDirectory "$stamp-$BaselineReleaseId-vs-$CandidateReleaseId-$HistoryMode-cooldown-audit.json"

$extractor = @'
import json
import sys
from datetime import datetime, timezone

review_path, output_path, baseline_release_id, candidate_release_id, history_mode = sys.argv[1:6]

with open(review_path, "r", encoding="utf-8") as handle:
    doc = json.load(handle)

comparison = doc.get("comparison") or {}

def metric(name):
    value = comparison.get(name)
    return value if isinstance(value, dict) else None

def horizon_row(rows, horizon):
    for row in rows or []:
        if int(row.get("horizon_days", -1)) == horizon:
            return row
    return None

def runtime_rows(review):
    return (review or {}).get("regime_separation_summaries") or []

def fp_episodes(assessment):
    episodes = (((assessment or {}).get("backtest_summary") or {})
        .get("rolling_audit") or {}).get("classified_episodes") or []
    return [
        {
            "start_date": episode.get("start_date"),
            "end_date": episode.get("end_date"),
            "duration_days": episode.get("duration_days"),
            "signal_count": episode.get("signal_count"),
            "classification": episode.get("classification"),
            "note": episode.get("note"),
        }
        for episode in episodes
        if episode.get("classification") == "false_positive"
    ]

def parse_date(value):
    return datetime.strptime(value, "%Y-%m-%d").date()

def overlaps(left, right):
    left_start = parse_date(left["start_date"])
    left_end = parse_date(left["end_date"])
    right_start = parse_date(right["start_date"])
    right_end = parse_date(right["end_date"])
    return left_start <= right_end and right_start <= left_end

def episode_kind(candidate, baseline_episodes):
    matched = [episode for episode in baseline_episodes if overlaps(candidate, episode)]
    if not matched:
        return "candidate_only", []

    max_baseline_duration = max(int(episode.get("duration_days") or 0) for episode in matched)
    if int(candidate.get("duration_days") or 0) > max_baseline_duration:
        return "extended_candidate_episode", matched
    return "shared_or_shorter_episode", matched

def top_episodes(episodes, limit=10):
    return sorted(
        episodes,
        key=lambda episode: (int(episode.get("duration_days") or 0), episode.get("start_date") or ""),
        reverse=True,
    )[:limit]

def scenario_false_positive_deltas():
    rows = []
    for scenario in comparison.get("backtest_scenarios") or []:
        baseline = int(scenario.get("baseline_false_positive_count") or 0)
        candidate = int(scenario.get("candidate_false_positive_count") or 0)
        delta = candidate - baseline
        if delta != 0:
            rows.append({
                "scenario_id": scenario.get("scenario_id"),
                "name": scenario.get("name"),
                "baseline_false_positive_count": baseline,
                "candidate_false_positive_count": candidate,
                "delta": delta,
                "outcome": scenario.get("outcome"),
            })
    return sorted(rows, key=lambda row: (row["delta"], row["scenario_id"] or ""), reverse=True)

baseline_runtime = runtime_rows(doc.get("baseline_runtime_review"))
candidate_runtime = runtime_rows(doc.get("candidate_runtime_review"))
comparison_runtime = comparison.get("runtime_separation_summary") or []

runtime_cooldown_rows = []
for horizon in (5, 20, 60):
    baseline_row = horizon_row(baseline_runtime, horizon)
    candidate_row = horizon_row(candidate_runtime, horizon)
    comparison_row = horizon_row(comparison_runtime, horizon)
    if not baseline_row and not candidate_row and not comparison_row:
        continue

    candidate_cooldown = None if not candidate_row else candidate_row.get("post_crisis_cooldown_avg_probability")
    candidate_positive = None if not candidate_row else candidate_row.get("positive_window_avg_probability")
    candidate_normal = None if not candidate_row else candidate_row.get("normal_avg_probability")
    cooldown_minus_positive = (
        None if candidate_cooldown is None or candidate_positive is None
        else candidate_cooldown - candidate_positive
    )
    cooldown_minus_normal = (
        None if candidate_cooldown is None or candidate_normal is None
        else candidate_cooldown - candidate_normal
    )
    runtime_cooldown_rows.append({
        "horizon_days": horizon,
        "baseline_diagnosis": None if not baseline_row else baseline_row.get("diagnosis"),
        "candidate_diagnosis": None if not candidate_row else candidate_row.get("diagnosis"),
        "baseline": baseline_row,
        "candidate": candidate_row,
        "comparison": comparison_row,
        "candidate_cooldown_minus_positive": cooldown_minus_positive,
        "candidate_cooldown_minus_normal": cooldown_minus_normal,
    })

baseline_fp = fp_episodes(doc.get("baseline_assessment"))
candidate_fp = fp_episodes(doc.get("candidate_assessment"))

candidate_regressions = []
for episode in top_episodes(candidate_fp, limit=25):
    kind, matched = episode_kind(episode, baseline_fp)
    if kind == "shared_or_shorter_episode":
        continue
    candidate_regressions.append({
        "kind": kind,
        "episode": episode,
        "overlapping_baseline_episodes": matched,
    })

no_go_reasons = []

def add_reason(code, summary, evidence):
    no_go_reasons.append({
        "code": code,
        "summary": summary,
        "evidence": evidence,
    })

precision = metric("actionable_precision")
if precision:
    candidate = precision.get("candidate")
    delta = precision.get("delta")
    if candidate is not None and (candidate < 0.70 or (delta is not None and delta <= -0.05)):
        add_reason(
            "actionable_precision_regression",
            "Candidate actionable precision is too weak for promotion.",
            precision,
        )

longest_fp = metric("longest_false_positive_episode_days")
if longest_fp:
    candidate = longest_fp.get("candidate")
    delta = longest_fp.get("delta")
    if (delta is not None and delta >= 7) or (candidate is not None and candidate > 30):
        add_reason(
            "longest_false_positive_episode_regression",
            "Candidate materially lengthens the longest pure false-positive episode.",
            longest_fp,
        )

runtime_floor = metric("runtime_floor_hit_count")
if runtime_floor and runtime_floor.get("delta") is not None and runtime_floor["delta"] <= -5:
    add_reason(
        "runtime_floor_hit_count_regression",
        "Candidate loses too many runtime floor hits.",
        runtime_floor,
    )

candidate20 = horizon_row(candidate_runtime, 20)
if candidate20:
    cooldown_minus_positive = (
        candidate20.get("post_crisis_cooldown_avg_probability")
        - candidate20.get("positive_window_avg_probability")
    )
    if candidate20.get("diagnosis") == "cooldown_bleed":
        add_reason(
            "candidate_20d_cooldown_bleed",
            "Candidate 20d runtime regime diagnosis is cooldown_bleed.",
            candidate20,
        )
    if cooldown_minus_positive >= 0:
        add_reason(
            "candidate_20d_cooldown_not_below_positive",
            "Candidate 20d cooldown average is not below positive-window average.",
            {
                "post_crisis_cooldown_avg_probability": candidate20.get("post_crisis_cooldown_avg_probability"),
                "positive_window_avg_probability": candidate20.get("positive_window_avg_probability"),
                "cooldown_minus_positive": cooldown_minus_positive,
            },
        )

recommendation = "manual_review"
if no_go_reasons:
    recommendation = "no_go_cooldown_false_positive"
elif candidate_regressions:
    recommendation = "manual_review_false_positive_episode_changes"
else:
    recommendation = "cooldown_false_positive_clean"

result = {
    "audit_type": "formal_candidate_cooldown_false_positive_audit",
    "generated_at": datetime.now(timezone.utc).isoformat(),
    "baseline_release_id": baseline_release_id,
    "candidate_release_id": candidate_release_id,
    "market_scope": doc.get("market_scope"),
    "history_mode": history_mode,
    "release_review_artifact": review_path,
    "reviewed_at": doc.get("reviewed_at"),
    "overall_guard_passed": doc.get("overall_guard_passed"),
    "probability_guard_passed": doc.get("probability_guard_passed"),
    "actionability_guard_passed": doc.get("actionability_guard_passed"),
    "operational_guard_passed": doc.get("operational_guard_passed"),
    "review_recommendation": doc.get("recommendation"),
    "comparison_metrics": {
        "timely_warning_rate": metric("timely_warning_rate"),
        "strict_actionable_point_count": metric("strict_actionable_point_count"),
        "runtime_floor_hit_count": runtime_floor,
        "actionable_precision": precision,
        "longest_false_positive_episode_days": longest_fp,
    },
    "runtime_cooldown_rows": runtime_cooldown_rows,
    "false_positive_episodes": {
        "baseline_top": top_episodes(baseline_fp),
        "candidate_top": top_episodes(candidate_fp),
        "candidate_regressions": candidate_regressions,
    },
    "scenario_false_positive_deltas": scenario_false_positive_deltas(),
    "no_go_reasons": no_go_reasons,
    "recommendation": recommendation,
}

with open(output_path, "w", encoding="utf-8") as handle:
    json.dump(result, handle, ensure_ascii=False, indent=2)
    handle.write("\n")

print(json.dumps({
    "output_path": output_path,
    "recommendation": recommendation,
    "no_go_reason_count": len(no_go_reasons),
    "candidate_regression_episode_count": len(candidate_regressions),
}, ensure_ascii=False))
'@

$summaryJson = $extractor | python - $reviewPath $outputPath $BaselineReleaseId $CandidateReleaseId $HistoryMode
if ($LASTEXITCODE -ne 0) {
    throw "cooldown audit extraction failed"
}

$summary = $summaryJson | ConvertFrom-Json
Write-Host "Cooldown / false-positive audit exported."
Write-Host ("  release review : {0}" -f $reviewPath)
Write-Host ("  output         : {0}" -f $summary.output_path)
Write-Host ("  recommendation : {0}" -f $summary.recommendation)
Write-Host ("  no-go reasons  : {0}" -f $summary.no_go_reason_count)
Write-Host ("  episode changes: {0}" -f $summary.candidate_regression_episode_count)
