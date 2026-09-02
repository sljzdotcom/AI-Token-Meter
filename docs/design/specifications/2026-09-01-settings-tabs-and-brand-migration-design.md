# AI Token Meter：Settings 分类与品牌迁移设计

**日期：** 2026-09-01  
**状态：** 视觉方案已确认，等待书面规格审查  
**范围：** 下一阶段需求 R1、R4、R5；不包含 WidgetKit（R3）或新的数据采集能力

## 1. 目标

本阶段完成两项相互关联的产品整理：把持续增长的 Settings 从单页表单改为清晰的四类顶部 Tab，并把所有当前用户可见品牌从 `AI Meter` 统一为 `AI Token Meter`。改名必须保留现有登录、Keychain、历史缓存、位置、字体和通知偏好，不能把一次视觉改名变成数据迁移风险。

确认后的副标题为 `Private AI usage monitor`。现有深蓝底、青绿到紫色进度环和白色仪表指针 App Icon 保留，不重新设计。

## 2. Settings 信息架构

Settings 使用原生 SwiftUI `TabView`，顶部固定四个 Tab。整个 Settings 根视图继续应用 `.aiMeterFontScope(.settings)`，因此无论用户选择 System、Antonio 或 DIN Condensed，Settings 的 Tab、标题、控件、说明和关于信息始终使用 macOS 系统字体与系统字号。

### 2.1 外观

- 显示浮动条；
- 屏幕边缘：Automatic、Left、Right；
- 详情自动隐藏；
- 显示字体；
- 恢复默认字体及字体缺失说明。

### 2.2 监控

- 刷新间隔的当前只读值；
- 70% 与 90% 用量通知开关；
- 登录时启动开关。

### 2.3 服务

- Claude：说明认证由 Claude Code CLI 管理；不新增凭证输入；
- Codex：说明认证由 Codex CLI 管理；不新增凭证输入；
- DeepSeek：余额基准、API Key 安全输入、保存状态和移除操作；
- Settings 消息仍在与触发操作相同的 Tab 内显示，避免用户切换页面寻找结果。

### 2.4 关于

- App Icon、`AI Token Meter` 和 `Private AI usage monitor`；
- 从 Bundle 元数据读取版本与构建号，不在 SwiftUI 中复制常量；
- 简短隐私说明：Claude/Codex 凭证由官方 CLI 管理，DeepSeek Key 存于 Keychain，历史缓存只保存标准化聚合数据；
- 不新增网络请求、更新检查、遥测或外部账户入口。

### 2.5 窗口与导航

- 设置窗口保持紧凑，不引入左侧边栏；建议基准尺寸为 560 × 540，内容较少的 Tab 允许保留留白，不通过切换 Tab 频繁改变窗口尺寸；
- Tab 顺序固定为外观、监控、服务、关于；新增设置按职责进入已有 Tab，只有出现全新职责域时才新增 Tab；
- 键盘、VoiceOver 标签、Tab 顺序和现有设置绑定保持可用；切换 Tab 不触发刷新或清空未保存的 DeepSeek Key 输入。

## 3. 组件边界

`SettingsView` 只管理当前 Tab 和整体窗口作用域。四个内容区域拆成专用 View，避免继续扩大单个文件：

- `AppearanceSettingsView`
- `MonitoringSettingsView`
- `ServicesSettingsView`
- `AboutSettingsView`

共享的 `SettingsTab` 只负责稳定标识、显示名称和 SF Symbol。现有 `AppModel` 仍是所有设置状态的唯一入口；本阶段不复制偏好存储，也不改变现有 setter 的行为。

品牌文案集中到一个小型 `AppBrand` 值域，供菜单面板、关于页、可访问性文本和测试使用。Bundle 元数据仍由 `Info.plist` 提供；Swift 常量不能替代系统元数据。

## 4. 品牌改名与兼容策略

### 4.1 改为新名称的用户可见位置

- `CFBundleDisplayName` 与 `CFBundleName`；
- 构建产物 `dist/AI Token Meter.app`；
- 菜单面板标题、副标题、退出提示和用户可见状态信息；
- README、用户指南、排障、隐私说明、当前架构与发布文档；
- 新增开发日志与 `CHANGELOG.md` 的 Unreleased 记录。

### 4.2 保持不变的兼容标识

- Bundle Identifier：`com.millerpan.AIMeter`；
- 可执行文件名与 Swift target：`AIMeterApp`；
- Swift Package 内部名称与源码模块名；
- `Application Support/AI Meter` 数据目录；
- 既有 UserDefaults key、Keychain service/account、LaunchAtLogin 注册身份和通知标识。

这些标识属于持久化兼容层，不向用户展示。保留它们可让新名称安装后直接读取原有 API Key、缓存、字体、贴边位置和自动隐藏设置。源码必须对这些旧字符串加兼容注释，避免以后误当成遗漏文本批量替换。

