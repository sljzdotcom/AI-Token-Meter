# 代码库结构

## 顶层目录

```text
AI-Meter/
├── Sources/                    # 产品源码
├── Tests/                      # 自动化测试与 fixture
├── docs/                       # 用户、架构、开发和历史设计文档
├── scripts/                    # 可移植测试、构建与维护脚本
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

`scripts/test.sh` 把 SwiftPM 与 Clang 缓存隔离到临时目录，并把额外参数原样传给 `swift test`；`scripts/generate-app-icon.swift` 确定性绘制所有 macOS 图标尺寸；`scripts/build-app.sh` 执行 release 构建、图标打包、App Bundle 组装与 ad-hoc 签名验证。

## `Sources/AIMeterApp`

```text
Sources/AIMeterApp/
├── AIMeterApp.swift            # App 入口与 SwiftUI scene
├── AppDelegate.swift           # 菜单栏、窗口和系统生命周期
├── AppModel.swift              # 主线程应用状态与刷新循环
├── Resources/
│   ├── Info.plist              # Bundle 元数据、最低系统版本、Agent App 标记
│   └── Logos/                  # Claude、Codex、DeepSeek 图标资源
├── System/
│   ├── ClaudeWorkspaceSetupLauncher.swift
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
│   └── NotificationService.swift
└── Views/
    ├── CodexResetCreditsView.swift
    ├── CodexDetailView.swift
    ├── DeepSeekAnalyticsView.swift
    ├── AIMeterVisualTheme.swift
    ├── FloatingStripShape.swift
    ├── FloatingStripView.swift
    ├── MenuBarPanel.swift
    ├── ProviderCard.swift
    ├── ProviderLogo.swift
    ├── ProviderLogoStyle.swift
    ├── SettingsView.swift
    ├── UsageRing.swift
    └── UsageVisualStyle.swift
```

目录原则：

- `Resources` 只放随 App Bundle 分发的静态资源和元数据；
- `System` 只放 macOS 或 WebKit 集成，不放用量业务解析；
- `Views` 只负责呈现与用户交互；
- `AppModel` 连接核心库与 App 生命周期，但不重新实现采集协议。

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
└── UI/                         # 可单测的面板交互状态与偏好值
```

### Collectors

- `ANSITextSanitizer.swift`：清理终端控制字符；
- `ClaudeCollector.swift`：Claude 认证和 `/usage` 采集；
- `ClaudeSetupScriptBuilder.swift`：生成一次性工作区设置命令；
- `ClaudeUsageParser.swift`：解析 Claude 用量；
- `ClaudeUsageWorkspace.swift`：管理专用空工作区；
- `CodexAppServerClient.swift`：Codex JSON-RPC 客户端；
- `CodexLocalActivityReader.swift`：只读查询本机 Codex 聚合列并计算 30 天活动；
- `CodexCollector.swift` / `CodexUsageParser.swift`：采集与解析 Codex 数据；
- `CommandRunner.swift` / `PTYCommandRunner.swift`：普通和伪终端进程执行；
- `DeepSeekCollector.swift`：余额采集；
- `ExecutableLocator.swift`：定位用户 PATH 之外的常见 CLI 安装路径；
- `TerminalUsageParser.swift`：终端文本解析共享逻辑；
- `UsageCollector.swift`：采集器协议。

### DeepSeek

- `DeepSeekClient.swift`：官方余额 API；
- `DeepSeekWebsitePayloadParser.swift`：验证、解析并合并官网用量/费用分片；
- `DeepSeekHistoryNormalizer.swift`：生成固定 30 天逐日聚合。

### Domain

- `UsageModels.swift`：服务、指标、状态和统一快照；
- `ProviderSupplementalData.swift`：Codex 重置额度、本机活动与 DeepSeek 每日历史模型。

## `Tests/AIMeterCoreTests`

测试文件与被测类型同名或以功能域命名，固定外部输出位于 `Fixtures/`。新增外部解析格式时，应同时提交：

1. 不含凭证和个人信息的最小 fixture；
2. 成功解析测试；
3. 缺字段或格式变化测试；
4. 敏感信息清理测试（若涉及错误或缓存）。

## `docs`

- `user-guide/`：面向使用者的当前行为；
- `architecture/`：当前代码和数据流；
- `development/`：开发环境、测试、发布、提交历史和逐日日志；
- `design/specifications/`：历史设计方案；
- `design/implementation-plans/`：历史实施计划；
- `assets/`：README 与文档使用的图片。

历史设计目录不用于描述当前配置。功能演进后可以保留当时决策，但必须更新用户指南、架构文档与 `CHANGELOG.md`。

## 命名与放置规则

- 新的外部服务采集器放入 `Collectors/`，服务专属复杂转换可建立独立目录；
- 跨 UI 与采集共享的数据结构放入 `Domain/`；
- 只与界面计算有关的纯逻辑放入 `Presentation/` 或 `UI/`；
- macOS API、窗口、通知、登录项和 WebKit 生命周期放入 App 的 `System/`；
- 不把密钥、真实账户响应、生成的 `.app` 或构建缓存提交到仓库。
