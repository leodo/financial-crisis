use std::{fs, path::Path as FsPath};

use anyhow::{bail, Context};
use chrono::{DateTime, FixedOffset, Utc};
use fc_domain::{ModelReleaseManifest, ModelReleaseRecord};
use serde::Deserialize;

use super::guardrails::{compare_operational_guardrails, print_operational_guardrail_summary};
use super::options::{
    ReleaseListOptions, ReleasePublishOptions, ReleaseShowOptions, ReleaseSwitchOptions,
};

pub(crate) async fn research_release_publish(args: &[String]) -> anyhow::Result<()> {
    let options = ReleasePublishOptions::parse(args)?;
    let store = crate::open_sqlite_store().await?;
    store.migrate().await?;
    let manifest = crate::read_release_manifest(&options.manifest_path)?;
    ensure_release_publish_eligible(&manifest, options.review_only)?;
    let record = ModelReleaseRecord {
        manifest,
        created_at: Utc::now(),
        activated_at: None,
        retired_at: None,
    };
    store.upsert_model_release(&record).await?;
    println!(
        "Saved release {} for market scope {}.",
        record.manifest.release_id, record.manifest.market_scope
    );
    println!("  Bundle     {}", record.manifest.bundle_uri);
    println!("  Prob mode  {}", record.manifest.probability_mode);
    println!("  PIT mode   {}", record.manifest.point_in_time_mode);
    if options.review_only {
        println!("  Publish    review-only");
    }

    if options.activate {
        activate_release_with_runtime_guard(
            &store,
            &record.manifest.market_scope,
            &record.manifest.release_id,
            options.reload_api,
            &options.api_reload_url,
            options.skip_operational_guard,
            &options.updated_by,
        )
        .await?;
    }

    Ok(())
}

pub(crate) async fn research_release_list(args: &[String]) -> anyhow::Result<()> {
    let options = ReleaseListOptions::parse(args)?;
    let store = crate::open_sqlite_store().await?;
    store.migrate().await?;
    let releases = store
        .list_model_releases(options.market_scope.as_deref())
        .await?;
    if releases.is_empty() {
        println!("No model releases found.");
        return Ok(());
    }
    println!(
        "{:<32} {:<18} {:<12} {:<12} {:<16} {:<24}",
        "release_id", "market_scope", "status", "serving", "prob_mode", "created_at"
    );
    for release in releases {
        println!(
            "{:<32} {:<18} {:<12} {:<12} {:<16} {:<24}",
            crate::truncate_text(&release.manifest.release_id, 32),
            crate::truncate_text(&release.manifest.market_scope, 18),
            crate::truncate_text(&release.manifest.status, 12),
            crate::truncate_text(&release.manifest.serving_status, 12),
            crate::truncate_text(&release.manifest.probability_mode, 16),
            release.created_at.to_rfc3339()
        );
    }
    Ok(())
}

pub(crate) async fn research_release_show(args: &[String]) -> anyhow::Result<()> {
    let options = ReleaseShowOptions::parse(args)?;
    let store = crate::open_sqlite_store().await?;
    store.migrate().await?;
    let release = store
        .load_model_release(&options.release_id)
        .await?
        .with_context(|| format!("release {} not found", options.release_id))?;
    println!("{}", serde_json::to_string_pretty(&release)?);
    Ok(())
}

pub(crate) async fn research_release_activate(args: &[String]) -> anyhow::Result<()> {
    let options = ReleaseSwitchOptions::parse(args)?;
    let store = crate::open_sqlite_store().await?;
    store.migrate().await?;
    let market_scope =
        resolve_release_market_scope(&store, &options.release_id, options.market_scope.as_deref())
            .await?;
    activate_release_with_runtime_guard(
        &store,
        &market_scope,
        &options.release_id,
        options.reload_api,
        &options.api_reload_url,
        options.skip_operational_guard,
        &options.updated_by,
    )
    .await?;
    Ok(())
}