### 4.3 本机安装迁移

构建脚本只生成 `AI Token Meter.app`。本次验收安装按以下顺序执行：

1. 完全退出当前 `AIMeterApp`；
2. 把 `/Applications/AI Meter.app` 和已有 `/Applications/AI Token Meter.app` 中实际存在的旧包分别移动到带时间戳的 `/private/tmp` 备份目录；
3. 安装新候选到 `/Applications/AI Token Meter.app`；
4. 校验严格签名、Info.plist、arm64 架构、候选与安装可执行文件 SHA-256；
5. 启动新名称，确认旧偏好、DeepSeek Keychain 状态、位置和字体选择仍保留；
6. `/Applications` 最终只保留新名称，避免重复登录项或两个菜单栏进程。

如验收失败，退出新包并从备份恢复旧包。迁移过程不删除 Keychain、Application Support 或 UserDefaults。

## 5. 数据流与状态

Settings Tab 选择只存在于当前设置窗口会话，不需要跨启动持久化。所有已有控件继续直接绑定 `AppModel`；DeepSeek API Key 输入只保存在 `ServicesSettingsView` 的临时内存状态中，保存后立即清空，不进入日志或文档。

品牌改名不改变三个 Provider 的刷新、额度、余额、重置券、30 天历史或通知算法。Widget 所需的共享容器、App Group 和时间线数据不在本阶段引入。

## 6. 错误与降级

- 自定义字体缺失时继续显示禁用状态并安全回退；Settings 自身不受影响；
- DeepSeek Key 保存或移除结果显示在服务 Tab，不泄露 Key；
- Bundle 版本字段缺失时关于页显示 `Version unavailable`，不崩溃；
- 旧安装包不存在时安装迁移直接继续；备份或安装失败时立即停止，不覆盖唯一可用版本；
- 保持旧数据目录不可读时的现有服务降级，不自动创建第二套空数据以伪装迁移成功。

## 7. 测试与验收

### 7.1 自动化

1. `SettingsTab` 恰好包含四项、顺序稳定、名称和图标明确；
2. 四个设置区域只包含归属自己的控件，根 `SettingsView` 使用系统字体作用域；
3. 字体选项仍只显示名称，不使用对应字体预览；
4. 菜单标题、副标题、退出提示和可访问性品牌文案使用新名称；
5. Info.plist 的显示名/Bundle 名、构建脚本 App 名和产物路径统一；
6. 兼容标识保持不变，并通过测试列出允许保留的旧字符串；
7. About 版本格式在字段存在与缺失时均稳定；
8. 完整现有测试无回归。

### 7.2 构建与安装

- 完整测试、Release 构建、plist lint、严格签名、arm64 和空白检查全部通过；
- 候选与安装包主可执行文件 SHA-256 一致；
- `/Applications` 只保留 `AI Token Meter.app`；
- 菜单栏、Settings、About、Dock/Finder 信息使用新名称和现有仪表图标；
- Antonio、Right、约 97%、自动隐藏 8 秒及 DeepSeek Key 配置状态在改名后保持。

### 7.3 人工界面

- 四个 Tab 可点击、键盘导航正常、没有文字截断；
- Settings 始终为系统字体；
- DeepSeek Key 输入切换 Tab 后不意外丢失；
- 菜单标题和副标题清晰，Settings 标题与应用名一致；
- Launch at Login 关闭再开启后指向新安装包，重启应用仍只有一个菜单栏实例。

## 8. 文档与历史边界

README、当前用户指南、架构、设置与发布文档统一使用新产品名。历史设计规格和开发日志保留当时的 `AI Meter` 文字，避免重写 Git 历史事实；在文档索引中说明 2026-09-01 起可见名称改为 `AI Token Meter`。

`docs/next-phase-requirements.md` 更新为：R1、R4、R5 完成；R2 保留人工环境验收边界；R3 Widget 待下一阶段。此前“尚未合并 main”的过期状态同步修正。

## 9. 非目标

- 不实现 WidgetKit；
- 不修改 Provider Logo、浮动条背景、肩部曲线或品牌色；
- 不迁移 Bundle Identifier、Keychain 或 Application Support 路径；
- 不增加自动更新、遥测、在线帮助中心或新的登录方式；
- 不解决 Mission Control、多显示器等 R2 人工验收项，这些在 Widget 前的验收收尾阶段单独处理。

## 10. 完成标准

四 Tab Settings、品牌改名、兼容保留、构建安装迁移和文档同步全部落在 `main`；自动化、签名和安装身份通过；用户可见界面只使用 `AI Token Meter` 与 `Private AI usage monitor`；现有账户状态、缓存和偏好无损；Widget 需求继续作为下一份独立规格。
