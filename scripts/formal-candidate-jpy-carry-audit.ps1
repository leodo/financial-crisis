param(
    [string]$SqlitePath = "data/fc-local.sqlite",
    [string]$MarketScope = "financial_system",
    [double]$GateThreshold = 0.38,
    [string]$OutputDir = "artifacts/research/jpy-carry-audit"
)

$ErrorActionPreference = "Stop"
if ($PSVersionTable.PSVersion.Major -ge 7) {
    $PSNativeCommandUseErrorActionPreference = $false
}

$Root = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location -LiteralPath $Root

function Escape-SqlLiteral {
    param([string]$Value)
    $Value.Replace("'", "''")
}

function Invoke-SqliteJson {
    param([string]$Sql)

    $db = "file:{0}?mode=ro&cache=shared" -f ($SqlitePath -replace "\\", "/")
    $raw = & sqlite3 -json $db $Sql
    if ($LASTEXITCODE -ne 0) {
        throw "sqlite3 query failed"
    }
    if (-not $raw) {
        return @()
    }
    $json = ($raw -join "`n")
    $parsed = $json | ConvertFrom-Json
    if ($null -eq $parsed) {
        return @()
    }
    @($parsed)
}

function Resolve-LatestDatasetKey {
    param([string]$DatasetId)

    $datasetIdSql = Escape-SqlLiteral -Value $DatasetId
    $marketSql = Escape-SqlLiteral -Value $MarketScope
    $rows = Invoke-SqliteJson -Sql @"
select dataset_key
from analytics_formal_datasets
where dataset_id = '$datasetIdSql'
  and market_scope = '$marketSql'
order by datetime(created_at) desc
limit 1;
"@
    if ($rows.Count -eq 0 -or -not $rows[0].dataset_key) {
        throw "No formal dataset found for dataset_id=$DatasetId market_scope=$MarketScope"
    }
    [string]$rows[0].dataset_key
}

function Read-WindowRows {
    param(
        [string]$DatasetKey,
        [string]$FromDate,
        [string]$ToDate
    )

    $datasetSql = Escape-SqlLiteral -Value $DatasetKey
    $fromSql = Escape-SqlLiteral -Value $FromDate
    $toSql = Escape-SqlLiteral -Value $ToDate
    Invoke-SqliteJson -Sql @"
select
  as_of_date,
  split_name,
  primary_scenario_id,
  scenario_family,
  scenario_training_role,
  protected_action_window,
  label_5d,
  label_20d,
  label_60d,
  action_label_5d,
  action_label_20d,
  action_label_60d,
  regime_5d,
  regime_20d,
  regime_60d,
  json_extract(features_json, '$.us_usdjpy_level') as usdjpy_level,
  json_extract(features_json, '$.us_usdjpy_change_20d') as usdjpy_change_20d,
  json_extract(features_json, '$.us_fed_funds_level') as fed_funds_level,
  json_extract(features_json, '$.external_dimension_score') as external_dimension_score
from analytics_formal_dataset_rows
where dataset_key = '$datasetSql'
  and as_of_date between '$fromSql' and '$toSql'
order by as_of_date asc;
"@
}

function To-NullableDouble {
    param($Value)
    if ($Value -is [array]) {
        $Value = @($Value | Where-Object { $null -ne $_ -and $_ -ne "" } | Select-Object -First 1)
        if ($Value -is [array]) {
            $Value = $Value | Select-Object -First 1
        }
    }
    if ($null -eq $Value -or $Value -eq "") {
        return $null
    }
    [double]$Value
}

function Round-Safe {
    param(
        $Value,
        [int]$Digits = 6
    )
    if ($null -eq $Value) {
        return $null
    }
    if ($Value -is [array]) {
        $Value = @($Value | Where-Object { $null -ne $_ } | Select-Object -First 1)
        if ($Value -is [array]) {
            $Value = $Value | Select-Object -First 1
        }
        if ($null -eq $Value) {
            return $null
        }
    }
    [math]::Round([double]$Value, $Digits)
}

function To-IntSafe {
    param($Value)
    if ($Value -is [array]) {
        $Value = @($Value | Where-Object { $null -ne $_ -and $_ -ne "" } | Select-Object -First 1)
        if ($Value -is [array]) {
            $Value = $Value | Select-Object -First 1
        }
    }
    if ($null -eq $Value -or $Value -eq "") {
        return 0
    }
    [int]$Value
}

