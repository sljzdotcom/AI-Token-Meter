# 代码库结构

## 顶层目录

```text
AI-Meter/
├── Sources/                    # 产品源码
├── Tests/                      # 自动化测试与 fixture
├── docs/                       # 用户、架构、开发和历史设计文档
├── scripts/                    # 可移植测试、构建与维护脚本
├── appcast.xml                 # Sparkle 稳定版更新清单
├── Package.swift               # Swift Package 清单
├── README.md                   # 项目首页
├── CHANGELOG.md                # 版本变更
├── CONTRIBUTING.md             # 贡献规范
└── SECURITY.md                 # 安全政策
```

生成内容不进入版本控制：

- `.build/`：SwiftPM 构建目录；
- `dist/`：打包后的 `.app`；
- `DerivedData/`：Xcode 派生文件；
- `.worktrees/`：本地隔离开发工作树。

`scripts/test.sh` 把 SwiftPM 与 Clang 缓存隔离到临时目录，并把额外参数原样传给 `swift test`；`scripts/generate-app-icon.swift` 确定性绘制所有 macOS 图标尺寸；`scripts/build-app.sh` 执行 release 构建、图标打包、主应用/Widget 条件组装、Sparkle 嵌入与签名验证；`scripts/verify-widget-bundle.sh` 检查扩展；`scripts/verify-update-bundle.sh`、`package-update-release.sh` 和 `verify-update-archive.sh` 分别验证 App、生成正式更新资产并核对/抗篡改验证 appcast。

## `Sources/AIMeterApp`

```text
Sources/AIMeterApp/
├── AIMeterApp.swift            # App 入口与 SwiftUI scene
├── AppDelegate.swift           # 菜单栏、窗口和系统生命周期
├── AppModel.swift              # 主线程应用状态与刷新循环
├── SoftwareUpdate/             # 更新状态、协调器与 Sparkle 适配边界
├── Resources/
│   ├── Info.plist              # Bundle 元数据、最低系统版本、Agent App 标记
│   ├── AIMeterApp.entitlements # 签名构建时注入的 App Group 模板
│   └── Logos/                  # Claude Code、OpenAI Codex、DeepSeek 图标资源
├── System/
│   ├── ClaudeWorkspaceSetupLauncher.swift
│   ├── ClaudeDetailPanelLayout.swift
│   ├── CodexDetailPanelLayout.swift
│   ├── DeepSeekWebSession.swift
│   ├── FloatingDetailInteractionOwnership.swift
│   ├── FloatingPanelController.swift
│   ├── FloatingStripAccessibilityMovement.swift
│   ├── FloatingStripDisplayState.swift
│   ├── FloatingStripLayout.swift
│   ├── FloatingStripScreenResolver.swift
│   ├── InteractivePanel.swift
│   ├── LaunchAtLoginService.swift
│   ├── NotificationService.swift
│   └── WidgetSnapshotPublisher.swift
└── Views/
    ├── AboutSettingsView.swift
    ├── AppearanceSettingsView.swift
    ├── ClaudeDetailPresentation.swift # Claude Code 详情空状态、模型占比与无障碍文案
    ├── ClaudeDetailView.swift
    ├── CodexResetCreditsView.swift
    ├── CodexDetailView.swift
    ├── DeepSeekAnalyticsView.swift
    ├── AIMeterVisualTheme.swift
    ├── FloatingStripShape.swift
    ├── FloatingStripView.swift
    ├── MenuBarPanel.swift
    ├── MonitoringSettingsView.swift
    ├── ProviderAccentPalette.swift # 三服务品牌色与语义色优先级
    ├── ProviderCard.swift
    ├── ProviderLogo.swift
    ├── ProviderLogoStyle.swift
    ├── SettingsView.swift
    ├── SoftwareUpdateSettingsView.swift # About 更新状态与两个手动动作
    ├── SettingsTab.swift
    ├── ServicesSettingsView.swift
    ├── UsageRing.swift
    └── UsageVisualStyle.swift
```

目录原则：

- `Resources` 只放随 App Bundle 分发的静态资源和元数据；
- `System` 只放 macOS 或 WebKit 集成，不放用量业务解析；
- `SoftwareUpdate` 隔离 Sparkle 类型、状态和动作串行化，其他界面只依赖本项目协议；
- `Views` 只负责呈现与用户交互；
- `AppModel` 连接核心库与 App 生命周期，但不重新实现采集协议。

## `Sources/AIMeterWidgetExtension`

```text
Sources/AIMeterWidgetExtension/
├── AITokenMeterWidget.swift        # WidgetBundle 与 StaticConfiguration
├── WidgetTimelineSource.swift      # 只读 App Group 时间线、预览与过期降级
├── WidgetLayoutPolicy.swift        # Small/Medium/Large 字段合同
├── Resources/
│   ├── Info.plist
│   ├── AITokenMeterWidget.entitlements
│   ├── Backgrounds/
│   └── Logos/
└── Views/
    ├── SmallWidgetView.swift       # 仅三个 Logo 状态环
    ├── MediumWidgetView.swift      # 三张额度卡
    ├── LargeWidgetView.swift       # Provider 行、重置与充值券
    └── Widget*.swift              # 根视图、背景、Logo 与进度组件
```

该 target 不允许直接依赖网络、CLI、Keychain 或 WebKit，只依赖 `AIMeterCore` 中的脱敏 Widget 合同。资源在 SwiftPM 测试时使用资源 Bundle，正式 `.appex` 打包时复制到扩展 `Contents/Resources`。