pub(crate) async fn research_release_rollback(args: &[String]) -> anyhow::Result<()> {
    let options = ReleaseSwitchOptions::parse(args)?;
    let store = crate::open_sqlite_store().await?;
    store.migrate().await?;
    let target_release = store
        .load_model_release(&options.release_id)
        .await?
        .with_context(|| format!("release {} not found", options.release_id))?;
    ensure_release_activation_eligible(&target_release)?;
    let market_scope =
        resolve_release_market_scope(&store, &options.release_id, options.market_scope.as_deref())
            .await?;
    let activated = store
        .rollback_model_release(&market_scope, &options.release_id, &options.updated_by)
        .await?;
    println!(
        "Rolled back {} to release {}.",
        market_scope, activated.manifest.release_id
    );
    println!(
        "  mode={} serving={} pit={}",
        activated.manifest.probability_mode,
        activated.manifest.serving_status,
        activated.manifest.point_in_time_mode
    );
    if options.reload_api {
        crate::reload_api_runtime(&options.api_reload_url).await?;
        println!("Reloaded API runtime via {}.", options.api_reload_url);
    }
    Ok(())
}

pub(crate) async fn activate_release_with_runtime_guard(
    store: &fc_storage::SqliteStore,
    market_scope: &str,
    release_id: &str,
    reload_api: bool,
    api_reload_url: &str,
    skip_operational_guard: bool,
    updated_by: &str,
) -> anyhow::Result<ModelReleaseRecord> {
    let target_release = store
        .load_model_release(release_id)
        .await?
        .with_context(|| format!("release {release_id} not found"))?;
    ensure_release_activation_eligible(&target_release)?;

    let previous_active = store.load_active_model_release(market_scope).await?;
    let previous_release_id = previous_active
        .as_ref()
        .map(|release| release.manifest.release_id.clone());
    let activation_review_gate = if reload_api && !skip_operational_guard {
        resolve_release_activation_review_gate(
            market_scope,
            previous_release_id.as_deref(),
            release_id,
        )
    } else {
        None
    };
    if let Some(ReleaseActivationReviewGate::BlockCandidate { summary }) =
        activation_review_gate.as_ref()
    {
        bail!(
            "release {} cannot be activated because the latest {} release review at {} already marked it FAIL against baseline {}",
            release_id,
            summary.history_mode,
            summary.reviewed_at,
            summary.baseline_release_id
        );
    }
    let should_check_guard = reload_api
        && !skip_operational_guard
        && previous_release_id.as_deref() != Some(release_id)
        && !matches!(
            activation_review_gate.as_ref(),
            Some(ReleaseActivationReviewGate::AllowBaselineRestore { .. })
        );
    let baseline_assessment = if should_check_guard {
        Some(crate::fetch_assessment_snapshot_for_guard(api_reload_url).await?)
    } else {
        None
    };

    let activated = store
        .activate_model_release(market_scope, release_id, updated_by)
        .await?;
    println!(
        "Activated release {} for {}.",
        activated.manifest.release_id, activated.manifest.market_scope
    );
    println!(
        "  mode={} serving={} pit={}",
        activated.manifest.probability_mode,
        activated.manifest.serving_status,
        activated.manifest.point_in_time_mode
    );

    if reload_api {
        println!(
            "Reloading API runtime via {api_reload_url}. First load for a new release may take several minutes while history snapshots are materialized."
        );
        crate::reload_api_runtime(api_reload_url).await?;
        println!("Reloaded API runtime via {api_reload_url}.");
    }

    if let Some(ReleaseActivationReviewGate::AllowBaselineRestore { summary }) =
        activation_review_gate.as_ref()
    {
        println!(
            "Skipping runtime regression comparison because the latest {} release review at {} already marked current active {} as FAIL against baseline {}.",
            summary.history_mode,
            summary.reviewed_at,
            summary.candidate_release_id,
            summary.baseline_release_id
        );
        return Ok(activated);
    }

    if let Some(baseline_assessment) = baseline_assessment {
        let candidate_assessment =
            crate::fetch_assessment_snapshot_for_guard(api_reload_url).await?;
        let regressions =
            compare_operational_guardrails(&baseline_assessment, &candidate_assessment);
        if regressions.is_empty() {
            print_operational_guardrail_summary(&baseline_assessment, &candidate_assessment);
            return Ok(activated);
        }

        if let Some(previous_release_id) = previous_release_id
            .as_deref()
            .filter(|previous_release_id| *previous_release_id != release_id)
        {
            println!(
                "Operational guard failed after activating {release_id}. Rolling back to {previous_release_id}."
            );
            let rolled_back = store
                .rollback_model_release(market_scope, previous_release_id, updated_by)
                .await?;
            if reload_api {
                println!(
                    "Reloading API runtime after rollback via {api_reload_url}. This may also take several minutes."
                );
                crate::reload_api_runtime(api_reload_url).await?;
                println!("Reloaded API runtime after rollback.");
            }
            bail!(
                "release {} regressed against baseline release {} and was rolled back to {}:\n  - {}",
                release_id,
                baseline_assessment
                    .method
                    .release_id
                    .as_deref()
                    .unwrap_or("unknown"),
                rolled_back.manifest.release_id,
                regressions.join("\n  - ")
            );
        }

        bail!(
            "release {} regressed against baseline but no previous active release was available for automatic rollback:\n  - {}",
            release_id,
            regressions.join("\n  - ")
        );
    }

    if !reload_api && !skip_operational_guard {
        println!(
            "Operational guard skipped because --reload-api was not enabled; use --reload-api to compare the new runtime against the current baseline."
        );
    } else if skip_operational_guard {
        println!("Operational guard explicitly skipped.");
    }

    Ok(activated)
}