function Normalize-ScoreThreshold {
    param(
        [string]$FeatureName,
        [double]$FeatureValue,
        [double]$Threshold
    )
    if (($FeatureName -in @("overall_score", "structural_score", "trigger_score", "external_dimension_score", "external_shock_score")) -and
        [math]::Abs($FeatureValue) -le 1.0 -and
        [math]::Abs($Threshold) -gt 1.0) {
        return $Threshold / 100.0
    }
    $Threshold
}

function Normalize-ScoreScale {
    param(
        [string]$FeatureName,
        [double]$FeatureValue,
        [double]$Scale
    )
    if (($FeatureName -in @("overall_score", "structural_score", "trigger_score", "external_dimension_score", "external_shock_score")) -and
        [math]::Abs($FeatureValue) -le 1.0 -and
        [math]::Abs($Scale) -gt 1.0) {
        return $Scale / 100.0
    }
    $Scale
}

function Scaled-TailPos {
    param(
        [string]$FeatureName,
        $Value,
        [double]$Threshold,
        [double]$Scale
    )
    $number = To-NullableDouble -Value $Value
    if ($null -eq $number) {
        return $null
    }
    $effectiveThreshold = Normalize-ScoreThreshold -FeatureName $FeatureName -FeatureValue $number -Threshold $Threshold
    $effectiveScale = Normalize-ScoreScale -FeatureName $FeatureName -FeatureValue $number -Scale $Scale
    [math]::Max(0.0, [math]::Min(1.0, (($number - $effectiveThreshold) / [math]::Max($effectiveScale, 1e-6))))
}

function Scaled-TailAbs {
    param(
        [string]$FeatureName,
        $Value,
        [double]$Threshold,
        [double]$Scale
    )
    $number = To-NullableDouble -Value $Value
    if ($null -eq $number) {
        return $null
    }
    $effectiveThreshold = Normalize-ScoreThreshold -FeatureName $FeatureName -FeatureValue $number -Threshold $Threshold
    $effectiveScale = Normalize-ScoreScale -FeatureName $FeatureName -FeatureValue $number -Scale $Scale
    [math]::Max(0.0, [math]::Min(1.0, (([math]::Abs($number) - $effectiveThreshold) / [math]::Max($effectiveScale, 1e-6))))
}

function Resolve-JpyCarryProxy {
    param($Row)

    $levelTail = Scaled-TailPos -FeatureName "us_usdjpy_level" -Value $Row.usdjpy_level -Threshold 145.0 -Scale 20.0
    $changeTail = Scaled-TailAbs -FeatureName "us_usdjpy_change_20d" -Value $Row.usdjpy_change_20d -Threshold 4.0 -Scale 8.0
    $fundingTail = Scaled-TailPos -FeatureName "us_fed_funds_level" -Value $Row.fed_funds_level -Threshold 4.0 -Scale 3.0
    $externalTail = Scaled-TailPos -FeatureName "external_dimension_score" -Value $Row.external_dimension_score -Threshold 50.0 -Scale 35.0
    if ($null -eq $levelTail -or $null -eq $changeTail -or $null -eq $fundingTail -or $null -eq $externalTail) {
        return [pscustomobject]@{
            proxy = $null
            level_tail = $levelTail
            change_tail = $changeTail
            funding_tail = $fundingTail
            external_tail = $externalTail
        }
    }

    $stressConfirmation = [math]::Max($changeTail, $externalTail)
    $confirmedLevel = $levelTail * (0.25 + 0.75 * $stressConfirmation)
    $proxy = 0.45 * $confirmedLevel +
        0.25 * $changeTail +
        0.15 * $fundingTail * $stressConfirmation +
        0.15 * $externalTail

    [pscustomobject]@{
        proxy = [math]::Max(0.0, [math]::Min(1.0, $proxy))
        level_tail = $levelTail
        change_tail = $changeTail
        funding_tail = $fundingTail
        external_tail = $externalTail
    }
}

function Get-RiskSupportReasons {
    param($Row)

    $reasons = New-Object 'System.Collections.Generic.List[string]'
    if ((To-IntSafe -Value $Row.protected_action_window) -eq 1) {
        $reasons.Add("protected_action_window")
    }
    foreach ($field in @("label_5d", "label_20d", "label_60d", "action_label_5d", "action_label_20d", "action_label_60d")) {
        if ((To-IntSafe -Value $Row.$field) -eq 1) {
            $reasons.Add($field)
        }
    }
    foreach ($field in @("regime_5d", "regime_20d", "regime_60d")) {
        $value = [string]$Row.$field
        if ($value -in @("pre_warning_buffer", "positive_window", "in_crisis")) {
            $reasons.Add(("{0}:{1}" -f $field, $value))
        }
    }
    $reasons.ToArray()
}

