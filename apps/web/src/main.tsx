import React from "react";
import ReactDOM from "react-dom/client";
import { AlertTriangle } from "lucide-react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import App from "./App";
import { ErrorBoundary } from "./components/ErrorBoundary";
import { RecoveryPanel } from "./components/RecoveryPanel";
import "./styles.css";

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      refetchInterval: 60_000,
      retry: 1
    }
  }
});

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <ErrorBoundary
      fallback={({ error, reset }) => (
        <div className="app-root-fallback">
          <RecoveryPanel
            actions={[
              {
                label: "重试前端渲染",
                onClick: () => {
                  reset();
                },
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
              `前端异常：${error.message}`,
              "这属于前端根级渲染错误，通常和最近的字段变更、懒加载视图或状态边界有关。"
            ]}
            footer="如果刷新后仍反复出现，先看浏览器控制台，再检查最近的 Web 端变更。"
            icon={<AlertTriangle size={18} />}
            summary="系统已经阻断整页白屏，并保留了重试与刷新入口。"
            title="前端渲染异常"
            tone="error"
          />
        </div>
      )}
    >
      <QueryClientProvider client={queryClient}>
        <App />
      </QueryClientProvider>
    </ErrorBoundary>
  </React.StrictMode>
);
