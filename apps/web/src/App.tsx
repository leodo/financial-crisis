import { Suspense, useMemo, useState } from "react";
import {
  Activity,
  AlertTriangle,
  RefreshCw,
  RotateCcw,
  ServerCrash,
  ShieldCheck
} from "lucide-react";
import {
  formatDateTimeWithLocal,
  dataModeLabel,
  qualityLabel,
  timeBucketLabel
} from "./format";
import { probabilityDiagnosticAnomalyHorizons } from "./views/decision/probabilityDiagnostics";
import {
  currentMvpRiskState,
  mvpProbabilityInputIsAuditOnly,
  mvpRiskStateDisplayCopy,
  mvpRiskStateDisplayLabel
} from "./views/decision/mvpRiskState";
import {
  probabilityModelFinalSnapshotValue,
  probabilityRuntimeReferenceNote,
  probabilitySnapshotValue
} from "./views/decision/signalLayerBuilders";
import { useConsoleData, type ConsoleReadyData, type ConsoleDataSnapshot } from "./useConsoleData";
import { ErrorBoundary } from "./components/ErrorBoundary";
import { RecoveryPanel } from "./components/RecoveryPanel";
import { navItems, renderActiveView, type View } from "./viewRegistry";

const VIEW_DATA_KEYS: Record<View, Array<keyof ConsoleReadyData>> = {
  decision: ["assessment", "assessmentHistory", "method", "posture", "overview", "backtests", "indicators"],
  drivers: ["assessment", "indicators", "overview", "posture"],
  events: ["assessment", "events"],
  backtests: ["assessment", "backtests", "backtestTimeline"],
  audit: ["assessment", "audit"],
  indicators: ["indicators"],
  sources: ["assessment", "sources"],
  method: ["assessment", "posture", "method"]
};

const DATASET_LABELS: Record<keyof ConsoleReadyData, string> = {
  assessment: "当前评估快照",
  assessmentHistory: "概率轨迹",
  posture: "执行节奏建议",
  method: "方法与版本说明",
  audit: "版本核对",
  overview: "维度总览",
  indicators: "指标细项",
  events: "事件确认",
  sources: "数据源状态",
  backtests: "历史回测摘要",
  backtestTimeline: "滚动回测轨迹"
};

function isView(value: string | null): value is View {
  return value !== null && navItems.some((item) => item.id === value);
}

function initialViewFromLocation(): View {
  if (typeof window === "undefined") {
    return "decision";
  }

  const requestedView = new URLSearchParams(window.location.search).get("view");
  return isView(requestedView) ? requestedView : "decision";
}

function buildReadyData(
  view: View,
  data: ConsoleDataSnapshot
): ConsoleReadyData | null {
  const requiredKeys = VIEW_DATA_KEYS[view];
  if (requiredKeys.some((key) => data[key] === undefined)) {
    return null;
  }

  return data as ConsoleReadyData;
}

function firstQueryError(
  data: ConsoleDataSnapshot,
  queryErrors: Partial<Record<keyof ConsoleReadyData, unknown>>,
  view: View
): unknown {
  const requiredKeys = VIEW_DATA_KEYS[view];
  return requiredKeys
    .map((key) => queryErrors[key])
    .find((value) => value !== null && value !== undefined);
}

function formatErrorText(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }

  if (typeof error === "string" && error.trim().length > 0) {
    return error;
  }

  return "未知错误";
}

function productionSourceIssueLabels(sources: ConsoleDataSnapshot["sources"]): string[] {
  return (
    sources
      ?.filter(
        (source) =>
          source.production_allowed &&
          ["delayed", "partial_failure", "failed"].includes(source.health.status)
      )
      .map((source) => source.display_name) ?? []
  );
}

