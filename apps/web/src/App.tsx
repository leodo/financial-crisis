import { Suspense, useState } from "react";
import {
  RefreshCw,
  ShieldCheck
} from "lucide-react";
import {
  formatDateTime,
  dataModeLabel,
  qualityLabel,
  timeBucketLabel
} from "./format";
import { useConsoleData } from "./useConsoleData";
import { navItems, renderActiveView, type View } from "./viewRegistry";

export default function App() {
  const [view, setView] = useState<View>("decision");
  const {
    assessment,
    reload,
    isLoading,
    error,
    readyData
  } = useConsoleData();
  const errorText = error instanceof Error ? error.message : "未知错误";
  const activeNavItem = navItems.find((item) => item.id === view) ?? navItems[0];

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
                onClick={() => setView(item.id)}
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
                <span>评估日期 {assessment.data.as_of_date}</span>
                <span>数据模式 {dataModeLabel(assessment.data.runtime.data_mode)}</span>
                <span>最近数据 {assessment.data.runtime.latest_observation_at ?? "—"}</span>
                <span>生成时间 {formatDateTime(assessment.data.runtime.generated_at)}</span>
                <span>风险时距 {timeBucketLabel(assessment.data.time_to_risk_bucket)}</span>
                <span>可信度 {qualityLabel(assessment.data.data_trust.quality_grade)}</span>
              </div>
            ) : null}
          </div>
        </header>

        {isLoading && <div className="notice">正在加载评估数据…</div>}
        {error && <div className="notice error">API 请求失败：{errorText}</div>}
        {reload.isError && (
          <div className="notice error">重新加载本地库失败，请检查 API 日志或数据源状态。</div>
        )}

        {readyData && (
          <Suspense fallback={<div className="notice">正在加载视图…</div>}>
            {renderActiveView(view, readyData)}
          </Suspense>
        )}
      </main>
    </div>
  );
}