function Average-OrNull {
    param([object[]]$Values)

    $numbers = @($Values | Where-Object { $null -ne $_ } | ForEach-Object { [double]$_ })
    if ($numbers.Count -eq 0) {
        return $null
    }
    ($numbers | Measure-Object -Average).Average
}

function Classify-Window {
    param(
        [int]$GateActiveCount,
        [int]$SupportedGateActiveCount,
        [int]$NormalGateActiveCount
    )

    if ($GateActiveCount -eq 0) {
        return "no_gate_active_jpy_carry"
    }
    $supportRatio = $SupportedGateActiveCount / [double]$GateActiveCount
    if ($NormalGateActiveCount -eq 0 -and $supportRatio -ge 0.80) {
        return "clean_supported_carry_pressure"
    }
    if ($supportRatio -ge 0.50) {
        return "mixed_but_supported_carry_pressure"
    }
    "ordinary_fx_spike_risk"
}

function Analyze-Window {
    param($Window)

    $datasetKey = Resolve-LatestDatasetKey -DatasetId $Window.dataset_id
    $rows = @(Read-WindowRows -DatasetKey $datasetKey -FromDate $Window.from -ToDate $Window.to)
    $enriched = foreach ($row in $rows) {
        $proxy = Resolve-JpyCarryProxy -Row $row
        $supportReasons = @(Get-RiskSupportReasons -Row $row)
        $jpyProxy = $proxy.proxy
        [pscustomobject]@{
            as_of_date = [string]$row.as_of_date
            split_name = [string]$row.split_name
            primary_scenario_id = [string]$row.primary_scenario_id
            scenario_family = [string]$row.scenario_family
            protected_action_window = To-IntSafe -Value $row.protected_action_window
            regime_5d = [string]$row.regime_5d
            regime_20d = [string]$row.regime_20d
            regime_60d = [string]$row.regime_60d
            support_reasons = $supportReasons
            supported_risk_context = $supportReasons.Count -gt 0
            jpy_carry_proxy = Round-Safe -Value $jpyProxy
            gate_active = $null -ne $jpyProxy -and $jpyProxy -ge $GateThreshold
            usdjpy_level = Round-Safe -Value (To-NullableDouble -Value $row.usdjpy_level)
            usdjpy_change_20d = Round-Safe -Value (To-NullableDouble -Value $row.usdjpy_change_20d)
            fed_funds_level = Round-Safe -Value (To-NullableDouble -Value $row.fed_funds_level)
            external_dimension_score = Round-Safe -Value (To-NullableDouble -Value $row.external_dimension_score)
            level_tail = Round-Safe -Value $proxy.level_tail
            change_tail = Round-Safe -Value $proxy.change_tail
            funding_tail = Round-Safe -Value $proxy.funding_tail
            external_tail = Round-Safe -Value $proxy.external_tail
        }
    }

    $gateRows = @($enriched | Where-Object { $_.gate_active })
    $supportedGateRows = @($gateRows | Where-Object { $_.supported_risk_context })
    $normalGateRows = @($gateRows | Where-Object { -not $_.supported_risk_context })
    $topRows = @(
        $enriched |
            Where-Object { $null -ne $_.jpy_carry_proxy } |
            Sort-Object -Property @{ Expression = "jpy_carry_proxy"; Descending = $true }, as_of_date |
            Select-Object -First 8
    )
    $maxRow = $topRows | Select-Object -First 1
    $gateActiveCount = $gateRows.Count
    $supportedGateActiveCount = $supportedGateRows.Count
    $normalGateActiveCount = $normalGateRows.Count

    [pscustomobject]@{
        window_id = $Window.window_id
        title = $Window.title
        dataset_id = $Window.dataset_id
        dataset_key = $datasetKey
        from = $Window.from
        to = $Window.to
        row_count = $enriched.Count
        gate_threshold = $GateThreshold
        gate_active_count = $gateActiveCount
        supported_gate_active_count = $supportedGateActiveCount
        ordinary_gate_active_count = $normalGateActiveCount
        support_ratio = if ($gateActiveCount -gt 0) { Round-Safe -Value ($supportedGateActiveCount / [double]$gateActiveCount) } else { $null }
        protected_action_window_count = @($enriched | Where-Object { $_.protected_action_window -eq 1 }).Count
        supported_risk_context_count = @($enriched | Where-Object { $_.supported_risk_context }).Count
        avg_jpy_carry_proxy = Round-Safe -Value (Average-OrNull -Values @($enriched | ForEach-Object { $_.jpy_carry_proxy }))
        max_jpy_carry_proxy = if ($maxRow) { $maxRow.jpy_carry_proxy } else { $null }
        max_jpy_carry_proxy_date = if ($maxRow) { $maxRow.as_of_date } else { $null }
        classification = Classify-Window -GateActiveCount $gateActiveCount -SupportedGateActiveCount $supportedGateActiveCount -NormalGateActiveCount $normalGateActiveCount
        top_jpy_carry_rows = $topRows
    }
}

