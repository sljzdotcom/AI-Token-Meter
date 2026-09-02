# AI Meter 全局显示字体选择设计规格

**日期：** 2026-08-31

**状态：** 已实施；菜单与详情字体视觉验收待手工完成

**基础分支：** `codex/deep-sea-background`

## 1. 背景与目标

AI Meter 当前主要使用 SwiftUI 的 macOS 系统字体 San Francisco；部分标题和关键数值显式使用 San Francisco Rounded。用户希望在不改变布局、字号、粗细和信息层级的前提下，可以把整个应用的显示字体即时切换为 Antonio 或 DIN Condensed，并能一键恢复系统默认字体。

本功能建立一套集中式、可持久化的语义字体系统，作用于 AI Meter 自己绘制的文字。新安装与旧版本升级仍以系统字体作为初始默认值；当前用户在新版安装验收后选择 Antonio。

## 2. 已确认的字体选项

设置页固定展示三个选项：

1. `System Default`：使用现有 macOS San Francisco，并保留原有 Rounded 设计请求；
2. `Antonio`：使用本机已安装的 Antonio 字体家族；
3. `DIN Condensed`：使用本机已安装的 DIN Condensed 字体家族。

字体文件不随 App Bundle 或 Git 仓库分发。Antonio 与 DIN 由操作系统字体注册表提供，避免引入未知来源或许可不明确的字体文件。

## 3. 设置界面与即时行为

在 Settings 的 `Appearance` 分区增加：

- `Display font` 选择器，展示上述三个字体名称；
- 每个选项以自身字体预览名称，系统默认项使用系统字体；
- 字体不可用时，选项继续显示但不可选择，并明确标注 `Not installed`；
- `Restore Default Font` 按钮，点击后将选择恢复为 `System Default`；
- 已经处于系统默认时，恢复按钮禁用。

选择变化必须立即刷新当前正在显示的浮动条、Provider 详情页、菜单栏面板和设置窗口，不要求退出 App 或重新打开窗口。

## 4. 字体作用范围

字体选择作用于 AI Meter 自己绘制的全部可见文字：

- 菜单栏弹出面板中的标题、状态、时间和按钮文字；
- Claude、Codex、DeepSeek 详情中的标题、标签、百分比、余额、日期、说明和图表文字；
- Settings 中的分区、字段、说明和按钮文字；
- 浮动条中未来或异常状态下出现的文字；
- App 内辅助提示和错误说明。

以下内容不改变：

- Claude、Codex、DeepSeek Logo；
- SF Symbols 图标；
- 图形、圆环、背景纹理和颜色；
- macOS 系统警告框或其他系统进程绘制的文字；
- WebKit 内 DeepSeek 官方网页自身的字体。

## 5. 语义字体架构

新增独立的字体选择领域类型和偏好存储，不把字体名称散落在各个视图中。

### 5.1 领域与持久化

`DisplayFontChoice` 使用稳定的持久化值：

- `system`
- `antonio`
- `din-condensed`

`DisplayFontPreferenceStore` 只负责读取和保存选择：

- 未保存值时返回 `.system`；
- 遇到未知、损坏或旧版本不支持的值时回退 `.system`；
- 切换时立即写入 `UserDefaults`；
- 不因某次启动时字体暂不可用而覆盖用户保存的选择。

### 5.2 可用性解析

字体可用性由 AppKit 字体注册表检查：

- System 始终可用；
- Antonio 通过家族名 `Antonio` 检查；
- DIN Condensed 通过家族名 `DIN Condensed` 检查；
- 当前保存字体不可用时，界面实际渲染安全回退到系统字体，并在 Settings 标记该字体未安装；
- 用户安装字体后，下一次打开 Settings 或重启 App 即可重新选择，无需修改偏好文件。

### 5.3 语义角色

建立统一字体解析层，至少支持现有界面使用的角色：

- large title / title / title2 / title3；
- headline / subheadline / body；
- caption / caption2；
- 明确点数大小；
- regular、semibold、bold 等字重。

系统选项继续使用 SwiftUI 原生 semantic style，并保留调用方现有 Rounded 设计意图。Antonio 与 DIN Condensed 使用 `Font.custom(_:size:relativeTo:)` 或等价的动态语义映射，保持当前相对字号、字重和辅助功能缩放能力。

每个视图通过统一 modifier 或环境值请求语义字体，不直接读取 `UserDefaults`，也不自行判断字体名称。

## 6. 状态传播

`AppModel` 在初始化时从 `DisplayFontPreferenceStore` 载入选择，并以可观察属性暴露：

- `displayFontChoice`：用户保存的选择；
- 设置方法负责持久化并触发 SwiftUI 更新；
- 恢复默认调用同一设置路径写入 `.system`；
- 根场景把选择作为字体环境注入 MenuBar、Settings 和浮动面板；
- 独立 AppKit 窗口中的 SwiftUI Hosting View 使用同一 AppModel，因此当前打开详情也会即时重绘。

不得通过重建 AppModel、关闭窗口或重启进程实现字体切换。

## 7. 默认值与迁移

- 新安装：`.system`；
- 已有用户首次升级：由于没有字体偏好键，同样解析为 `.system`；
- 当前开发与验收机器：安装最终候选版后，通过正常设置路径选择 `.antonio`；
- `Restore Default Font` 写入 `.system`，而不是删除其他无关设置；
- 字体选择不改变贴边方向、垂直位置、自动隐藏时间或任何服务配置。

