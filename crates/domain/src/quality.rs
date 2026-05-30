use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QualityGrade {
    A,
    B,
    C,
    D,
    F,
}

impl QualityGrade {
    pub fn from_score(score: f64) -> Self {
        if score >= 90.0 {
            Self::A
        } else if score >= 75.0 {
            Self::B
        } else if score >= 60.0 {
            Self::C
        } else if score >= 40.0 {
            Self::D
        } else {
            Self::F
        }
    }

    pub fn scoring_weight(self) -> f64 {
        match self {
            Self::A => 1.0,
            Self::B => 0.9,
            Self::C => 0.6,
            Self::D | Self::F => 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataQualitySummary {
    pub overall_score: f64,
    pub grade: QualityGrade,
    pub stale_indicator_count: usize,
    pub low_quality_indicator_count: usize,
    pub prototype_source_count: usize,
    pub blocked_indicator_count: usize,
}
