# Provider 用户可见名称统一设计规格

**日期：** 2026-09-02
**状态：** 待书面规格确认
**需求：** `REQ-20260902-011`

## 1. 背景与目标

AI Token Meter 当前在主应用、Widget、通知、辅助功能和用户文档中交替使用 `Claude` / `Claude Code` 与 `Codex` / `OpenAI Codex`。用户要求将当前产品里的服务名称统一为：

- `Claude Code`
- `OpenAI Codex`
- `DeepSeek` 保持不变

本次目标是统一用户看到和听到的名称，同时保持本机数据、认证、命令和升级兼容性不变。

## 2. 方案比较

### 方案 A：核心正式名称 + 各端复用（采用）

在 `AIMeterCore` 为 `UsageProvider` 和 `WidgetProvider` 提供正式显示名称。主应用卡片、设置、详情、通知、辅助功能和 Widget 统一引用这些属性；完整句子中的服务名称使用同一正式名称组合。

优点是名称只有一个事实来源，后续不易再次分叉；对数据格式和命令没有影响。代价是需要有意识地替换少量当前写死文案。

### 方案 B：逐页面改写

直接修改每个 `Text`、`Section` 和通知字符串。改动看似简单，但会继续保留多份名称定义，未来容易遗漏或回退。

### 方案 C：全仓机械替换

把所有 `Claude` 和 `Codex` 文本直接替换。虽然覆盖最广，但会污染历史日志、CLI 正式命令、数据路径、测试夹具和兼容标识，风险不可接受。

## 3. 当前用户可见范围

以下区域统一使用新名称：

1. 菜单栏弹出面板中的 Provider 卡片；
2. 浮动条和三个详情页的辅助功能名称；
3. Claude Code 与 OpenAI Codex 详情页标题、当前说明和空状态文案；
4. Settings > Services 的分组标题、状态与操作反馈；
5. macOS 通知正文；
6. Small、Medium、Large Widget 的名称及 Widget 说明；
7. Demo 模式中可见的账户标签；
8. README、当前用户指南、架构/隐私/测试/发布等现行维护文档；
9. 新增或更新的测试名称与断言中代表当前产品文案的部分。

## 4. 保持不变的兼容边界

以下内容不是用户品牌文案，必须保持原值：

- `UsageProvider.claude`、`UsageProvider.codex`、`WidgetProvider` 的原始枚举值；
- JSON、缓存和 Widget 快照中的 `claude` / `codex` 数据标识；
- `claude`、`codex` CLI 可执行文件名与实际命令；
- `.claude`、`.codex`、Claude Code 历史目录和 Codex SQLite 路径；
- Bundle Identifier、Keychain service、App Group、通知 `userInfo` 与已有持久化键；
- Logo 资源文件名、脚本文件名及登录脚本内部命令；
- 已归档的设计规格、实施计划、开发日志、提交历史和历史截图描述；
- 第三方或官方协议中必须原样保留的字段和值。

## 5. 架构与数据流

### 5.1 核心名称源

`UsageProvider` 增加公开只读 `displayName`：

- `.claude` → `Claude Code`
- `.codex` → `OpenAI Codex`
- `.deepSeek` → `DeepSeek`

`ProviderPresentation.title` 直接使用该属性，不再维护自己的名称 switch。

`WidgetProvider` 同样增加公开只读 `displayName`，确保 Widget Extension 不复制另一套字符串。Widget 的内部枚举和值保持不变。

### 5.2 主应用与通知

主应用已有的 `UsageProvider.displayName` 扩展移除，避免与核心定义冲突。详情、Settings、启动/登录反馈、通知正文和无数据提示改用核心名称或由核心名称组合的完整句子。

Provider Logo、颜色、排序和图标资源不变。

### 5.3 文档

只更新描述当前产品行为的文档。历史开发记录保留当时原文，避免重写历史证据。文档中的真实命令和路径继续使用原值，例如 `codex app-server`、`claude auth status --json` 和 `~/.codex/state_5.sqlite`。

## 6. 测试策略

采用测试驱动实现：

1. 先为 `UsageProvider.displayName` 与 `WidgetProvider.displayName` 添加失败断言；
2. 更新 `ProviderPresentation` 当前标题断言，证明主展示链路使用正式名称；
3. 更新 Widget 布局、账户状态、通知相关及界面展示测试中真实描述当前文案的断言；
4. 运行定向测试，确认测试先红后绿；
5. 运行完整测试，确认数据解析、缓存兼容、CLI、浮动条、详情和 Widget 均无回归；
6. 构建 Release、严格签名、安装并通过辅助功能树确认最终名称；
7. 核对候选与安装版可执行文件 SHA-256 一致。

测试不依赖全仓源码字符串搜索来证明界面正确；正式名称由可执行展示模型测试和真实安装验收保护。

## 7. 错误处理与安全

- 改名不改变错误分类、超时、缓存回退或认证流程；
- 不读取或迁移凭证，不修改 Keychain 项；
- 不新增网络请求、持久化字段或权限；
- 不把内部枚举或命令改成带空格的品牌字符串；
- 无障碍标签与可见标题使用相同正式名称，避免视觉和朗读不一致。

## 8. 验收口径

1. 当前产品界面中 Provider 标题不再单独显示 `Claude` 或 `Codex`；
2. 用户看到和 VoiceOver 听到的名称分别为 `Claude Code` 与 `OpenAI Codex`；
3. Widget、通知和 Settings 与详情页保持一致；
4. CLI 命令、持久化数据和历史记录不被重命名；
5. 完整自动化、Release 签名、安装哈希和真实界面验收全部通过；
6. README、Changelog、用户指南、需求台账和开发日志保持同步。
