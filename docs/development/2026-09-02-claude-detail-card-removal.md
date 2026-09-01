# Claude 详情卡片精简开发验收日志

**日期：** 2026-09-02  
**需求：** `REQ-20260901-008`  
**结果：** 已完成、安装并通过真实桌面交互验收

## 背景与目标

Claude 详情页原本在本机 Sessions、Active days、Tokens 和每日趋势下方继续显示 Token composition 与 Top models。用户确认这两张卡片不需要展示，希望页面更短、更聚焦，同时保留额度、本机活动趋势和隐私说明。

本次采用展示层精简方案：不修改 Claude 本机活动采集器、领域模型、缓存字段或旧数据解码，因此已有历史缓存仍可读取，后续也不需要迁移或清空数据。

## 实现范围

- 从 `ClaudeDetailView` 移除 Token composition 与 Top models 的调用和视图函数；
- 删除只为 Top models 展示服务的 `ClaudeModelRowPresentation` 与百分比转换辅助；
- 保留以下内容：
  - Current session 与 Weekly 官方额度；
  - Sessions、Active days、Tokens；
  - 30 天每日 Token 柱图及零活动空状态；
  - 本机数据隐私提示、采集状态和更新时间；
- 保留 `ClaudeModelActivity`、Input/Output/Cache 聚合值、快照编码和缓存兼容路径。

## 测试先行证据

新增源码契约测试后先运行 Claude 详情测试，旧实现按预期产生 **4 个失败断言**：两张卡片的调用和可见标题仍存在。完成最小展示层改动后，定向测试结果为：

- **4 项 Claude 详情展示测试通过；**
- **0 个失败。**

最终完整测试结果：

- **294 项测试、58 个测试组通过；**
- **0 个失败；**
- 相比 295 项基线，新增 1 项卡片缺席回归测试，并删除 2 项已经失去产品意义的 Top models 展示辅助测试。

源码契约持续验证两张卡片的调用和可见标题不存在，同时 Sessions、每日趋势与隐私提示仍存在。

## Release、安装与真实桌面验收

使用 `AI_METER_INCLUDE_WIDGET=0` 构建正式候选包，结果为：

- `codesign --verify --deep --strict` 通过；
- Bundle Identifier：`com.millerpan.AIMeter`；
- 版本：0.1.0（build 1）；
- 主程序：arm64 Mach-O；
- 候选版与安装版主程序 SHA-256 均为 `ee12a70ca638e067bfeb50040e8ed65c44126ba2ef115aee35ae012dbe65f6ba`。

覆盖前的旧安装版已可恢复地备份到：

`/private/tmp/AI Token Meter-pre-card-cleanup-20260902-0738.app`

安装后通过真实辅助功能交互点击 Claude：状态由 `Detail closed` 变为 `Detail open`；等待当前设置的 8 秒后自动回到 `Detail closed`。浮岛仍保持右侧、垂直 50%，Codex 与 DeepSeek 状态未受影响。

Claude 详情使用非激活 `NSPanel`，当前辅助功能截图只捕获浮岛本体，不能把完整详情面板作为视觉截图证据。因此本次不把不完整截图冒充详情验收；可见内容由源码契约、SwiftUI 编译、正式安装包哈希一致性和实际详情开关状态共同覆盖。

## 安全与隐私检查

- 未增加网络请求、凭证读取或历史目录扫描；
- 未改变聊天正文、提示词、项目路径等隐私边界；
- 未删除旧缓存字段，避免因界面精简导致数据迁移风险；
- 用户要求移除的内容只从详情展示层消失。

## Git 节点

- `6fd2a8c`：确认 Claude 详情卡片精简设计；
- `196a2c6`：保存实施计划与需求台账进度；
- `98b96e6`：以测试驱动方式移除两张卡片及专用展示辅助。

## 已知边界

- 底层仍聚合 Input/Output/Cache 与模型 ID，这是旧缓存兼容边界，不代表详情页仍展示这些内容；
- Widget 签名与 Gallery 验收仍由 `REQ-20260901-003` 单独跟踪，不属于本次范围。
