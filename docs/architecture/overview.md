# 架构概览

## 目标

AI Token Meter 的架构围绕四个约束设计：

1. 三个服务互不拖累，单项失败不影响其他项；
2. 凭证由系统或官方 CLI 管理，业务层只接收必要数据；
3. UI 只消费统一领域模型，不直接解析终端或网页响应；
4. 网络或工具暂时不可用时保留可辨识的缓存，而不是显示伪实时数据。

## 模块边界

```text
Claude CLI ─┐
Codex CLI/DB┼─> Collectors ─> RefreshCoordinator ─> UsageSnapshot ─> AppModel ─> SwiftUI
DeepSeek API┘            │             │                    │
                         │             └─ SnapshotCache     ├─ Menu bar panel
DeepSeek WebKit ─> Normalizer ─> DeepSeekHistoryStore       ├─ Floating panel
                                                          └─ WidgetSnapshotPublisher
                                                                  │
App Group file <─ privacy-safe Widget envelope <───────────────────┘
      │
      └─> AIMeterWidgetExtension (read-only) ─> Small / Medium / Large

Keychain ────────────────> DeepSeekCollector
UserDefaults ────────────> UI preferences / threshold state
```

## AIMeterCore

`AIMeterCore` 是与窗口生命周期解耦的可测试核心库。

### Collectors

- `ClaudeCollector`：定位 CLI、检查认证、在隔离工作区通过 PTY 执行 `/usage`。
- `CodexCollector`：启动 `codex app-server` 读取 JSON-RPC 速率限制，并以只读 SQLite 查询补充本机线程活动聚合；本地查询失败不会拖累官方额度。
- `DeepSeekCollector`：从 `SecretStore` 取得 Keychain 密钥并调用余额 API。
- `ClaudeUsageParser`、`CodexUsageParser`：把外部格式转换成统一指标。
- `CommandRunner`、`PTYCommandRunner`：负责超时、进程终止和输出收集。

### Domain

`UsageSnapshot` 是 UI、缓存和通知共享的唯一用量快照。它包含：

- 服务标识；
- 主、次指标；
- 可用性与采集状态；
- 获取时间、过期时间和来源版本；
- Codex 重置额度摘要；
- Codex 本机 30 天活动摘要；
- DeepSeek 标准化历史用量。

### Coordination

`RefreshCoordinator` 并发请求各采集器，将失败映射为可行动状态，并在需要时使用最近缓存。协调层不负责界面文案布局。

### Persistence and Security

- `SnapshotCache`：统一快照缓存；
- `DeepSeekHistoryStore`：标准化每日用量缓存；
- `KeychainStore`：DeepSeek API Key；
- `SensitiveTextRedactor`：写缓存、错误消息和通知前清理常见敏感形态；
- `ThresholdEvaluator`：跨刷新保存 70% / 90% 通知状态。

### Widget shared contract

- `WidgetSnapshotBuilder` 重用 `ProviderPresentation`，确保主程序与 Widget 的额度选择、DeepSeek 基准消耗和状态含义一致；
- `WidgetSnapshotStore` 原子写入版本化 JSON，损坏、缺失或未知版本安全降级为空状态；
- 共享快照只包含三项展示值、进度、过期时间、最近重置与重置券数量/最近到期，不包含密钥、Cookie、手机号、邮箱或重置券内部 ID；
- `SensitiveTextRedactor` 在进入共享容器前再次清理所有可见字符串。

## AIMeterApp

`AIMeterApp` 负责 macOS 生命周期、系统服务与 SwiftUI 展示。

### AppModel

`AppModel` 是主线程状态中心：

- 创建采集协调器和 DeepSeek 网页会话；
- 启动后立即刷新，随后每 300 秒刷新；
- 应用 DeepSeek 余额基准和历史数据；
- 保存用户设置；
- 向窗口、通知和菜单栏发布状态。
- 在演示启动、真实刷新和 DeepSeek 基准变化后发布 Widget 展示快照，再请求 WidgetKit 更新；发布失败不改变主应用状态。

### Settings information architecture

