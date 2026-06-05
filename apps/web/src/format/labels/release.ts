const METHOD_VERSION_FIELD_LABELS: Record<string, string> = {
  score: "评分规则版本",
  prob: "概率模型版本",
  calibration: "概率校准版本",
  feature: "特征集版本",
  label: "标签口径版本",
  posture: "执行节奏规则版本",
  playbook: "仓位动作框架版本",
  "prob mode": "概率模式",
  release: "运行状态",
  "release id": "当前生效版本",
  "pit mode": "点位可见性"
};

export function methodVersionFieldLabel(field: string): string {
  return METHOD_VERSION_FIELD_LABELS[field] ?? field;
}

const PROBABILITY_MODE_LABELS: Record<string, string> = {
  heuristic_mvp: "启发式过渡层"
};

export function probabilityModeLabel(mode: string): string {
  if (PROBABILITY_MODE_LABELS[mode]) {
    return PROBABILITY_MODE_LABELS[mode];
  }
  if (mode.startsWith("formal_bundle")) {
    return "正式概率包";
  }
  return mode;
}

const POINT_IN_TIME_MODE_LABELS: Record<string, string> = {
  strict: "严格 PIT",
  best_effort: "过渡 PIT"
};

export function pointInTimeModeLabel(mode: string): string {
  return POINT_IN_TIME_MODE_LABELS[mode] ?? mode;
}

const RELEASE_SERVING_STATUS_LABELS: Record<string, string> = {
  healthy: "运行正常",
  degraded: "降级运行"
};

export function releaseServingStatusLabel(status: string): string {
  return RELEASE_SERVING_STATUS_LABELS[status] ?? status;
}

const RELEASE_MANIFEST_STATUS_LABELS: Record<string, string> = {
  active: "当前生效",
  approved: "已批准",
  archived: "已归档",
  rolled_back: "已回退",
  retired: "已退役"
};

export function releaseManifestStatusLabel(status: string): string {
  return RELEASE_MANIFEST_STATUS_LABELS[status] ?? status;
}

const RUNTIME_THRESHOLD_LABELS: Record<string, string> = {
  "prepare floor": "准备档进入线",
  "hedge floor": "对冲档进入线",
  "defend floor": "防守档进入线",
  "weeks bridge": "数周窗口桥接线",
  "external bridge": "外部冲击桥接线",
  "carry bridge": "日元套息桥接线"
};

export function runtimeThresholdLabel(label: string): string {
  return RUNTIME_THRESHOLD_LABELS[label] ?? label;
}

const RELEASE_REVIEW_HISTORY_MODE_LABELS: Record<string, string> = {
  strict_rebuild: "严格重放",
  default: "默认历史缓存"
};

export function releaseReviewHistoryModeLabel(mode: string): string {
  return RELEASE_REVIEW_HISTORY_MODE_LABELS[mode] ?? mode ?? "—";
}

const RELEASE_REVIEW_WORKSTREAM_LABELS: Record<string, string> = {
  strict_review_vs_runtime_mapping: "严格评审 vs 运行映射",
  posture_continuity: "执行节奏连续性",
  score_confirmation: "评分确认层",
  transitional_bridge: "过渡桥接层"
};

export function releaseReviewWorkstreamLabel(workstream: string): string {
  return RELEASE_REVIEW_WORKSTREAM_LABELS[workstream] ?? workstream;
}

const RELEASE_REVIEW_ATTRIBUTION_LABELS: Record<string, string> = {
  candidate_regression: "候选版新增退化",
  both_baseline_and_candidate: "主线已有短板，候选未修复",
  baseline_shared_weakness: "主线既有短板"
};

export function releaseReviewAttributionLabel(attribution: string): string {
  return RELEASE_REVIEW_ATTRIBUTION_LABELS[attribution] ?? attribution;
}

const RELEASE_REVIEW_ACTION_TYPE_LABELS: Record<string, string> = {
  candidate_reject_or_retrain: "判退 / 重训",
  shared_blocker_fix_before_promotion: "晋升前先修",
  baseline_research_fix: "主线研究修复",
  manual_review: "继续人工复核"
};

export function releaseReviewActionTypeLabel(actionType: string): string {
  return RELEASE_REVIEW_ACTION_TYPE_LABELS[actionType] ?? actionType;
}

export function releaseReviewVerdictLabel(passed: boolean): string {
  return passed ? "通过当前 guard" : "存在 guard blocker";
}
