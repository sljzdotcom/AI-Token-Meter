# Antigravity Claude/GPT 共享额度详情

**需求：** REQ-20261009-002  
**现场基线：** 0.10.7/build35、Antigravity CLI 1.3.1；配额截图为 2026-10-08 23:26（Asia/Singapore）。本地变更未发布。  
**工作区：** `/private/tmp/AI-Meter-safe-auto-refresh`  
**设计：** [规格](../design/specifications/2026-10-09-antigravity-shared-model-quota-design.md) · [计划](../design/implementation-plans/2026-10-09-antigravity-shared-model-quota.md)

## 结果

官方 `/usage` 报告的 `Claude and GPT models` 共享组现在作为独立的可选窗口存入 Antigravity 快照，并在 macOS/Windows 详情中显示五小时及每周剩余比例与重置时间。Gemini 两个指标、主摘要、浮动条、提醒与原有青绿色强调色保持不变；缺少共享数据时隐藏区块。Claude Code 和 OpenAI Codex 使用各自现有数据源，未参与本次字段或界面绑定。

双平台接受旧快照中缺少共享字段的情况，并从旧的四窗口缓存中将共享池迁移到新字段。解析拒绝部分窗口、重复键、非法百分比和重置值。共享快照合同 schema 与 fixture 同步更新。

官方依据：[Google Antigravity Models](https://antigravity.google/docs/models?hl=en) 将 `Gemini Models` 与 `Claude and GPT models` 列为不同额度组，并分别提供 Five Hour 与 Weekly Remaining；[CLI Model Quotas](https://www.antigravity.google/docs/cli/commands/usage) 说明 `/usage` 展示模型额度并刷新额度状态。

## 安全与范围

不运行真实 `agy`，不读取或改变 Google/Claude/OpenAI 账号、Keychain、浏览器或已安装用户应用。未改动采集命令。Windows 的 Rust 与 React 测试及构建在 macOS 宿主执行，只能证明共享代码、序列化合同和前端行为，不能替代原生 Windows 构建、签名包或真机验证。没有推送、PR、合并或发布。

## 验证

- `AI_METER_TEST_BUILD_DIR=/private/tmp/ai-meter-safe-auto-refresh-final scripts/test.sh`：审查修正前通过。Swift 共 643 项：Widget 12、Core 288、App 322、刷新调度 3、PTY runner 18。公共合同、发行资产归一化、更新源探测、354 份 Markdown 文档及公开发布安全门禁均通过。
- 审查修正后，在隔离的 SwiftPM Core 测试包运行解析与缓存用例，13 项通过，包含非 Gemini 清除和拒绝未知窗口的新回归测试。
- 共享配额 React 定向测试 11 项通过，`npm run build` 通过；Windows Rust 定向解析、缓存和共享合同测试共 20 项通过。
- Windows Rust 定向解析、缓存和共享合同测试共 20 项通过，`cargo fmt --check` 通过；跨平台快照合同 6 个 fixture 通过。此工作区未执行 Windows 原生构建。
- `git diff --check`、`scripts/check-docs.sh`（354 份 Markdown）与 `ruby scripts/check-cross-platform-contracts.rb .` 均通过。
- macOS Release 构建的编译步骤到达 dSYM 生成，但系统以 `Operation not permitted` 拒绝 `dsymutil`，因此不能记为 Release 构建通过。首次直接执行还受沙箱拦截；重试使用项目测试相同的隔离 SwiftPM 路径后，错误收敛到 dSYM 生成阶段。
- 独立只读审查发现 3 项低风险合同/兼容性不一致，已全部修正：Swift 会清除非 Gemini 快照上的共享字段、共享窗口只接受两项指定标签、schema 将 `resetAt` 列入必填字段。审查修正后的 Core 回归和 schema 负向检查通过。
- 未运行真实CLI、账号/凭证流程、Windows 真机或签名安装器验证。

## Git

REQ-20261009-001 的本地安全修正已先提交在分支 `codex/antigravity-safe-auto-refresh`（提交 `b2db563`），没有发布。REQ-20261009-002 与本文档的功能实现已完成独立审查和本地验证，仍待本地提交；尚未推送、合并或发布。
