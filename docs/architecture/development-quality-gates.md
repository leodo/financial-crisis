# 开发质量门禁

状态：`Review`

最后更新：2026-06-04

## 1. 目的

本文档只回答一个问题：后续开发如何继续推进，同时避免三类常见失控：

1. 活跃任务分散在多份 TODO 中，真相源不唯一；
2. 重要改动没有统一审查门禁，回归风险靠人脑记忆；
3. 研究结论、运行逻辑和 UI 解释逐步漂移，各说各话。

这份文档不是替代模型设计，也不是替代工程治理方案。它只定义：

- 哪两份 TODO 是活跃真相源；
- 哪类改动必须先补设计；
- 每类改动至少要过哪些本地和 CI 检查；
- 哪类候选必须补 release review 证据。

## 2. 活跃真相源

从现在开始，仓库里只有两份活跃 TODO 可以继续新增“当前任务”：

1. 模型/数据/回测主线：
   - [危机概率评估设计 TODO](../roadmap/crisis-probability-design-todo.md)
2. 工程结构/模块边界/质量治理主线：
   - [工程维护性 TODO](../roadmap/engineering-maintainability-todo.md)

其余 roadmap 文档的角色如下：

- [设计 TODO 总索引](../roadmap/design-todo.md)：历史索引，不再承载新主线任务；
- [第二轮细分设计清单](../roadmap/second-round-design-backlog.md)：历史 backlog，不再承载当前活跃任务；
- [SQLite 历史数据实现路线](../roadmap/sqlite-historical-data-implementation-plan.md)：专项实施背景，若产生当前任务，必须镜像回两份活跃 TODO 之一。

规则：

1. 任何新任务都必须先决定归属到“模型主线”或“工程治理主线”。
2. 不允许把当前活跃任务只写在旧 backlog、临时笔记或提交说明里。
3. 一条任务如果同时影响模型和工程，可以在两份活跃 TODO 中互相引用，但必须有唯一主归属。

## 3. 先补设计，再写代码的触发条件

出现以下任一情况时，先补设计文档，再编码：

1. 修改危机标签、horizon 概率定义、episode 目标或 Go/No-Go 口径；
2. 修改 point-in-time 可见性、feature contract、API contract、SQLite schema；
3. 引入新的 family head、overlay、训练目标、校准策略或 release promotion 规则；
4. 改动会同时影响 worker 训练、API runtime、Web 解释三层事实来源；
5. 改动会新增一个新的大页面、研究工作流或跨模块目录边界。

如果改动只是：

- 文案修正；
- 局部 bug 修复；
- 已有设计内的小范围实现；
- 低风险重构且行为不变；

则可以直接编码，但仍要更新对应 TODO 状态。

## 4. 改动分级与必过门禁

| 改动类型 | 例子 | 必补文档 | 本地必跑 | 额外证据 |
| --- | --- | --- | --- | --- |
| 文档/说明层 | 修正文案、补解释 | 对应文档或 TODO | 无强制代码检查 | 无 |
| Rust 代码层 | API、worker、domain、storage | 若改变语义则补设计 | `just verify` | 无 |
| Web 展示层 | view、格式化、解释文案 | 若改信息架构则补产品设计 | `just verify` | 宽表/布局变更需自测页面 |
| 共享契约层 | API DTO、schema、feature contract | 必补设计/契约文档 | `just verify` | 说明兼容性与迁移影响 |
| 候选模型/阈值层 | 新 bundle、目标函数、校准策略 | 必补研究设计或 TODO 结论 | `just verify` | `just release-review-fast <candidate>` |
| 正式晋升层 | active release 替换、Go/No-Go | 必补 release/review 结论 | `just verify` | `just release-review <candidate>` |

补充规则：

1. `release-review-fast` 只负责方向性 triage，不能替代正式放行；
2. 涉及候选训练但不打算晋升的实验，也必须把 No-Go 结论写回文档或 TODO，避免后面重复踩坑；
3. 如果修改影响“训练口径”和“运行口径”，提交前必须确认是否应收敛到共享逻辑，而不是只修一边。

## 5. 当前统一本地门禁

默认本地门禁命令：

```text
just verify
```

当前包含：

1. `verify-artifacts`
   - 审计版本化 artifact 目录，避免未审计研究副产物混入提交流程；
2. `fmt-check`
   - 检查 Rust 代码格式，不直接改文件；
3. `test`
   - 运行 Rust workspace 测试；
4. `lint`
   - 运行 Rust clippy，warning 即失败；
5. `web-build`
   - 构建前端生产包，检查类型与打包可用性。

如果只想快速做开发中自检，可以先跑：

```text
just check-all
```

但提交前仍以 `just verify` 为准。

MVP 决策面板的运行时数字可信检查需要本地服务，因此不放进默认 CI/`just verify`。凡是修改首屏核心数字、`mvp_risk_state`、关键指标新鲜度、正式概率审计态或数据模式展示，都必须先启动 `just dev`，再运行：

```text
just mvp-regression
```

该命令会读取 `/api/assessment/current`，并检查当前 API 是否使用 SQLite、本地 USDJPY 是否与 JPY carry 模块一致、关键近端指标是否存在，以及 USDJPY tail 语义异常时是否降级为 `audit_only`。

## 6. CI 门禁

仓库 CI 至少应自动执行：

1. `cargo fmt --all -- --check`
2. `cargo test --workspace`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `npm ci && npm run build`（`apps/web`）

原则：

1. CI 与本地门禁保持同一套事实来源；
2. 本地通过但 CI 失败，视为未通过；
3. 新增语言/子系统时，必须先接入这套门禁，再继续扩展功能。

## 7. 审查与证据回写

以下改动完成后，必须把结论写回文档或 TODO，而不是只留在聊天记录里：

1. 一条实验路径被证伪；
2. 一条 release review 得到明确的 No-Go/Go 结论；
3. 某个 family split、threshold、bridge、target design 被确认无效；
4. 某项工程治理措施已经落地完成。

回写位置规则：

- 模型/实验结论：`crisis-probability-design-todo.md` 或相关 analytics 设计文档；
- 工程/结构治理结论：`engineering-maintainability-todo.md` 或 architecture 治理文档。

## 8. 完成定义

当以下条件同时满足时，可以认为“治理闭环已落地”：

1. 活跃 TODO 真相源只有两份，并持续保持；
2. `just verify` 能覆盖本地提交前的统一质量门禁；
3. CI 自动执行与本地一致的核心检查；
4. release candidate 的 triage / formal review 有明确命令和文档回写要求；
5. 新功能不再绕开设计、TODO、门禁和证据回写直接落地。