fn ensure_release_publish_eligible(
    manifest: &ModelReleaseManifest,
    review_only: bool,
) -> anyhow::Result<()> {
    if review_only {
        return Ok(());
    }

    if release_requires_review_only(manifest) {
        bail!(
            "release {} is marked {} and cannot be published as a formal release without --review-only; keep it in review-only storage or republish an approved healthy release",
            manifest.release_id,
            release_state_label(manifest)
        );
    }

    Ok(())
}

fn ensure_release_activation_eligible(release: &ModelReleaseRecord) -> anyhow::Result<()> {
    if release_requires_review_only(&release.manifest) {
        bail!(
            "release {} is marked {} and cannot be activated directly; complete review and promote an approved healthy release instead",
            release.manifest.release_id,
            release_state_label(&release.manifest)
        );
    }
    Ok(())
}

fn release_requires_review_only(manifest: &ModelReleaseManifest) -> bool {
    manifest.status == "candidate" || manifest.serving_status == "shadow"
}

fn release_state_label(manifest: &ModelReleaseManifest) -> String {
    format!("{}/{}", manifest.status, manifest.serving_status)
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ReleaseActivationReviewGate {
    BlockCandidate {
        summary: RelevantReleaseReviewSummary,
    },
    AllowBaselineRestore {
        summary: RelevantReleaseReviewSummary,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RelevantReleaseReviewSummary {
    reviewed_at: String,
    history_mode: String,
    baseline_release_id: String,
    candidate_release_id: String,
    overall_guard_passed: bool,
}

#[derive(Debug, Clone, Deserialize)]
struct ReleaseReviewArtifactReleaseRef {
    release_id: String,
}

#[derive(Debug, Clone, Deserialize)]
struct ReleaseReviewArtifactWire {
    reviewed_at: String,
    market_scope: String,
    history_mode: String,
    baseline_release: ReleaseReviewArtifactReleaseRef,
    candidate_release: ReleaseReviewArtifactReleaseRef,
    overall_guard_passed: bool,
}

fn resolve_release_activation_review_gate(
    market_scope: &str,
    previous_release_id: Option<&str>,
    target_release_id: &str,
) -> Option<ReleaseActivationReviewGate> {
    let previous_release_id = previous_release_id?;
    if previous_release_id == target_release_id {
        return None;
    }
    let summary = load_latest_relevant_release_review_summary(
        market_scope,
        previous_release_id,
        target_release_id,
    )?;
    if summary.overall_guard_passed {
        return None;
    }
    resolve_release_activation_review_gate_for_summary(
        previous_release_id,
        target_release_id,
        summary,
    )
}

fn resolve_release_activation_review_gate_for_summary(
    previous_release_id: &str,
    target_release_id: &str,
    summary: RelevantReleaseReviewSummary,
) -> Option<ReleaseActivationReviewGate> {
    if summary.baseline_release_id == previous_release_id
        && summary.candidate_release_id == target_release_id
    {
        return Some(ReleaseActivationReviewGate::BlockCandidate { summary });
    }
    if summary.baseline_release_id == target_release_id
        && summary.candidate_release_id == previous_release_id
    {
        return Some(ReleaseActivationReviewGate::AllowBaselineRestore { summary });
    }
    None
}

fn load_latest_relevant_release_review_summary(
    market_scope: &str,
    first_release_id: &str,
    second_release_id: &str,
) -> Option<RelevantReleaseReviewSummary> {
    let directories = [
        FsPath::new("artifacts/research/release-review"),
        FsPath::new("reports/release-review"),
    ];
    load_latest_relevant_release_review_summary_from_dirs(
        market_scope,
        first_release_id,
        second_release_id,
        &directories,
    )
}

fn load_latest_relevant_release_review_summary_from_dirs(
    market_scope: &str,
    first_release_id: &str,
    second_release_id: &str,
    directories: &[&FsPath],
) -> Option<RelevantReleaseReviewSummary> {
    let mut candidates = Vec::<(
        u8,
        Option<DateTime<FixedOffset>>,
        RelevantReleaseReviewSummary,
    )>::new();
    for directory in directories {
        let Ok(entries) = fs::read_dir(directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
                continue;
            }
            let Ok(body) = fs::read_to_string(&path) else {
                continue;
            };
            let Ok(wire) = serde_json::from_str::<ReleaseReviewArtifactWire>(&body) else {
                continue;
            };
            if wire.market_scope != market_scope {
                continue;
            }
            let matches_pair = (wire.baseline_release.release_id == first_release_id
                && wire.candidate_release.release_id == second_release_id)
                || (wire.baseline_release.release_id == second_release_id
                    && wire.candidate_release.release_id == first_release_id);
            if !matches_pair {
                continue;
            }
            let history_mode_priority = if wire.history_mode == "strict_rebuild" {
                1
            } else {
                0
            };
            candidates.push((
                history_mode_priority,
                DateTime::parse_from_rfc3339(&wire.reviewed_at).ok(),
                RelevantReleaseReviewSummary {
                    reviewed_at: wire.reviewed_at,
                    history_mode: wire.history_mode,
                    baseline_release_id: wire.baseline_release.release_id,
                    candidate_release_id: wire.candidate_release.release_id,
                    overall_guard_passed: wire.overall_guard_passed,
                },
            ));
        }
    }

    candidates.sort_by(|left, right| {
        right
            .0
            .cmp(&left.0)
            .then_with(|| right.1.cmp(&left.1))
            .then_with(|| right.2.reviewed_at.cmp(&left.2.reviewed_at))
    });
    candidates.into_iter().next().map(|(_, _, summary)| summary)
}

async fn resolve_release_market_scope(
    store: &fc_storage::SqliteStore,
    release_id: &str,
    override_market_scope: Option<&str>,
) -> anyhow::Result<String> {
    if let Some(market_scope) = override_market_scope {
        return Ok(market_scope.to_string());
    }
    let release = store
        .load_model_release(release_id)
        .await?
        .with_context(|| format!("release {release_id} not found"))?;
    Ok(release.manifest.market_scope)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use chrono::Utc;
    use fc_domain::{ModelReleaseManifest, ModelReleaseRecord};

    use super::{
        ensure_release_activation_eligible, ensure_release_publish_eligible,
        load_latest_relevant_release_review_summary_from_dirs,
        resolve_release_activation_review_gate_for_summary, ReleaseActivationReviewGate,
        RelevantReleaseReviewSummary,
    };

    fn manifest(status: &str, serving_status: &str) -> ModelReleaseManifest {
        ModelReleaseManifest {
            release_id: format!("release-{status}-{serving_status}"),
            market_scope: "financial_system".to_string(),
            status: status.to_string(),
            probability_mode: "formal_bundle_v1".to_string(),
            serving_status: serving_status.to_string(),
            bundle_uri: "bundle.json".to_string(),
            feature_set_version: crate::DEFAULT_FORMAL_FEATURE_SET_VERSION.to_string(),
            label_version: "formal_label_v1_main".to_string(),
            prob_model_version: "prob".to_string(),
            calibration_version: "calib".to_string(),
            posture_policy_version: "posture".to_string(),
            action_playbook_version: "playbook".to_string(),
            point_in_time_mode: "best_effort".to_string(),
            training_range_start: None,
            training_range_end: None,
            calibration_range_start: None,
            calibration_range_end: None,
            evaluation_range_start: None,
            evaluation_range_end: None,
            brier_score: None,
            log_loss: None,
            ece: None,
            note: String::new(),
        }
    }

    fn release(status: &str, serving_status: &str) -> ModelReleaseRecord {
        ModelReleaseRecord {
            manifest: manifest(status, serving_status),
            created_at: Utc::now(),
            activated_at: None,
            retired_at: None,
        }
    }

    #[test]
    fn candidate_release_is_not_activation_eligible() {
        let error =
            ensure_release_activation_eligible(&release("candidate", "healthy")).unwrap_err();
        assert!(
            error.to_string().contains("candidate/healthy"),
            "unexpected error: {error:#}"
        );
    }

    #[test]
    fn shadow_release_is_not_activation_eligible() {
        let error = ensure_release_activation_eligible(&release("approved", "shadow")).unwrap_err();
        assert!(
            error.to_string().contains("approved/shadow"),
            "unexpected error: {error:#}"
        );
    }

    #[test]
    fn approved_healthy_release_is_activation_eligible() {
        ensure_release_activation_eligible(&release("approved", "healthy")).unwrap();
    }

    #[test]
    fn candidate_shadow_release_requires_review_only_publish() {
        let error =
            ensure_release_publish_eligible(&manifest("candidate", "shadow"), false).unwrap_err();
        assert!(
            error.to_string().contains("without --review-only"),
            "unexpected error: {error:#}"
        );
    }

    #[test]
    fn candidate_shadow_release_can_publish_for_review_only() {
        ensure_release_publish_eligible(&manifest("candidate", "shadow"), true).unwrap();
    }

    #[test]
    fn approved_healthy_release_can_publish_formally() {
        ensure_release_publish_eligible(&manifest("approved", "healthy"), false).unwrap();
    }

    #[test]
    fn release_review_loader_prefers_strict_rebuild_over_newer_default() {
        let root = temp_test_dir("release-review-loader");
        std::fs::write(
            root.join("strict.json"),
            r#"{
  "reviewed_at": "2026-06-07T08:00:00+00:00",
  "market_scope": "financial_system",
  "history_mode": "strict_rebuild",
  "baseline_release": { "release_id": "baseline" },
  "candidate_release": { "release_id": "candidate" },
  "overall_guard_passed": false
}"#,
        )
        .unwrap();
        std::fs::write(
            root.join("default.json"),
            r#"{
  "reviewed_at": "2026-06-07T09:00:00+00:00",
  "market_scope": "financial_system",
  "history_mode": "default",
  "baseline_release": { "release_id": "baseline" },
  "candidate_release": { "release_id": "candidate" },
  "overall_guard_passed": true
}"#,
        )
        .unwrap();

        let dirs = [root.as_path()];
        let summary = load_latest_relevant_release_review_summary_from_dirs(
            "financial_system",
            "baseline",
            "candidate",
            &dirs,
        )
        .unwrap();

        assert_eq!(summary.history_mode, "strict_rebuild");
        assert!(!summary.overall_guard_passed);

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn failed_review_blocks_candidate_activation() {
        let gate = resolve_release_activation_review_gate_for_summary(
            "baseline",
            "candidate",
            RelevantReleaseReviewSummary {
                reviewed_at: "2026-06-07T08:00:00+00:00".to_string(),
                history_mode: "strict_rebuild".to_string(),
                baseline_release_id: "baseline".to_string(),
                candidate_release_id: "candidate".to_string(),
                overall_guard_passed: false,
            },
        );

        assert!(matches!(
            gate,
            Some(ReleaseActivationReviewGate::BlockCandidate { .. })
        ));
    }

    #[test]
    fn failed_review_allows_restoring_baseline_from_rejected_candidate() {
        let gate = resolve_release_activation_review_gate_for_summary(
            "candidate",
            "baseline",
            RelevantReleaseReviewSummary {
                reviewed_at: "2026-06-07T08:00:00+00:00".to_string(),
                history_mode: "strict_rebuild".to_string(),
                baseline_release_id: "baseline".to_string(),
                candidate_release_id: "candidate".to_string(),
                overall_guard_passed: false,
            },
        );

        assert!(matches!(
            gate,
            Some(ReleaseActivationReviewGate::AllowBaselineRestore { .. })
        ));
    }

    fn temp_test_dir(prefix: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "fc-worker-{prefix}-{}",
            Utc::now()
                .timestamp_nanos_opt()
                .unwrap_or_default()
                .unsigned_abs()
        ));
        std::fs::create_dir_all(&path).unwrap();
        path
    }
}
