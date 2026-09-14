# Google Antigravity Gemini 优先详情设计

**需求：** REQ-20260914-001  
**状态：** 用户已确认推荐 A，进入实施  
**基线：** `5c1adb2`（v0.10.0 发布证据已合入的 `origin/main`）  
**确认：** 用户于 2026-09-14 回复“确认，按照你的推荐进行”  
**发布边界：** 本需求允许规格、计划、实现、测试、审查与本地整合，不授权公开推送或发布新版本

## 问题与目标

Google Antigravity 的 `/usage` 当前可返回 Gemini 与 Claude/GPT 两组五小时、每周额度。现有双平台实现把四个窗口全部显示，并把最严重的任一窗口用于 Antigravity 的顶部百分比、圆环、菜单/托盘汇总与提醒。用户认为 Claude Code 与 Codex 信息在此详情中没有价值，希望页面聚焦 Gemini，并使用登录后真实可读的 Antigravity CLI 信息充实详情。

本次目标是把 Antigravity 的产品口径完整收束到 Gemini：界面只显示 Gemini 五小时与每周窗口，所有汇总与提醒也只由这两项计算。独立的 Claude Code、OpenAI Codex Provider 不变。详情新增当前 Gemini 模型、动态可用 Gemini 模型数量与系列、CLI 版本和检查时间。

## 已验证来源与能力边界

- [官方 `/usage`](https://antigravity.google/docs/cli/commands/usage) 返回模型额度与重置窗口；它由 CLI 自身处理并刷新后端状态。
- [Headless 文档](https://antigravity.google/docs/cli/headless/)要求把 `/usage`、`/model` 作为独立的 `agy -p` 命令；`agy models` 用于列出可用模型。
- [CLI 参考](https://antigravity.google/docs/cli/reference/)没有稳定的账户、套餐或 30 日聚合命令。
- 本机已登录 `agy 1.2.2` 只读核验：`/model` 返回一个制表符分隔的模型 ID 与显示名；`models` 返回动态列表，当前含 11 个 Gemini 变体、4 个 Gemini 系列以及 3 个第三方模型。数量与型号仅是现场证据，产品不得硬编码。
- `/credits` 的余额属于独立付费余额，用户已确认首版排除。不得从模型清单推测套餐，也不得读取会话正文制造 30 日统计。

这些命令不发送自然语言提示，不发起模型生成。采集仍使用私有临时目录、受控环境、有界时间和输出大小；不读取或复制 OAuth/Keychain/Credential Manager 内容。

## 额度合同

`/usage` 解析器继续验证 CLI 的完整结构，但只输出两条 Gemini 指标：

1. `Gemini · Five hour`
2. `Gemini · Weekly`

两条 Gemini 行必须各出现一次。CLI 可以只返回这两行，也可以额外返回完整的 Claude/GPT 五小时与每周两行；额外行必须成对、唯一且字段合法，随后被丢弃。未知分组、未知窗口、重复行、只出现一条额外行、非法百分比或非法重置时间全部失败关闭。

Remaining 继续转换为已用百分比。两条 Gemini 指标按固定显示顺序保存；`primaryMetric` 与 `secondaryMetric` 按已用比例排序，平局保持五小时在前。Windows `usedRatio` 取两条 Gemini 指标的最大已用比例。这样顶部悬浮条、详情圆环、菜单/托盘汇总和 70%/90% 提醒自然只使用 Gemini 数据，不需要在各显示层再做第二套过滤。

旧缓存加载时立即归一化：只保留标签严格匹配的两条 Gemini 指标，重新计算主次指标与 `usedRatio`。没有完整两条 Gemini 指标的旧 Antigravity 缓存不得继续作为可见额度；避免隐藏的 Claude/GPT 指标在升级后离线期间驱动界面或提醒。

## CLI 补充信息合同

新增可选 `AntigravityCLIInfo`：

- `currentModel`：仅当 `/model` 返回唯一、合法的 Gemini 模型时保存其显示名；当前选中 Claude/GPT 或输出歧义时省略。
- `availableModelCount`：`models` 返回的合法 Gemini 模型行数量。
- `modelFamilies`：从 Gemini 显示名移除末尾 `(High)`、`(Medium)`、`(Low)` 等档位后按首次出现顺序去重，例如 `Gemini 3.8 Flash`。限制合理数量与字段长度，避免异常输出进入缓存或界面。
- 检查时间复用快照的 `fetchedAt`；CLI 版本复用 `sourceVersion`。

`/usage` 是唯一决定额度刷新成功与否的命令。`/model` 和 `models` 在额度成功后以各自的有界命令读取；任一超时、退出失败、认证失败、格式变化或没有 Gemini 行，只省略对应字段，不能把成功额度降级。任务取消仍立即传播。失败刷新保留上次成功快照及补充信息，并沿用缓存时间和 stale 状态。

## 双平台界面

### macOS

详情保留青绿渐变标题、进度强调色和不透明深海底板。内容顺序：

1. Google Antigravity 标题；
2. “Gemini quota”区，两条额度行；
3. “Antigravity CLI”紧凑信息卡：当前模型、可用模型摘要、CLI 版本；
4. 数据更新时间、诊断、安装指南和 Retry。

信息卡使用现有主题颜色、紧凑正文与 caption，不新增高饱和颜色。面板高度从四额度卡布局收紧为两额度行加一张信息卡；小屏继续滚动。

### Windows

沿用现有 `provider-detail--gemini` 青绿 `#3ED6B2`、不透明深色渐变和 compact density。两张额度卡保持双列；下方增加一张 `antigravity-cli-info` 紧凑卡，显示与 macOS 同语义的三项信息。窄窗口按既有媒体查询单列，不改变其他 Provider。

英文、简体中文及 macOS 繁体中文补齐“Gemini quota”“Antigravity CLI”“Current model”“Available models”“CLI version”“N models · N families”等文本。模型产品名不翻译。

## 缓存、合同与隐私

共享 JSON 合同把 `geminiQuotaMetrics` 上限改为 2，允许的标签仅为两条 Gemini 窗口；新增可选 `antigravityCLIInfo`。Swift/Rust/TypeScript 使用同名 camelCase 字段，旧快照缺失新字段仍可解码。新写缓存永远不包含 Claude/GPT Antigravity 指标。

补充信息只含公开模型显示名、计数、系列和 CLI 版本，不含账号、套餐、余额、提示词、响应、路径或凭据。隐私清洗仍作用于字符串，异常长或不符合 Gemini 格式的模型字段不保存。

## 验收口径

1. 四行和两行 `/usage` 合成输出都只产出两条 Gemini 指标；缺失 Gemini、半组额外行、重复、未知或非法字段拒绝。
2. 原来最严重的 Claude/GPT 指标不再改变 Antigravity 主指标、Windows `usedRatio`、顶部圆环、全局汇总或提醒。
3. 旧四行缓存加载后只剩两条 Gemini 指标并重算；只有 Claude/GPT 或不完整 Gemini 的缓存不作为可见额度。
4. `/model` 仅接受当前 Gemini；`models` 动态过滤第三方模型、去重系列并处理 banner、顺序与格式变化。
5. 任一补充命令失败时额度仍为 fresh；取消、输出上限和临时目录清理继续受控。
6. macOS 与 Windows 详情仅显示两条 Gemini 额度，显示可用的 CLI 信息，不出现 Claude/GPT 卡片或 AI Credits；缺补充字段时布局自然收缩。
7. 双平台定向及完整测试、生产构建、合同、文档、安全检查和独立审查通过。真实 Windows DPI/读屏与不同账号模型清单保留为现场验收边界。