## `Sources/AIMeterCore`

```text
Sources/AIMeterCore/
├── Collectors/                 # CLI/API 采集、进程执行与解析
├── Coordination/               # 多服务刷新和缓存降级
├── DeepSeek/                   # DeepSeek API、官网数据标准化
├── Domain/                     # 跨模块统一领域模型
├── Notifications/              # 用量阈值判定
├── Preferences/                # 浮岛侧边、屏幕与归一化位置偏好
├── Persistence/                # 快照与历史聚合持久化
├── Presentation/               # 面向 UI 的文案和语义状态
├── Security/                   # Keychain、SecretStore、敏感文本清理
├── UI/                         # 可单测的面板交互状态与偏好值
└── Widget/                     # 版本化展示快照、构建器与原子 App Group 存储
```

### Collectors

- `ANSITextSanitizer.swift`：清理终端控制字符；
- `ClaudeCollector.swift`：Claude Code 认证和 `/usage` 采集；
- `ClaudeLocalActivityReader.swift`：只读聚合当前 Mac 最近 30 天的 Claude Code 计量元数据；
- `ClaudeSetupScriptBuilder.swift`：生成一次性工作区设置命令；
- `ClaudeUsageParser.swift`：解析 Claude Code 用量；
- `ClaudeUsageWorkspace.swift`：管理专用空工作区；
- `CodexAppServerClient.swift`：OpenAI Codex JSON-RPC 客户端；
- `CodexLocalActivityReader.swift`：只读查询本机 OpenAI Codex 聚合列并计算 30 天活动；
- `CodexCollector.swift` / `CodexUsageParser.swift`：采集与解析 OpenAI Codex 数据；
- `CommandRunner.swift` / `PTYCommandRunner.swift`：普通和伪终端进程执行；
- `DeepSeekCollector.swift`：余额采集、隔离的钥匙串读取超时与单次在途保护；
- `ExecutableLocator.swift`：定位用户 PATH 之外的常见 CLI 安装路径；
- `TerminalUsageParser.swift`：终端文本解析共享逻辑；
- `UsageCollector.swift`：采集器协议。

### DeepSeek

- `DeepSeekClient.swift`：官方余额 API；
- `DeepSeekWebsitePayloadParser.swift`：验证、解析并合并官网用量/费用分片；
- `DeepSeekHistoryNormalizer.swift`：生成固定 30 天逐日聚合。

### Domain

- `UsageModels.swift`：服务、指标、状态和统一快照；
- `ProviderSupplementalData.swift`：OpenAI Codex 重置额度、本机活动与 DeepSeek 每日历史模型。

### Presentation

- `AppBrand.swift`：集中定义可见产品名称、副标题与版本文案；兼容身份仍由 Bundle 与既有持久化路径负责。
- `AppPresentation.swift`：把统一快照转换成菜单栏、卡片和状态展示模型。

## `Tests/AIMeterCoreTests`

测试文件与被测类型同名或以功能域命名，固定外部输出位于 `Fixtures/`。新增外部解析格式时，应同时提交：

1. 不含凭证和个人信息的最小 fixture；
2. 成功解析测试；
3. 缺字段或格式变化测试；
4. 敏感信息清理测试（若涉及错误或缓存）。

## `Tests/AIMeterAppTests`

覆盖只属于 macOS App 层、但可以从窗口和视图中抽离验证的边界逻辑：

- 浮岛玻璃拖动命中区、Logo 排除区和透明肩部排除；
- AppKit 窗口坐标与 SwiftUI 顶部坐标转换；
- 指针按下、拖动、松开状态及位置位移；
- 启动过程不在主线程同步读取 Keychain。

## `Tests/AIMeterWidgetExtensionTests`

- 锁定只支持 Small、Medium、Large 及各自可见字段；
- 锁定 Small 无可见文字、按钮或链接，同时保留无障碍标签；
- 验证缺失数据占位、过期快照降级与固定 Gallery 预览；
- 扫描扩展源码，防止引入网络、进程执行或 Keychain 路径。

## `docs`

- `project-status.md`：当前能力、版本、验证与未完成事项的事实快照；
- `requirements-backlog.md`：需求状态唯一来源；
- `user-guide/`：面向使用者的当前行为；
- `architecture/`：当前代码、数据流和长期决策；
- `development/`：开发环境、维护手册、测试、发布、提交历史和逐日日志；
- `design/specifications/`：历史设计方案；
- `design/implementation-plans/`：历史实施计划；
- `assets/`：README 与文档使用的图片。

历史设计目录不用于描述当前配置。功能演进后可以保留当时决策，但必须更新用户指南、架构文档与 `CHANGELOG.md`。

仓库不再维护第二份“下一阶段需求”文件，也不再使用内部命名的 `docs/superpowers`。所有新需求先进入台账，所有规格和计划进入 `docs/design`。

## 命名与放置规则

- 新的外部服务采集器放入 `Collectors/`，服务专属复杂转换可建立独立目录；
- 跨 UI 与采集共享的数据结构放入 `Domain/`；
- 只与界面计算有关的纯逻辑放入 `Presentation/` 或 `UI/`；
- macOS API、窗口、通知、登录项和 WebKit 生命周期放入 App 的 `System/`；
- 不把密钥、真实账户响应、生成的 `.app` 或构建缓存提交到仓库。
