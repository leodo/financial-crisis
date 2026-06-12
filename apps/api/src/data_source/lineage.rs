use chrono::NaiveDate;
use fc_domain::{AssessmentSnapshot, KeyIndicatorLineage, KeyIndicatorLineageEvidenceLevel};
use fc_storage::{ObservationLineageRecord, SqliteStore};

pub(super) async fn enrich_sqlite_key_indicator_lineage(
    store: &SqliteStore,
    as_of_date: NaiveDate,
    assessment: &mut AssessmentSnapshot,
) {
    for indicator in &mut assessment.key_indicators {
        match store
            .load_latest_observation_lineage(
                &indicator.indicator_id,
                &indicator.entity_id,
                as_of_date,
            )
            .await
        {
            Ok(Some(lineage)) => {
                indicator.lineage = Some(key_indicator_lineage_from_record(lineage));
            }
            Ok(None) => {
                indicator.lineage = Some(missing_lineage());
            }
            Err(error) => {
                tracing::warn!(
                    indicator_id = %indicator.indicator_id,
                    entity_id = %indicator.entity_id,
                    error = %format!("{error:#}"),
                    "failed to load key indicator lineage"
                );
            }
        }
    }
}

fn missing_lineage() -> KeyIndicatorLineage {
    KeyIndicatorLineage {
        evidence_level: KeyIndicatorLineageEvidenceLevel::Missing,
        note: "未在本地 SQLite 中找到该关键指标的观测追溯记录。".to_string(),
        raw_payload_id: None,
        run_id: None,
        run_status: None,
        fetched_at: None,
        records_written: None,
        response_hash: None,
        raw_file_path: None,
    }
}

fn key_indicator_lineage_from_record(record: ObservationLineageRecord) -> KeyIndicatorLineage {
    let evidence_level = match (record.run_id.as_ref(), record.raw_payload_id.as_ref()) {
        (Some(_), Some(_)) => KeyIndicatorLineageEvidenceLevel::RunRawObservation,
        (None, Some(_)) => KeyIndicatorLineageEvidenceLevel::RawObservation,
        _ => KeyIndicatorLineageEvidenceLevel::ObservationOnly,
    };
    let note = match evidence_level {
        KeyIndicatorLineageEvidenceLevel::RunRawObservation => {
            "该值可追溯到一次 ingestion run、对应 raw response 和落库观测。".to_string()
        }
        KeyIndicatorLineageEvidenceLevel::RawObservation => {
            "该值可追溯到 raw response 和落库观测，但缺少 ingestion run 记录。".to_string()
        }
        KeyIndicatorLineageEvidenceLevel::ObservationOnly => {
            "该值当前只有落库观测记录，缺少 raw response / ingestion run 追溯。".to_string()
        }
        KeyIndicatorLineageEvidenceLevel::Missing => {
            "未在本地 SQLite 中找到该关键指标的观测追溯记录。".to_string()
        }
    };

    KeyIndicatorLineage {
        evidence_level,
        note,
        raw_payload_id: record.raw_payload_id,
        run_id: record.run_id,
        run_status: record.run_status,
        fetched_at: record.fetched_at,
        records_written: record.records_written,
        response_hash: record.response_hash,
        raw_file_path: record.raw_file_path,
    }
}
