# Provider 用户可见名称统一开发与验收记录

**日期：** 2026-09-02  
**需求：** `REQ-20260902-011`  
**分支：** `codex/provider-visible-names`  
**状态：** 进行中

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

待完成后补充：完整测试数量与套件、Release 构建、严格签名、Widget 包检查、候选/安装版 SHA-256。

## 真实界面与辅助功能验收

待安装候选后检查菜单面板、浮动条、两项详情、Settings Services 与 Widget 当前说明。重点确认 `OpenAI Codex` 在详情和 Widget 中不截断。

## 安全与隐私

- 没有读取、迁移或记录任何凭证；
- 通知 `userInfo` 继续保存原始 Provider raw value，不改为展示名称；
- 没有新增网络请求、权限或持久化字段；
- 历史规格、旧实施计划和既有开发日志保持原文，不重写历史证据。

## Git 证据

- `04b707b`：建立书面设计规格；
- `b2a2b8d`：记录用户确认；
- `7c358db`：完成测试驱动实施计划；
- `68b833f`：集中核心正式名称；
- `318b556`：统一主应用、通知、解析器和 Widget 当前名称。

最终文档、验证和主分支合并提交待验收后补充。

## 已知限制

Widget Gallery 与真实桌面 Widget 仍受现有 Apple Development 证书事项 `REQ-20260901-003` 限制；本次会验证 Extension 编译、包结构和可执行展示模型，不把证书限制误报为命名功能失败。
