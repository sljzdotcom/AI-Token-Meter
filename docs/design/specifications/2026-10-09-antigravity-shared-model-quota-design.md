# Antigravity Claude/GPT 共享额度详情规格

**需求：** REQ-20261009-002  
**状态：** 已获用户批准，开发中  
**范围：** macOS 与 Windows 的 Google Antigravity 详情

## 目标

在 Antigravity 详情中展示官方 `/usage` 报告的 `Claude and GPT models` 五小时及每周额度，同时保留现有 Gemini 额度、主指标、告警、浮动条和配色行为。

## 官方数据边界

Google Antigravity 的官方模型页将 `Gemini Models` 与 `Claude and GPT models` 分组，并分别提供 Weekly 与 Five Hour Limit Remaining。第二组是 Antigravity 内 Claude/GPT 模型共享池，不是 Claude Code 或 OpenAI Codex 的独立账号额度。应用只显示 CLI 报告中实际提供的共享组百分比和重置时间，不推断套餐、模型级明细或 Credits。

## 行为

- 两平台解析器都保留可选的两条共享池窗口，字段与 Gemini 窗口分离。
- 两条共享窗口必须同时存在且通过现有百分比、日期、单位、来源验证；部分或重复窗口拒绝该输出。只有 Gemini 的旧输出仍有效。
- Antigravity 详情在 Gemini 额度之后显示“Claude/GPT shared quota”区块，两张卡片沿用现有指标卡与 Antigravity 强调色，分别显示五小时和每周剩余比例及重置时间。
- 缺少共享池时不显示空区块或伪造数字。共享池不进入 `primaryMetric`、`secondaryMetric`、`usedRatio`、提醒、菜单栏或 Widget。
- 缓存与跨平台快照传输保留字段；旧快照没有该可选字段时继续解码，仍可显示已有 Gemini 数据。
- 该字段仅属于 `.gemini`/Antigravity 快照；其他 Provider 携带此字段视为无效数据。
- UI 使用现有英文、简体中文、繁体中文及 Windows 本地化体系，不新增颜色或第五个浮动环。

## 验收

1. Gemini-only、Gemini+共享池、共享池缺失、部分/重复/非法共享池及旧缓存在 Swift 与 Rust 合同中得到一致处理。
2. macOS 与 Windows 详情分别显示真实的五小时、每周剩余量与重置时间；不出现 Claude Code/Codex 账号资料。
3. Gemini 主百分比、四个 Provider 浮动条和阈值提醒不受共享额度变化影响。
4. 相关平台测试、生产构建、共享合同和文档检查通过；不运行真实 `agy`、账号、Keychain、浏览器或发布流程。

## 参考

- [Google Antigravity Models](https://antigravity.google/docs/models?hl=en)
- [Google Antigravity CLI Model Quotas](https://www.antigravity.google/docs/cli/commands/usage)