`SettingsView` 只负责顶部 `TabView` 与设置消息路由，四个职责视图分别承载 Appearance、Monitoring、Services 和 About。服务配置反馈通过 `SettingsMessageKind` 定位到 Services，登录项错误定位到 Monitoring；后续设置按职责落入对应视图，避免重新形成单一长页面。Settings 根视图固定使用系统字体作用域，不继承浮动条和详情的可选显示字体。

### Brand and compatibility

`AppBrand` 集中提供 **AI Token Meter**、**Private AI usage monitor** 和版本文案。可见名称与构建产物已迁移，但 `com.millerpan.AIMeter`、`AIMeterApp`、Keychain 身份及 `Application Support/AI Meter` 兼容目录保持不变，以沿用现有偏好、缓存和 Claude 工作区批准。

视图主题层由 `ProviderAccentPalette` 集中映射 Claude、Codex 和 DeepSeek 的正常状态渐变；`UsageSemantic` 在 warning、critical、stale 和 unavailable 时覆盖品牌色，避免服务身份色削弱状态含义。

### FloatingPanelController

负责无标题桌面层浮窗、左右贴边定位、详情窗口、外部点击监听、自动隐藏任务和关闭时清理。悬浮条使用不激活 App 的 `NSPanel`；详情使用可成为 Key Window、但不会成为 Main Window 的专用 `InteractivePanel`。`FloatingDetailInteractionPolicy` 规定只有 DeepSeek 需要激活 App 并把 First Responder 交给网页，Claude、Codex 继续被动显示。切换或关闭详情会先清理 First Responder，SwiftUI View 不直接管理全局事件监听。

### DeepSeekWebSession

负责隔离 WebKit 会话、官方域名限制、页面状态和相关 JSON 的标准化入口。它分别接收官网每日 Token/请求与每日费用响应，只在两类数据都存在时合并并写入完整 30 天缓存。网页层与 API Key 余额采集相互独立；网页历史失败不会让余额失效。

## AIMeterWidgetExtension

Widget 扩展是独立的受沙箱进程，只从双方签名授权的 App Group 读取共享快照。`WidgetTimelineSource` 建议系统约 30 分钟后刷新时间线，并在快照超过 `expiresAt` 后显示陈旧状态；实际调度由 WidgetKit 的系统预算决定。Gallery 预览只使用固定示例数据，不读取真实账户。

- Small：三个 Logo 与状态环，不含可见文字、按钮或链接；无障碍标签仍包含服务和值；
- Medium：三个固定顺序额度卡，显示名称、主值、短标签和进度；
- Large：三项 Provider 行、最近重置、Codex 重置券数量与最近到期；
- 根视图使用 `aitokenmeter://open` 唤醒主应用，不提供登录、兑换或其他账户操作。

## 状态与降级

外部采集统一映射为以下状态：

- `fresh`：本轮实时成功；
- `cached`：实时失败但存在最近成功数据；
- `refreshing`：正在更新；
- `notInstalled`：对应 CLI 未找到；
- `authenticationRequired`：需要登录或重新配置密钥；
- `setupRequired`：Claude 私有工作区需一次性批准；
- `unavailable`：超时、网络或服务不可用；
- `unrecognizedOutput`：外部工具输出格式暂时无法识别。

UI 必须展示状态含义和更新时间，不能用旧数据覆盖失败而不作说明。

## 并发与生命周期

- `AppModel` 标记为 `@MainActor`；
- 每次刷新设置互斥状态，避免重复刷新重叠；
- 采集器和网络操作异步执行；
- 详情自动隐藏、全局点击监听和刷新循环都在退出时取消；
- 进程执行器在超时、取消和正常结束之间只完成一次 continuation。

## 可测试性

核心逻辑通过协议和纯数据结构隔离：

- 解析器使用固定文本/JSON fixture；
- Keychain 通过 `SecretStore` 抽象；
- 采集器、缓存和时钟相关逻辑可替换；
- UI 状态计算放在 Presentation 层；
- 提供商详情的激活与网页焦点规则放在纯策略中，AppKit 面板能力由独立测试目标验证；
- 真实 CLI 测试默认跳过，只有显式启用才接触本机账户。
