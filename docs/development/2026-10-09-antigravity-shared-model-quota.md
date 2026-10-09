# Antigravity Claude/GPT 共享额度详情

**需求：** REQ-20261009-002  
**现场基线：** 0.10.7/build35、Antigravity CLI 1.3.1；配额截图为 2026-10-08 23:26（Asia/Singapore）。本地变更未发布。  
**开发状态：** 本地实现、独立审查和最终可执行验证完成；等待协调确认后续整合步骤。
**工作区：** `/private/tmp/AI-Meter-safe-auto-refresh`  
**设计：** [规格](../design/specifications/2026-10-09-antigravity-shared-model-quota-design.md) · [计划](../design/implementation-plans/2026-10-09-antigravity-shared-model-quota.md)

## 结果

官方 `/usage` 报告的 `Claude and GPT models` 共享组现在作为独立的可选窗口存入 Antigravity 快照，并在 macOS/Windows 详情中显示五小时及每周剩余比例与重置时间。Gemini 两个指标、主摘要、浮动条、提醒与原有青绿色强调色保持不变；缺少共享数据时隐藏区块。Claude Code 和 OpenAI Codex 使用各自现有数据源，未参与本次字段或界面绑定。

双平台接受旧快照中缺少共享字段的情况，并从旧的四窗口缓存中将共享池迁移到新字段。解析拒绝部分窗口、重复键、非法百分比和重置值。共享快照合同 schema 与 fixture 同步更新。

官方依据：[Google Antigravity Models](https://antigravity.google/docs/models?hl=en) 将 `Gemini Models` 与 `Claude and GPT models` 列为不同额度组，并分别提供 Five Hour 与 Weekly Remaining；[CLI Model Quotas](https://www.antigravity.google/docs/cli/commands/usage) 说明 `/usage` 展示模型额度并刷新额度状态。

## 安全与范围

不运行真实 `agy`，不读取或改变 Google/Claude/OpenAI 账号、Keychain、浏览器或已安装用户应用。未改动采集命令。合同 fixture 中的额度和重置时间是合成测试数据，仅验证字段与 UI，不代表任何真实账户的当前额度。Windows 的 Rust 与 React 测试及构建在 macOS 宿主执行，只能证明共享代码、序列化合同和前端行为，不能替代原生 Windows 构建、签名包或真机验证。没有推送、PR、合并或发布。

## 验证

- 最终提交基线 `0e7540f`/`00b769b`：`AI_METER_TEST_BUILD_DIR=/private/tmp/ai-meter-safe-auto-refresh-final scripts/test.sh` 通过，Swift 共 645 项：Widget 12、Core 290、App 322、刷新调度 3、PTY runner 18。最终没有失败项。21 个测试与 1 个 UI 宿主套件按运行环境跳过：4 个已安装 CLI/Keychain 集成，10 个窗口宿主本地化/账号视图，7 个浮动条时序与拖动几何测试。完整输出保存在 `/private/tmp/ai-meter-safe-auto-refresh-final-full.log`。首次沙箱运行在测试开始前因禁止下载固定 Sparkle 依赖而退出；使用标准权限重新运行后全套通过。
- Windows 前端完整 `npm test -- --run`：18 个测试文件、135 项通过；`npm run build` 的 TypeScript 检查与 Vite 生产前端构建通过。
- Windows Rust 宿主 `cargo test --manifest-path src-tauri/Cargo.toml`：268 项通过，0 失败、0 ignored；`cargo fmt --manifest-path src-tauri/Cargo.toml --check` 通过。三个 Windows 专用 harness 在 macOS 主机没有可运行测试（0 项）。Rust 完整输出保存在 `/private/tmp/ai-meter-safe-auto-refresh-rust-final.log`。一次沙箱内重跑有 6 项 loopback mock 服务因 `Operation not permitted` 失败；使用普通权限重跑后 268 项全部通过。
- `ruby scripts/check-cross-platform-contracts.rb .` 与 `bash scripts/test-cross-platform-contracts.sh` 通过（6 fixtures，含移除 `resetAt` 必填项后门禁失败的负向验证）；共享快照合同、发行资产归一化、更新源探测、355 份 Markdown 文档及公开发布安全门禁均通过。
- `git diff --check`、`scripts/check-docs.sh`（最终 355 份 Markdown）与跨平台合同检查均通过。
- macOS `swift build --product AIMeterApp -c release` 首次在沙箱内的 dSYM 生成阶段遇 `Operation not permitted`。授予该本地构建命令标准权限，并将 SwiftPM 缓存、dSYM 与产物限定在 `/private/tmp/ai-meter-safe-auto-refresh-final` 后，生产构建成功（20.10 秒）。因此准确原因是沙箱阻止 `dsymutil`；正常权限本地构建可行。这不包含签名、安装包或发布流程。
- 独立只读闭环审查见[复审记录](2026-10-09-antigravity-shared-model-quota-review.md)：前轮指出的 3 项合同/兼容性问题已修复，最终复核均通过。确认 Swift 非 Gemini 快照清除共享字段、严格拒绝额外未知窗口，Schema 与负向合同要求 `resetAt`；确认两端明确处于 Google Antigravity 详情上下文，并显示共享额度剩余比例与重置时间；旧快照缺字段保持可解码/隐藏，四窗口旧缓存迁移正常。审查者未运行测试或真实 CLI。
- 没有运行 Windows 原生构建、Windows 真机测试、签名安装包或应用发布；Windows 专用 harness 在 macOS 上为 0 项。没有触发 CI。
- 未运行真实CLI、账号/凭证流程、Windows 真机或签名安装器验证。

## Git

REQ-20261009-001 的本地安全修正已先提交在分支 `codex/antigravity-safe-auto-refresh`（提交 `b2db563`），没有发布。REQ-20261009-002 已在同一分支本地提交 `0e7540f`，最终验证记录为 `00b769b`，独立复审与本地验证完成；尚未推送、合并或发布。
