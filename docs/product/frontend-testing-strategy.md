# 前端测试策略

状态：`Draft`

最后更新：2026-06-30

## 1. 目标

前端测试的目的是**保护关键用户流程不被无意破坏**，不是追求 100% 覆盖率。优先级：

1. 纯函数逻辑（数据转换、格式校验、口径说明）— 无副作用，测试成本最低，收益最高
2. 组件渲染路径（loading / empty / error / 正常数据）— 捕获 UI 回归
3. 数据流集成（API 返回 → 状态更新 → 视图渲染）— 验证完整链路

## 2. 测试分层

```
分层          工具                             覆盖目标
─────────────────────────────────────────────────────────────
纯函数测试    Vitest                            工具函数、数据转换、口径文案
组件测试      Vitest + @testing-library/react   渲染状态、用户交互
集成测试      Vitest + MSW                      数据流：API → Hook → 组件
E2E 测试      Playwright（后续按需引入）          关键用户旅程
```

当前已落地：**第一层（纯函数）**，47 个测试。

### 2.1 纯函数测试（已起步）

覆盖范围：
- `api.ts` — `normalizeAssessmentSnapshot` 的 governance 合并逻辑
- `App.tsx` — `formatErrorText`、`isView`、`firstQueryError`、`productionSourceIssueLabels`
- `DecisionView.tsx` — `splitAuditNote`、`splitAuditDetail`、`auditSegmentClassName`、`changeValueToneClassName`、`numberAuditToneClass`

写法规范：
```typescript
// 1. 测试文件与源文件同级，后缀 .test.ts
// 2. 被测试函数如果未 export，在测试文件中重新定义
// 3. 一个 describe 对应一个函数
// 4. it() 描述行为而非实现
describe("formatErrorText", () => {
  it("returns Error.message for Error instances", () => {
    expect(formatErrorText(new Error("API failed"))).toBe("API failed");
  });
  it("returns fallback for null/undefined", () => {
    expect(formatErrorText(undefined)).toBe("未知错误");
  });
});
```

### 2.2 组件测试（下一步）

每个视图组件覆盖 4 种状态：

| 状态 | 测试内容 |
|------|---------|
| Loading | 显示加载指示器，不崩溃 |
| Empty / 缺数据 | 显示缺省提示，不渲染空列表 |
| Error | 显示错误信息 + 重试按钮 |
| 正常数据 | 渲染关键数据点和文案 |

工具：`@testing-library/react` + `@testing-library/user-event`

```typescript
// 示例
render(<DecisionHeroSummary data={mockData} loading={false} />);
expect(screen.getByText("当前是否危险")).toBeInTheDocument();
expect(screen.getByText("正常")).toBeInTheDocument();
```

### 2.3 集成测试（后续）

用 MSW（Mock Service Worker）拦截 fetch 请求，测试完整数据流：
```
API 返回 mock 数据 → React Query 更新 → 组件重新渲染 → 断言 DOM
```

### 2.4 E2E 测试（暂缓）

在全栈启动后用 Playwright 走完整用户流程。当前阶段不优先做。

## 3. 哪些先测

按优先级排序：

1. **决策面板（DecisionView）** — 首页首屏，最影响用户体验
2. **加载状态（App.tsx 的 LoadingState）** — 刚才修复的路径，需要回归保护
3. **API 层（api.ts）** — normalizeAssessmentSnapshot、超时处理
4. **数字审计口径（NumberAuditRows）** — 纯渲染逻辑，容易遗漏文案
5. **各视图的 loading/error 状态** — 覆盖率高但改动频率低

## 4. 运行方式

```bash
# 单次运行
npm run test

# watch 模式（开发时使用）
npm run test:watch

# 通过 just
just web-test
just web-test-watch
```

测试在 `check-all` 门禁中自动执行，与 Rust 测试、前端构建同级。

## 5. 不做什么

- 不 mock 具体组件内部实现细节（如 state 变量名）
- 不测试第三方库（ECharts、React Table）的渲染正确性
- 不为了覆盖率写测试（如为 getter/setter 写 trivial 测试）
- 不依赖真实 API —— 所有外部请求必须 mock 或 stub