$windows = @(
    [pscustomobject]@{
        window_id = "black_monday_1987_fx_window"
        title = "1987 Black Monday high-FX window"
        dataset_id = "formal_v1_ext_acute_pre1990"
        from = "1987-09-01"
        to = "1987-11-20"
    },
    [pscustomobject]@{
        window_id = "early_90s_banking_fx_window"
        title = "1990 early banking stress high-FX window"
        dataset_id = "formal_v1_ext_stress_1990_daily"
        from = "1990-07-01"
        to = "1990-10-31"
    },
    [pscustomobject]@{
        window_id = "jpy_unwind_2024_fx_window"
        title = "2024 JPY carry unwind watch window"
        dataset_id = "formal_v1_main_1990_daily"
        from = "2024-07-01"
        to = "2024-09-30"
    }
)

$windowSummaries = @($windows | ForEach-Object { Analyze-Window -Window $_ })
$ordinarySpikeWindows = @($windowSummaries | Where-Object { $_.classification -eq "ordinary_fx_spike_risk" })
$supportedWindows = @($windowSummaries | Where-Object { $_.classification -in @("clean_supported_carry_pressure", "mixed_but_supported_carry_pressure") })
$noGateWindows = @($windowSummaries | Where-Object { $_.classification -eq "no_gate_active_jpy_carry" })

$overallConclusion = if ($ordinarySpikeWindows.Count -gt 0) {
    "needs_proxy_tightening"
} elseif ($supportedWindows.Count -gt 0 -and $noGateWindows.Count -eq 0) {
    "supported_by_protected_or_prewarning_pressure"
} elseif ($supportedWindows.Count -gt 0) {
    "partially_supported_but_sparse"
} else {
    "insufficient_jpy_carry_gate_evidence"
}

$report = [pscustomobject]@{
    generated_at = (Get-Date).ToUniversalTime().ToString("o")
    sqlite_path = $SqlitePath
    market_scope = $MarketScope
    gate_threshold = $GateThreshold
    objective = "Check whether jpy_carry overlay support in high-FX windows comes from protected/pre-warning carry pressure or ordinary FX spikes."
    overall_conclusion = $overallConclusion
    ordinary_spike_window_count = $ordinarySpikeWindows.Count
    supported_window_count = $supportedWindows.Count
    no_gate_window_count = $noGateWindows.Count
    windows = $windowSummaries
    interpretation = @(
        "gate_active_count counts rows where the formal jpy_carry proxy is above the overlay gate.",
        "supported_gate_active_count requires protected/action labels or pre-warning/positive/crisis regime context.",
        "ordinary_gate_active_count is the main false-attribution risk: high FX/carry proxy without scenario support."
    )
}

$resolvedOutputDir = Join-Path $Root $OutputDir
New-Item -ItemType Directory -Force -Path $resolvedOutputDir | Out-Null
$stamp = (Get-Date).ToUniversalTime().ToString("yyyyMMddTHHmmss")
$reportPath = Join-Path $resolvedOutputDir "$stamp-jpy-carry-scenario-audit.json"
$report | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath $reportPath -Encoding UTF8

Write-Host "JPY carry scenario audit: $overallConclusion"
foreach ($summary in $windowSummaries) {
    Write-Host ("  {0}: gate={1}, supported={2}, ordinary={3}, max={4} @ {5}, class={6}" -f `
            $summary.window_id,
            $summary.gate_active_count,
            $summary.supported_gate_active_count,
            $summary.ordinary_gate_active_count,
            $summary.max_jpy_carry_proxy,
            $summary.max_jpy_carry_proxy_date,
            $summary.classification)
}
Write-Host "Report: $reportPath"