export default function App() {
  const [view, setView] = useState<View>(() => initialViewFromLocation());
  const requiredKeys = VIEW_DATA_KEYS[view];
  const {
    assessment,
    queries,
    data,
    reload
  } = useConsoleData(requiredKeys);
  const activeNavItem = navItems.find((item) => item.id === view) ?? navItems[0];
  const queryStateByKey: Record<
    keyof ConsoleReadyData,
    {
      isPending: boolean;
      isError: boolean;
    }
  > = {
    assessment: queries.assessment,
    assessmentHistory: queries.assessmentHistory,
    posture: queries.posture,
    method: queries.method,
    audit: queries.audit,
    overview: queries.overview,
    indicators: queries.indicators,
    events: queries.events,
    sources: queries.sources,
    backtests: queries.backtests,
    backtestTimeline: queries.backtestTimeline
  };
  const queryErrors: Partial<Record<keyof ConsoleReadyData, unknown>> = {
    assessment: queries.assessment.error,
    assessmentHistory: queries.assessmentHistory.error,
    posture: queries.posture.error,
    method: queries.method.error,
    audit: queries.audit.error,
    overview: queries.overview.error,
    indicators: queries.indicators.error,
    events: queries.events.error,
    sources: queries.sources.error,
    backtests: queries.backtests.error,
    backtestTimeline: queries.backtestTimeline.error
  };
  const readyData = buildReadyData(view, data);
  const viewError = firstQueryError(data, queryErrors, view);
  const hasViewError = viewError !== null && viewError !== undefined;
  const isViewLoading = !readyData && !viewError;
  const showDecisionWarmup =
    view === "decision" && Boolean(assessment.data) && !readyData && !viewError;
  const loadProgress = requiredKeys.map((key) => ({
    key,
    label: DATASET_LABELS[key],
    ready: data[key] !== undefined,
    pending: queryStateByKey[key].isPending,
    error: queryStateByKey[key].isError
  }));
  const readyCount = loadProgress.filter((item) => item.ready).length;
  const pendingLabels = loadProgress.filter((item) => !item.ready).map((item) => item.label);
  const errorText = formatErrorText(viewError);
  const assessmentErrorText = formatErrorText(queries.assessment.error);
  const healthErrorText = formatErrorText(queries.systemHealth.error);
  const isAssessmentUnavailable = !assessment.data && queries.assessment.isError;
  const systemHealthOk = queries.systemHealth.data?.status === "ok";
  const latestLagDays =
    assessment.data?.runtime.latest_key_indicator_lag_business_days ??
    assessment.data?.runtime.latest_observation_lag_business_days ??
    assessment.data?.runtime.latest_key_indicator_lag_days ??
    assessment.data?.runtime.latest_observation_lag_days ??
    null;
  const hasDataLag =
    !!assessment.data?.runtime.stale_warning && !assessment.data?.runtime.demo_mode;
  const probabilityAnomalyHorizons = useMemo(
    () => (assessment.data ? probabilityDiagnosticAnomalyHorizons(assessment.data) : []),
    [assessment.data]
  );
  const probabilityAuditOnly = assessment.data
    ? mvpProbabilityInputIsAuditOnly(assessment.data)
    : false;
  const mvpRiskState = assessment.data ? currentMvpRiskState(assessment.data) : null;
  const sourceIssueLabels = useMemo(
    () => productionSourceIssueLabels(data.sources),
    [data.sources]
  );
  const sourceIssueSummary =
    sourceIssueLabels.length > 0
      ? `生产源健康降级 ${sourceIssueLabels.length}：${sourceIssueLabels.join("、")}`
      : null;
  const runtimeReferenceNote = assessment.data
    ? probabilityRuntimeReferenceNote(assessment.data)
    : null;
  const riskWindowDisplayLabel = assessment.data
    ? probabilityAuditOnly
      ? mvpRiskStateDisplayLabel(mvpRiskState?.label ?? "观察为主")
      : timeBucketLabel(assessment.data.time_to_risk_bucket)
    : "—";
  const riskWindowSummaryLabel =
    probabilityAuditOnly
      ? `MVP 风险状态 ${mvpRiskStateDisplayLabel(mvpRiskState?.label ?? "观察为主")}（${
          probabilityAnomalyHorizons.length > 0
            ? `${probabilityAnomalyHorizons.join(" / ")} 正式读数偏低`
            : "正式概率仅作参考"
        }）`
      : assessment.data
        ? `风险时距 ${timeBucketLabel(assessment.data.time_to_risk_bucket)}`
        : "风险时距 —";
  const statusSummary = useMemo(() => {
    if (!assessment.data) {
      return null;
    }

    return [
      riskWindowSummaryLabel,
      `关键数据覆盖 ${qualityLabel(assessment.data.data_trust.quality_grade)}`,
      sourceIssueSummary,
      `关键数据 ${assessment.data.runtime.latest_key_indicator_at ?? assessment.data.runtime.latest_observation_at ?? "—"}`
    ].filter((item): item is string => item !== null);
  }, [assessment.data, riskWindowSummaryLabel, sourceIssueSummary]);
  const handleViewChange = (nextView: View) => {
    setView(nextView);
    if (typeof window === "undefined") {
      return;
    }

    const url = new URL(window.location.href);
    if (nextView === "decision") {
      url.searchParams.delete("view");
    } else {
      url.searchParams.set("view", nextView);
    }
    window.history.replaceState(null, "", `${url.pathname}${url.search}${url.hash}`);
  };

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand">
          <ShieldCheck size={22} />
          <div>
            <strong>金融危机概率评估</strong>
            <span>美国风险决策控制台</span>
          </div>
        </div>

        <nav className="nav">
          {navItems.map((item) => {
            const Icon = item.icon;
            return (
              <button
                key={item.id}
                className={view === item.id ? "nav-item active" : "nav-item"}
                onClick={() => handleViewChange(item.id)}
                type="button"
                title={item.label}
              >
                <Icon size={18} />
                <span>{item.label}</span>
              </button>
            );
          })}
        </nav>

        <section className="sidebar-note">
          <strong>这不是自动交易指令</strong>
          <span>面板输出的是系统评估、风险时距和执行节奏，不替代个性化仓位决策。</span>
        </section>
      </aside>

      <main className="main">
        <header className="topbar">
          <div className="topbar-main">
            <div className="topbar-title-row">
              <div>
                <h1>{activeNavItem.title}</h1>
                <p>{activeNavItem.description}</p>
              </div>
              <button
                className={reload.isPending ? "icon-button spinning" : "icon-button"}
                disabled={reload.isPending}
                onClick={() => {
                  reload.mutate();
                }}
                title="重新加载本地库并刷新面板"
                type="button"
              >
                <RefreshCw size={16} />
              </button>
            </div>
            {assessment.data ? (
              <div className="meta-strip">
                <span>评估口径日期 {assessment.data.as_of_date}</span>
                <span>数据模式 {dataModeLabel(assessment.data.runtime.data_mode)}</span>
                <span>
                  关键数据{" "}
                  {assessment.data.runtime.latest_key_indicator_at ??
                    assessment.data.runtime.latest_observation_at ??
                    "—"}
                </span>
                <span>生成时间 {formatDateTimeWithLocal(assessment.data.runtime.generated_at)}</span>
                <span>风险时距 {riskWindowDisplayLabel}</span>
                <span>关键数据覆盖 {qualityLabel(assessment.data.data_trust.quality_grade)}</span>
                {sourceIssueSummary ? <span>{sourceIssueSummary}</span> : null}
              </div>
            ) : null}
          </div>
        </header>

        {isAssessmentUnavailable && (
          <RecoveryPanel
            actions={[
              {
                label: reload.isPending ? "正在重新加载本地库…" : "重新加载本地库",
                onClick: () => {
                  reload.mutate();
                },
                disabled: reload.isPending,
                tone: "primary"
              },
              {
                label: "整页刷新",
                onClick: () => {
                  window.location.reload();
                }
              }
            ]}
            details={[
              systemHealthOk
                ? "API 健康检查可达，但当前评估接口没有返回可用快照。"
                : "前端当前没有确认到本地 API 健康响应，服务可能未启动或已卡住。",
              `评估接口错误：${assessmentErrorText}`,
              queries.systemHealth.isError ? `健康检查错误：${healthErrorText}` : "健康检查：/health 正常响应",
              "若问题反复出现，先执行 `just status`，必要时再执行 `just dev` 重新拉起服务。"
            ]}
            footer="面板不会自动推断缺失结论；拿不到当前评估快照时，只展示恢复方式和诊断信息。"
            icon={<ServerCrash size={18} />}
            summary="当前面板没有拿到评估快照，因此不继续渲染结论型视图，避免把空数据误当成低风险。"
            title="本地评估服务当前不可用"
            tone="error"
          />
        )}

        {assessment.data && hasDataLag && view !== "decision" && (
          <RecoveryPanel
            details={[
              `当前评估依赖的关键市场指标按工作日口径约落后 ${latestLagDays ?? "—"} 天。`,
              ...(statusSummary ?? []),
              "这套系统是免费日频/周频预警面板，不是盘中行情终端；近端风险判断必须保守解释。"
            ]}
            footer="如果你刚补完数据，可以用右上角刷新按钮重新载入本地库。"
            icon={<AlertTriangle size={18} />}
            summary="页面虽然可用，但当前数据时效性不足，短期决策不要把低概率直接理解成风险消失。"
            title="当前数据存在时效性提醒"
            tone="warning"
          />
        )}

        {assessment.data && probabilityAuditOnly && !isAssessmentUnavailable && (
          <section className="notice">
            <strong>当前处于参考态</strong>
            <div>
              当前正式概率只作为参考输入，动作信号和预算边界也只作为辅助参考；
              当前主结论优先看 MVP 风险状态 {mvpRiskStateDisplayLabel(mvpRiskState?.label ?? "观察为主")}。
              {probabilityAnomalyHorizons.length > 0
                ? ` 当前 ${probabilityAnomalyHorizons.join(" / ")} 命中模型方向异常。`
                : ""}
            </div>
          </section>
        )}

        {showDecisionWarmup && assessment.data && (
        <section className="warmup-panel" aria-live="polite">
            <div className="warmup-head">
              <div className="loading-state-icon" aria-hidden="true">
                <Activity size={18} />
              </div>
              <div className="warmup-copy">
                <strong>评估快照已返回，决策面板继续补齐其余模块</strong>
                <p>
                  当前不是白屏或无数据，而是明细模块还在加载。先看总风险、危机先验和执行节奏，
                  其余图表会继续补齐。
                </p>
              </div>
            </div>
            <div className="warmup-metric-grid">
              <div className="warmup-metric">
                <span>总风险强度</span>
                <strong>{assessment.data.scores.overall_score.toFixed(1)}</strong>
                <small>先看压力位置，不等于危机概率</small>
              </div>
              <div className="warmup-metric">
                <span>{probabilityAuditOnly ? "参考概率" : "危机先验"}</span>
                <strong>
                  {probabilityAuditOnly
                    ? "规则层优先"
                    : probabilitySnapshotValue(assessment.data.probabilities)}
                </strong>
                <small>
                  {probabilityAuditOnly
                    ? `当前页面参考值 ${probabilitySnapshotValue(
                        assessment.data.probabilities
                      )} 仅作参考值，当前不单独拿来判断风险时距。${
                        runtimeReferenceNote ? ` ${runtimeReferenceNote}` : ""
                      }`
                    : runtimeReferenceNote
                      ? `当前页面值 ${probabilitySnapshotValue(
                          assessment.data.probabilities
                        )}；模型原始输出 ${probabilityModelFinalSnapshotValue(
                          assessment.data
                        )}。`
                    : "5d / 20d / 60d"}
                </small>
              </div>
              <div className="warmup-metric">
                <span>当前执行节奏</span>
                <strong>
                  {probabilityAuditOnly
                    ? mvpRiskStateDisplayLabel(mvpRiskState?.label ?? "观察为主")
                    : timeBucketLabel(assessment.data.time_to_risk_bucket)}
                </strong>
                <small>
                  {probabilityAuditOnly
                    ? mvpRiskStateDisplayCopy(mvpRiskState?.summary ?? "") ||
                      `${probabilityAnomalyHorizons.join(
                        " / "
                      )} 概率读数偏低；完整面板会显示参考说明。`
                    : assessment.data.posture_reason}
                </small>
              </div>
              <div className="warmup-metric">
                <span>最新关键观测</span>
                <strong>
                  {assessment.data.runtime.latest_key_indicator_at ??
                    assessment.data.runtime.latest_observation_at ??
                    "—"}
                </strong>
                <small>
                  {assessment.data.runtime.stale_warning ?? "最新观测时间正常。"}
                </small>
              </div>
            </div>
            <div className="loading-state-grid">
              {loadProgress.map((item) => (
                <div
                  key={item.key}
                  className={
                    item.ready
                      ? "loading-chip ready"
                      : item.error
                        ? "loading-chip error"
                        : "loading-chip pending"
                  }
                >
                  <span>{item.label}</span>
                  <strong>{item.ready ? "已就绪" : item.error ? "失败" : "加载中"}</strong>
                </div>
              ))}
            </div>
            <small className="loading-state-footer">
              {pendingLabels.length > 0
                ? `仍在等待：${pendingLabels.join("、")}。若超过 10 秒仍未进入完整面板，先执行 just status，再点右上角刷新。`
                : "页面已经拿到全部模块，正在进入完整视图。"}
            </small>
          </section>
        )}

        {isViewLoading && !showDecisionWarmup && (
          <section className="loading-state-panel" aria-live="polite">
            <div className="loading-state-head">
              <div className="loading-state-icon" aria-hidden="true">
                <Activity size={18} />
              </div>
              <div className="loading-state-copy">
                <strong>{activeNavItem.label}已启动，正在读取数据</strong>
                <p>
                  当前视图需要 {requiredKeys.length} 组数据，已就绪 {readyCount} 组。
                  首次启动本地服务、刚刷新 SQLite，或者 API 刚完成热重启时，
                  页面会先显示这个启动面板，再进入完整视图。
                </p>
              </div>
            </div>
            <div className="loading-state-grid">
              {loadProgress.map((item) => (
                <div
                  key={item.key}
                  className={
                    item.ready
                      ? "loading-chip ready"
                      : item.error
                        ? "loading-chip error"
                        : "loading-chip pending"
                  }
                >
                  <span>{item.label}</span>
                  <strong>{item.ready ? "已就绪" : item.error ? "失败" : "加载中"}</strong>
                </div>
              ))}
            </div>
            <small className="loading-state-footer">
              {pendingLabels.length > 0
                ? `仍在等待：${pendingLabels.join("、")}。如果超过 10 秒仍未进入完整页面，先执行 just status，再点右上角刷新。`
                : "页面已经启动，正在拼装当前视图。"}
            </small>
          </section>
        )}
        {hasViewError && readyData && <div className="notice error">当前视图存在部分接口错误：{errorText}</div>}
        {reload.isError && (
          <div className="notice error">重新加载本地库失败，请检查 API 日志或数据源状态。</div>
        )}

        {!readyData && hasViewError && !isAssessmentUnavailable && (
          <RecoveryPanel
            actions={[
              {
                label: "重试当前视图",
                onClick: () => {
                  queries.assessment.refetch();
                  reload.mutate();
                },
                disabled: reload.isPending,
                tone: "primary"
              },
              ...(view === "decision"
                ? []
                : [
                    {
                      label: "切回决策面板",
                      onClick: () => {
                        setView("decision");
                      }
                    }
                  ]),
              {
                label: "整页刷新",
                onClick: () => {
                  window.location.reload();
                }
              }
            ]}
            details={[
              `当前视图：${activeNavItem.label}`,
              `接口错误：${errorText}`,
              "这通常是局部接口失败或数据结构暂时不完整，不代表系统已经得出低风险结论。 "
            ]}
            footer="如果只有某个标签页反复失败，优先检查对应 API 返回和该视图最近的字段变更。"
            icon={<AlertTriangle size={18} />}
            summary="当前标签页缺少必要数据，面板暂停渲染该视图，避免把不完整结果拼成正常页面。"
            title={`${activeNavItem.label} 当前无法完整显示`}
            tone="error"
          />
        )}

        {readyData && !isAssessmentUnavailable && (
          <ErrorBoundary
            fallback={({ error, reset }) => (
              <RecoveryPanel
                actions={[
                  {
                    label: "重试当前视图",
                    onClick: () => {
                      reset();
                    },
                    tone: "primary"
                  },
                  ...(view === "decision"
                    ? []
                    : [
                        {
                          label: "切回决策面板",
                          onClick: () => {
                            setView("decision");
                            reset();
                          }
                        }
                      ]),
                  {
                    label: "整页刷新",
                    onClick: () => {
                      window.location.reload();
                    }
                  }
                ]}
                details={[
                  `当前视图：${activeNavItem.label}`,
                  `渲染异常：${formatErrorText(error)}`,
                  "这是前端渲染层错误，不会直接说明当前风险高低。"
                ]}
                footer="其他标签页和刷新按钮仍然可用；这类错误通常意味着某个页面没有兼容新的字段形态。"
                icon={<RotateCcw size={18} />}
                summary="当前标签页在处理数据时抛出了前端异常，系统已阻断整页白屏并切换到恢复面板。"
                title={`${activeNavItem.label} 渲染失败`}
                tone="error"
              />
            )}
            resetKey={`${view}:${assessment.data?.as_of_date ?? "unknown"}`}
          >
            <Suspense fallback={<div className="notice">正在加载视图…</div>}>
              {renderActiveView(view, readyData)}
            </Suspense>
          </ErrorBoundary>
        )}
      </main>
    </div>
  );
}
