export interface PostureClauseDescriptor {
  label: string;
  summary: string;
  kind: "trigger" | "blocker" | "preference";
}

const POSTURE_CLAUSE_DESCRIPTORS: Record<string, PostureClauseDescriptor> = {
  defend_p5d_trigger: {
    label: "5日危机先验触发 defend",
    summary: "5日危机先验超过 defend 阈值，且触发层已经进入高压区。",
    kind: "trigger"
  },
  defend_carry_trigger: {
    label: "套息平仓压力触发 defend",
    summary: "JPY carry 压力和外部冲击共振，系统把短端窗口视为已打开。",
    kind: "trigger"
  },
  defend_actionability: {
    label: "动作头确认 defend",
    summary: "独立动作头认为近端保护优先级已经足够高。",
    kind: "trigger"
  },
  hedge_p20d_context: {
    label: "20日危机先验触发 hedge",
    summary: "20日危机先验越过 hedge 阈值，且触发层、外部层或事件层已经给出上下文确认。",
    kind: "trigger"
  },
  hedge_p60d_elevated: {
    label: "60日高位挤压到数周",
    summary: "60日先验已升高，结构脆弱性和外部冲击同步恶化，系统认为风险开始压缩到数周。",
    kind: "trigger"
  },
  hedge_carry_structural: {
    label: "JPY carry 叠加结构脆弱性",
    summary: "日元套息融资压力偏高，足以把原本的中期风险推到 hedge 档位。",
    kind: "trigger"
  },
  hedge_actionability: {
    label: "动作头确认 hedge",
    summary: "独立动作头提示未来几周的保护动作需要前置。",
    kind: "trigger"
  },
  prepare_p60d_structural: {
    label: "60日危机先验触发 prepare",
    summary: "60日危机先验超过 prepare 阈值，同时结构脆弱性已经明显抬升。",
    kind: "trigger"
  },
  prepare_structural_downgrade: {
    label: "结构脆弱性提前进入 prepare",
    summary: "即使 60 日先验还没到主阈值，但结构风险已经够高，系统先切到 prepare。",
    kind: "trigger"
  },
  prepare_external_structural: {
    label: "外部冲击放大 prepare",
    summary: "结构脆弱性还没到短端窗口，但外部放大器已经足够强，需要先做准备。",
    kind: "trigger"
  },
  prepare_carry_structural: {
    label: "JPY carry 提前进入 prepare",
    summary: "日元融资环境开始变紧，系统把它作为中期风险积累的放大器。",
    kind: "trigger"
  },
  prepare_actionability: {
    label: "动作头确认 prepare",
    summary: "独立动作头提示应该先准备现金、对冲工具和执行顺序。",
    kind: "trigger"
  },
  quality_blocked_hedge: {
    label: "数据质量阻断 hedge",
    summary: "原本存在 hedge 级信号，但当前数据质量太差，系统拒绝直接升级档位。",
    kind: "blocker"
  },
  preference_conservative_escalation: {
    label: "保守偏好上调档位",
    summary: "用户偏好更保守，系统把基础执行节奏再上调一档处理。",
    kind: "preference"
  },
  preference_aggressive_deescalation: {
    label: "进取偏好下调档位",
    summary: "用户偏好更进取，系统把基础执行节奏适度下调后再给出建议。",
    kind: "preference"
  },
  preference_neutral_no_adjustment: {
    label: "中性偏好未调整",
    summary: "用户偏好没有改变基础执行节奏。",
    kind: "preference"
  }
};

export function describePostureClause(code: string): PostureClauseDescriptor {
  return (
    POSTURE_CLAUSE_DESCRIPTORS[code] ?? {
      label: code,
      summary: "当前版本还没有为这个条款补充中文解释。",
      kind: "trigger"
    }
  );
}
