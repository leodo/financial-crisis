# 开源项目参考

状态：`Draft`

检索日期：2026-05-30

本文档记录可参考的开源项目。它们不一定适合作为主项目基础，但可以借鉴数据源接入、风险面板、模型方法和工程组织方式。

## 1. 重点参考

### 1.1 Equibles

链接：[daniel3303/Equibles](https://github.com/daniel3303/Equibles)

定位：

- 自托管金融数据平台。
- 项目描述包含 SEC filings、FINRA short interest、FRED、Yahoo Finance、CFTC、CBOE、PostgreSQL、Web UI、MCP server 等能力。

可借鉴：

- 免费和公开金融数据源的连接器范围。
- 本地优先和自托管部署思路。
- 数据抓取状态、数据查看、API 和 Web UI 的组合。
- 将多个异构数据源统一到本地数据库的方式。

注意事项：

- 技术栈不是 Rust 主导。
- 项目目标是金融数据平台，不是金融危机预警系统。
- 需要逐项检查 license、数据源条款和抓取方式。

建议动作：

- 第二轮优先阅读其数据源连接器结构。
- 提取可复用的数据源清单和抓取状态模型。
- 不直接复制代码，先借鉴设计。

### 1.2 OpenBB

链接：[OpenBB-finance/OpenBB](https://github.com/OpenBB-finance/OpenBB)

定位：

- 金融数据平台和投资研究工具。
- 覆盖多种数据提供商和金融资产类型。

可借鉴：

- provider abstraction。
- 数据命名、查询参数和结果模型。
- 多数据源集成策略。

注意事项：

- 系统规模较大，直接作为基础会引入大量复杂度。
- 它不是金融危机预警面板。
- 需要检查项目 license 和依赖边界。

建议动作：

- 参考其 provider/endpoint 抽象，不照搬整体架构。

### 1.3 Canairy

链接：[manavpthaker/canairy](https://github.com/manavpthaker/canairy)

定位：

- 风险信号和预警面板。
- 包含 React/TypeScript 前端、Flask 后端、多个数据采集器。

可借鉴：

- 预警面板的产品形态。
- 风险灯号、信号列表、趋势展示。
- 多类数据采集器组织方式。

注意事项：

- 定位偏家庭和社会风险准备，不是专业金融危机预警。
- 项目成熟度和维护状态需要进一步评估。

建议动作：

- 参考页面结构和数据采集目录组织。

## 2. 模型和方法参考

### 2.1 psymonitor

链接：[itamarcaspi/psymonitor](https://github.com/itamarcaspi/psymonitor)

定位：

- R 包，用于实时监测金融市场泡沫。
- 基于 Phillips-Shi-Yu 相关方法。

可借鉴：

- 资产泡沫检测方法。
- 回测和可视化方式。

注意事项：

- 它是模型工具，不是完整系统。
- 语言是 R，适合作为研究参考。

### 2.2 SystemicRisk

链接：[TommasoBelluzzo/SystemicRisk](https://github.com/TommasoBelluzzo/SystemicRisk)

定位：

- MATLAB 系统性风险分析框架。
- 涵盖多种系统性风险指标和泡沫检测方法。

可借鉴：

- 系统性风险指标体系。
- 风险传播和市场压力度量方法。

注意事项：

- 不是现代 Web 系统。
- 不适合直接作为工程主干。

## 3. 原型数据工具

### 3.1 yfinance

链接：[ranaroussi/yfinance](https://github.com/ranaroussi/yfinance)

定位：

- Yahoo Finance 数据下载工具。

可借鉴：

- 原型期快速拉取市场价格。
- 用于验证指标计算和面板展示。

注意事项：

- 非官方 Yahoo Finance API。
- 不建议作为生产系统强依赖。
- 使用前必须检查 Yahoo 数据条款。

## 4. 参考方式

推荐采用“借鉴设计，不直接绑定”的方式：

- 数据源清单可以参考 Equibles 和 OpenBB。
- 数据连接器抽象可以参考 OpenBB。
- 预警面板形态可以参考 Canairy。
- 泡沫和系统性风险指标可以参考 psymonitor 和 SystemicRisk。
- 市场价格原型可以临时使用 yfinance，但生产接口应可替换。

第二轮重点应放在 Equibles 的免费数据源连接器结构和 OpenBB 的 provider abstraction。

