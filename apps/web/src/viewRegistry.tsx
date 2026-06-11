import {
  Activity,
  BadgeInfo,
  Database,
  GitCompareArrows,
  History,
  Layers3,
  Radar,
  ShieldCheck,
  Table2,
  type LucideIcon
} from "lucide-react";
import { lazy, type ReactElement } from "react";
import type { ConsoleReadyData } from "./useConsoleData";
import DecisionView from "./views/decision/DecisionView";

const DriversView = lazy(async () => import("./views/drivers/DriversView"));
const IndicatorsView = lazy(async () => import("./views/indicators/IndicatorsView"));
const SourcesView = lazy(async () => import("./views/sources/SourcesView"));
const MethodView = lazy(async () => import("./views/method/MethodView"));
const EventsView = lazy(async () => import("./views/events/EventsView"));
const BacktestsView = lazy(async () => import("./views/backtests/BacktestsView"));
const AuditView = lazy(async () => import("./views/audit/AuditView"));

export type View =
  | "decision"
  | "drivers"
  | "events"
  | "backtests"
  | "audit"
  | "indicators"
  | "sources"
  | "method";

export interface ViewNavItem {
  id: View;
  label: string;
  icon: LucideIcon;
  title: string;
  description: string;
}

export const navItems: ViewNavItem[] = [
  {
    id: "decision",
    label: "决策面板",
    icon: ShieldCheck,
    title: "美国金融危机风险决策面板",
    description: "把风险强度、危机概率、历史类比和数据可信度分层展示。"
  },
  {
    id: "drivers",
    label: "风险驱动",
    icon: Layers3,
    title: "风险驱动拆解",
    description: "查看哪些结构、触发和缓冲因子正在推高或压低当前风险。"
  },
  {
    id: "events",
    label: "事件确认",
    icon: Radar,
    title: "事件层确认",
    description: "查看最近事件信号、待补确认缺口，以及事件层如何影响当前执行节奏。"
  },
  {
    id: "backtests",
    label: "回测表现",
    icon: History,
    title: "历史回测与误报边界",
    description: "查看历史场景命中、动作提前量，以及非危机阶段的误报边界。"
  },
  {
    id: "audit",
    label: "版本核对",
    icon: GitCompareArrows,
    title: "线上版本与研究核对",
    description: "查看当前线上版本、历史预测快照、训练工件和研究核对摘要。"
  },
  {
    id: "indicators",
    label: "指标细项",
    icon: Table2,
    title: "指标细项总览",
    description: "逐项查看评分口径、历史分位、30 天变化和指标级质量。"
  },
  {
    id: "sources",
    label: "数据可信度",
    icon: Database,
    title: "数据可信度与免费源状态",
    description: "查看覆盖率、告警、免费数据源状态以及生产可用性约束。"
  },
  {
    id: "method",
    label: "方法说明",
    icon: BadgeInfo,
    title: "方法说明与版本边界",
    description: "解释危机概率、动作概率、运行阈值和当前线上版本的边界。"
  }
];

const viewRegistry: Record<View, (data: ConsoleReadyData) => ReactElement> = {
  decision: (data) => (
    <DecisionView
      assessment={data.assessment}
      history={data.assessmentHistory}
      method={data.method}
      posture={data.posture}
      overview={data.overview}
      backtests={data.backtests}
      indicators={data.indicators}
    />
  ),
  drivers: (data) => (
    <DriversView
      assessment={data.assessment}
      indicators={data.indicators}
      overview={data.overview}
      posture={data.posture}
    />
  ),
  events: (data) => <EventsView assessment={data.assessment} events={data.events} />,
  backtests: (data) => (
    <BacktestsView
      assessment={data.assessment}
      backtests={data.backtests}
      timeline={data.backtestTimeline}
    />
  ),
  audit: (data) => <AuditView assessment={data.assessment} audit={data.audit} />,
  indicators: (data) => <IndicatorsView indicators={data.indicators} />,
  sources: (data) => <SourcesView assessment={data.assessment} sources={data.sources} />,
  method: (data) => (
    <MethodView
      assessment={data.assessment}
      posture={data.posture}
      method={data.method}
    />
  )
};

export function renderActiveView(view: View, data: ConsoleReadyData) {
  return viewRegistry[view](data);
}
