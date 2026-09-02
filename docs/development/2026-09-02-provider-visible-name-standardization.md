# Provider 用户可见名称统一开发与验收记录

**日期：** 2026-09-02  
**需求：** `REQ-20260902-011`  
**分支：** `codex/provider-visible-names`  
**状态：** 已完成并合入 `main`

## 背景与目标

当前产品在不同位置混用 `Claude` / `Claude Code` 与 `Codex` / `OpenAI Codex`。本阶段把现行界面、辅助功能、通知、Widget 与维护文档统一为：

- `Claude Code`
- `OpenAI Codex`
- `DeepSeek`

内部身份不变：`claude` / `codex` 枚举 raw value、JSON、缓存、Widget 快照、`claude` / `codex` 命令、`.claude` / `.codex` 路径、Bundle Identifier、Keychain、App Group、资源与登录脚本文件名均保持兼容。

## 设计与实现

### 单一正式名称源

`UsageProvider.displayName` 和 `WidgetProvider.displayName` 位于 `AIMeterCore`。`ProviderPresentation`、主应用、通知和 Widget 全部复用这两个属性，删除主应用与 Widget Extension 原有的重复名称 switch。

### 当前界面

- Provider 卡片、浮动条朗读和详情标题使用核心正式名称；
- Settings > Services、账户操作反馈、Demo 标签和 About 隐私说明使用正式名称；
- Claude Code 详情将 CLI 来源明确为 `Claude Code CLI`；
- OpenAI Codex 详情的本机活动说明使用完整名称；
- Small Widget 辅助功能、Medium/Large 标题、下次重置卡片及 Widget Gallery 描述使用完整名称；
- 较长的 `OpenAI Codex` Widget 标题增加单行缩放保护。

## TDD 证据

### 红灯

1. `AppPresentationTests` 首次编译失败，明确报告 `UsageProvider` / `WidgetProvider` 尚无 `displayName`；
2. `NotificationServiceTests` 首次编译失败，明确报告通知正文入口尚不存在；
3. 详情设置文案、Claude Code 账户回退标签、两种 CLI 无标签额度与工作区提示分别由更新后的断言暴露旧简称；
4. 删除 Widget 重复 `name` 后，编译器在 Small Widget 组合表达式报告类型推断超时。回溯发现其仍引用已移除的 `snapshot.provider.name`；改为核心 `displayName` 后恢复，布局未改动。

### 绿灯

以下定向套件全部通过：

- `AppPresentationTests`：17 项；
- `NotificationServiceTests`：1 项；
- `ServiceAccountSettingsTests`：9 项；
- `SettingsStructureTests`：4 项；
- `ClaudeAccountReaderTests`：6 项；
- `ClaudeUsageParserTests`：5 项；
- `CodexUsageParserTests`：4 项；
- `WidgetLayoutPolicyTests`：4 项。

## 完整自动化、构建与安装

- 完整自动化：**304 个测试、60 个测试组、0 失败**；
- 合并后回归：`main` 再次执行完整套件，仍为 **304 个测试、60 个测试组、0 失败**；
- 主应用：`scripts/build-app.sh` Release 构建成功，生成 Apple Silicon `arm64` 产物；
- 签名：无 Apple Development 身份时按设计生成 ad-hoc 主应用，`codesign --verify --deep --strict` 通过；Candidate CDHash 为 `6e31338fd37323b6710f0ebe951f5c4506b49d24`；
- Widget：使用独立临时模块缓存执行 `swift build -c release --product AIMeterWidgetExtension`，Release target 编译与链接成功；
- Widget Bundle：当前没有 Apple Development 身份，主应用构建按设计显示 `Widget skipped`，因此 `verify-widget-bundle.sh` 对无 `.appex` 的主应用返回失败；真实 Gallery、桌面安装及嵌套签名仍归既有延期项 `REQ-20260901-003`，没有伪报通过；
- 安装：旧版应用可恢复备份到 `/private/tmp/AI Token Meter.pre-provider-names-20260902-1424.app`，候选版安装到 `/Applications/AI Token Meter.app` 并成功启动；
- 候选版与安装版主可执行文件 SHA-256 均为 `5ef2fcd948e70c9e56267c9acd4819065f9dbda5273197d8d900af85cd2cf3d3`；安装版严格签名再次通过。

## 真实界面与辅助功能验收

已安装应用的真实辅助功能树确认：

- 浮动条依次朗读 `Claude Code, 0%, Current session`、`OpenAI Codex, 37%, Weekly limit` 与 DeepSeek 余额；
- 点击 OpenAI Codex 后辅助功能值切换为 `Detail open`，证明交互链路仍正常；
- Settings > Services 显示 `Claude Code`、`Claude Code · Pro`、`OpenAI Codex`，说明文字分别指向官方 Claude Code CLI 与 OpenAI Codex CLI；
- 浮动条、Settings 和 CLI 刷新均继续使用原 Provider 身份，未发生 raw value 迁移。

当前自动化界面工具不会枚举非激活详情面板的内部文本，因此没有把详情标题的直接辅助功能读取冒充为通过；该标题由核心 `displayName` 同源生成，已由 `ProviderPresentationTests`、源码编译和已安装交互链路共同覆盖。Widget 当前无法安装到桌面，其正式名称由核心测试、Small 辅助功能源码和独立 Release target 编译覆盖。

## 安全与隐私

- 没有读取、迁移或记录任何凭证；
- 通知 `userInfo` 继续保存原始 Provider raw value，不改为展示名称；
- 没有新增网络请求、权限或持久化字段；
- 历史规格、旧实施计划和既有开发日志保持原文，不重写历史证据。

## Git 证据

- `e09aa8c`：建立书面设计规格；
- `3da0952`：记录用户确认；
- `7156b74`：完成测试驱动实施计划；
- `3852ee1`：集中核心正式名称；
- `d450bd1`：统一主应用、通知、解析器和 Widget 当前名称；
- `5eacfca`：同步 README、用户指南、架构、隐私与开发文档；
- `5cb3ab8`：记录完整测试、Release、安装、真实辅助功能与证书限制；`main` 快进合并到该提交。

合并后完整回归通过，需求台账已关闭；后续维护只需继续使用核心 `displayName`，不要重新引入页面级简称 switch。

## 已知限制

Widget Gallery 与真实桌面 Widget 仍受现有 Apple Development 证书事项 `REQ-20260901-003` 限制；本次会验证 Extension 编译、包结构和可执行展示模型，不把证书限制误报为命名功能失败。