## 8. 错误处理与安全回退

- 字体家族不可用：运行时回退系统字体，设置选项禁用并显示 `Not installed`；
- 持久化值损坏：回退系统字体；
- 特定字形在 Antonio 或 DIN 中不存在：由 SwiftUI/macOS 字体级联补齐，不显示空白方框；
- 字体切换不得影响布局约束、详情窗口定位或文字可访问性；
- 不下载字体、不访问网络、不修改用户字体目录。

## 9. 测试与验收

自动化检查至少覆盖：

- 默认、保存、恢复和损坏值回退；
- 三个选项稳定的持久化标识与显示名称；
- System 永远可用，指定家族的 AppKit 可用性解析正确；
- 不可用字体的渲染回退和设置禁用状态；
- AppModel 切换后立即更新并持久化；
- 语义角色在三种选择下保持预期字号和字重映射；
- 现有视觉、拖动、详情、设置和 Provider 配色测试继续通过。

真实 UI 验收至少覆盖：

1. Settings 显示三个字体选项和恢复按钮；
2. System、Antonio、DIN Condensed 依次切换后，浮动条、菜单面板、三个详情页和 Settings 立即变化；
3. Logo、SF Symbols、进度环和深海背景不变化；
4. Antonio 下长日期、百分比、余额和错误说明无截断；
5. DIN Condensed 下小号说明仍可读；
6. 恢复默认后重新显示 San Francisco；
7. 重启 App 后选择保持；
8. 最终在当前机器选择 Antonio；
9. Release 构建、签名、Info.plist、安装包指纹和完整测试通过。

**当前验收状态：** 第 1、6、7、8、9 项已有 Settings 截图、辅助功能树、自动化、构建/安装指纹和持久化证据；Settings 中的 System、Antonio、DIN Condensed 与恢复默认切换也已直接观察。第 2 至 5 项中涉及菜单栏面板和三个非激活详情 `NSPanel` 的字体家族、长文本截断、字形与小号文字可读性尚未直接观察，仍需人工视觉验收。AX 的 `Detail open/closed` 状态与语义字体/布局回归是补充证据，不能替代该视觉检查。

## 10. 文档与 Git

实施完成后同步更新：

- `README.md`：字体选择的用户可见说明；
- `CHANGELOG.md`：新增字体设置；
- `docs/user-guide/settings.md`：选项、缺失字体和恢复默认行为；
- `docs/development/`：TDD、构建、安装和实机验收记录；
- 本规格状态和实施摘要；
- 实施计划完成状态及关键 Git 节点。

代码、测试、文档和验收分别保存清晰的 Git 检查点。未经用户选择，不推送远端或删除开发分支。

## 11. 明确不在本次范围内

- 任意字体文件导入或文件选择器；
- 字号、行距、字间距和粗细的用户自定义；
- 为不同 Provider 或不同窗口设置不同字体；
- 下载、安装、卸载或随 App 分发第三方字体；
- 修改 DeepSeek 官网 WebView 的字体；
- 改变现有布局、窗口尺寸、品牌配色、Logo 或浮动条轮廓。

## 12. 完成标准

Settings 稳定提供 System Default、Antonio、DIN Condensed 三个选择和恢复默认按钮；可用字体通过统一语义字体路径覆盖 AI Meter 自绘文字，缺失字体安全回退；新旧用户默认保持系统字体，当前机器最终选择 Antonio；完整自动化、Release 构建、安装、Settings 字体切换和持久化已有证据。菜单栏面板和三个详情在 Antonio/DIN 下的实际字体、长文本截断、字形和小号文字可读性仍须直接人工观察，完成后才能宣告真实 UI 全面验收通过。

## 13. 实施与验收摘要

> **2026-09-01 覆盖说明：** 本规格中「Settings 跟随显示字体」以及「字体选项使用各自字体预览」的相关条款，已由 [Settings 字体隔离与内容字号提升规格](2026-09-01-settings-font-isolation-and-content-size-step-design.md) 覆盖。本节保留原始实施与验收记录，不追溯改写历史结论。

- 领域、持久化、语义字体环境、全视图迁移和 Settings 控件分别由 `030f69d`、`ddd23f4`、`5685d91` 交付，并保留了逐阶段 TDD 红绿证据。
- 2026-08-31 的最终验证通过 170 个测试、35 个测试套件、0 失败；4 个依赖本机 Keychain/已安装 CLI 的检查按环境门控跳过。
- Release App Bundle 构建、ad-hoc 签名和 Info.plist 校验通过；候选版与 `/Applications/AI Meter.app` 的可执行文件 SHA-256 均为 `ca8a83ea29abb5f761dced018e9f664053311d4c28f939546f59200ec1822052`。
- 旧安装包可从 `/private/tmp/AI-Meter-app-backup-20260831-230934/AI Meter.app` 恢复。
- 实机 Settings 依次完成 System Default、Antonio、DIN Condensed、恢复默认、再次选择 Antonio 和重启持久化；最终保持 Antonio，浮岛恢复右侧 97%。Computer Use 能验证 Settings、选择状态、Provider 详情开关和浮岛辅助功能/视觉状态，但其窗口级截图不包含非激活详情 NSPanel 或菜单栏弹窗，因此尚未直接验证这些面板在 Antonio/DIN 下的字体、字形、截断和小号文字可读性；语义字体扫描、布局/视觉回归和可访问性状态仅作为补充证据，人工视觉验收仍待完成。
