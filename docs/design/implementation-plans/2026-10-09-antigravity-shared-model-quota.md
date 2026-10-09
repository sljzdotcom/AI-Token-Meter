# Antigravity Claude/GPT 共享额度详情实现计划

> **面向 AI 代理的工作者：** 按下列步骤逐项实施；先添加会失败的测试，再修改实现。只操作当前隔离工作区，不运行真实 CLI 或账号流程。

**目标：** 在 macOS 与 Windows Antigravity 详情中显示官方 Claude/GPT 共享额度窗口。  
**架构：** Swift 与 Rust 解析器将 Gemini 两个指标和可选共享池两个指标分别放进兼容快照字段；两端详情重用现有 MetricCard/进度条和强调色。共享池字段只出现在详情，不影响主指标或其他 UI。  
**技术栈：** Swift 6、Swift Testing、Rust、serde、React、Vitest、共享 JSON 合同。

---

## 文件与职责

- 修改 `Sources/AIMeterCore/Domain/UsageModels.swift`：添加共享池快照字段，规范化并隔离 Provider。
- 修改 `Sources/AIMeterCore/Collectors/GeminiUsageParser.swift`：解析并独立返回共享池指标。
- 修改 `Tests/AIMeterCoreTests/GeminiUsageParserTests.swift`：覆盖解析和兼容性合同。
- 修改 `Sources/AIMeterApp/Views/GeminiDetailView.swift` 与本地化 strings：macOS 详情区块及三种语言文案。
- 修改 `Tests/AIMeterAppTests/GeminiDetailPanelLayoutTests.swift`、`AppLocalizationTests.swift`：界面与本地化验证。
- 修改 `windows/src-tauri/src/domain/usage.rs`、`collectors/gemini.rs`：Rust 快照/校验/解析。
- 修改 `windows/src-tauri/tests/gemini_collector.rs` 与合同 fixture：Windows 原生数据合同。
- 修改 `windows/src/state/usage.ts`、`windows/src/details/ProviderDetail.tsx`、`GeminiDetail.test.tsx`、`localization.ts`：Windows 类型、详情及文案。
- 如共享合同定义集中在 `contracts/fixtures`，更新其 Antigravity 快照与对应合同测试；既有缓存无新字段仍须兼容。
- 新建 `docs/development/2026-10-09-antigravity-shared-model-quota.md` 并更新文档索引。

## 步骤

1. 在 Swift parser tests 中先断言共享池有两条精确指标，Gemini-only 仍无共享值，部分/重复/越界共享值拒绝。
2. 在 Rust collector tests 中写同样的合同测试并验证旧的 Gemini-only fixture 保持有效。
3. 先运行两组定向测试，确认失败来自缺少共享字段/解析逻辑。
4. 添加 Swift 可选快照字段，扩展标准化及 parser；确保旧 Codable 缓存中缺键可解码，非 Gemini 快照不保留共享指标。
5. 添加 Rust serde 默认字段、验证和 parser 输出；只在 `.gemini` 快照接受最多两条精确标签。
6. 运行 Swift parser 测试及 `cargo test` 的定向 Gemini collector/usage 合同测试并修复到通过。
7. 添加 macOS detail 测试，断言共享区块标题、五小时/每周百分比和重置文本；断言缺失时隐藏。
8. 添加 Windows `ProviderDetail` 测试并补英文、简体、繁体三种文案；保持 Antigravity 既有强调色。
9. 运行 Swift detail/localization 与 Windows Vitest 定向测试，确认失败后实现最少 UI 变更并通过。
10. 更新跨平台 fixture/schema 合同，使 Swift 解析器、Windows Rust 与前端共用一个代表性合成测试样例，且 Gemini 主指标未受共享池影响。
11. 运行相关完整平台测试、`scripts/test.sh`、macOS release build、Windows web/Rust合同测试及 `scripts/check-docs.sh`；记录跳过项与非原生验证范围。
12. 请求独立只读代码审查，处理发现后提交本地变更；不推送、合并或发布。

## 最终状态

本地实现及独立复审已完成。最终 Swift 645 项、Windows 前端 135 项、Windows Rust 宿主 268 项通过；macOS Release product build 通过。跳过项和平台限制见[开发记录](../../development/2026-10-09-antigravity-shared-model-quota.md)，审查闭环见[独立复审记录](../../development/2026-10-09-antigravity-shared-model-quota-review.md)。未触发 CI，未签名、合并或发布。
