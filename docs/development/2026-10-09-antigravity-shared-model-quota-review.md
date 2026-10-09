# Antigravity 共享额度独立复审

**需求：** REQ-20261009-002  
**审查基线：** `0e7540f` + `00b769b`，本地分支 `codex/antigravity-safe-auto-refresh`  
**范围：** 共享池规格、Swift/Rust 快照标准化、schema/fixture、详情 UI、本轮三项修正闭环。  
**审查方式：** 独立只读复核；未编辑文件、未运行测试、未运行真实 CLI、账号或浏览器。

## 首轮发现与修正

1. **共享字段的 Provider 边界不一致。** Swift 对非 Gemini 快照直接返回，可能保留不属于该 Provider 的 Antigravity 共享池；Rust 会拒绝。`UsageSnapshot.normalizedAntigravityQuota()` 现在会清除非 Gemini 的共享池，同时保留原快照的普通指标、状态、时间戳、本地活动与历史。`normalizationDropsSharedQuotaFromOtherProviders` 覆盖该行为。
2. **Swift 标准化接受未知额外窗口。** Swift 先前只挑出两个已知标签，可能忽略第三个未知项；Rust 会拒绝。现在标准化要求数量恰为两项且标签集合精确匹配，否则整组隐藏。`normalizationRejectsExtraUnknownSharedQuotaWindow` 覆盖该行为。
3. **Schema 未要求重置时间。** 共享指标此前可在独立 schema 消费者中缺少 `resetAt`。该字段现列于必填项；`test-cross-platform-contracts.sh` 移除该必填项并断言合同门禁失败。

## 闭环复核

最终独立复核确认以上三项修正均通过，没有残留问题。详情页由父级明确显示 **Google Antigravity**，共享窗口继续使用五小时/每周官方额度，展示“剩余比例”及重置时间；不读取 Claude Code 或 OpenAI Codex 的独立账号数据。

旧缓存不包含新共享字段时仍能解码且不显示空区块；旧四窗口缓存迁移保留 Gemini 主摘要并把两项共享额度独立保存。对应覆盖包括：

- Swift：`normalizationDropsSharedQuotaFromOtherProviders`、`normalizationRejectsExtraUnknownSharedQuotaWindow`、`oldSnapshotsWithoutSharedQuotaFieldStillDecode`、`legacyFourWindowCacheMigratesSharedPoolAndRecomputesGeminiSummary`。
- Windows：`renders Antigravity's shared Claude/GPT quota independently from Gemini`、`hides the shared quota section when the CLI reports no shared model pool`；同一 UI 测试确认四个窗口均有重置时间展示。
- 合同：移除共享指标的 `resetAt` 后，跨平台合同脚本必须失败。

独立复核仅审查代码和测试，不代替开发侧完整测试结果；最终完整回归见[开发记录](2026-10-09-antigravity-shared-model-quota.md)。
