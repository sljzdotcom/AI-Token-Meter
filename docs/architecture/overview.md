# 架构概览

## 目标

AI Meter 的架构围绕四个约束设计：

1. 三个服务互不拖累，单项失败不影响其他项；
2. 凭证由系统或官方 CLI 管理，业务层只接收必要数据；
3. UI 只消费统一领域模型，不直接解析终端或网页响应；
4. 网络或工具暂时不可用时保留可辨识的缓存，而不是显示伪实时数据。

## 模块边界

```text
Claude CLI ─┐
Codex CLI ──┼─> Collectors ─> RefreshCoordinator ─> UsageSnapshot ─> AppModel ─> SwiftUI
DeepSeek API┘            │             │                    │
                         │             └─ SnapshotCache     ├─ Menu bar panel
DeepSeek WebKit ─> Normalizer ─> DeepSeekHistoryStore       └─ Floating panel

Keychain ────────────────> DeepSeekCollector
UserDefaults ────────────> UI preferences / threshold state
```

## AIMeterCore

`AIMeterCore` 是与窗口生命周期解耦的可测试核心库。

### Collectors

- `ClaudeCollector`：定位 CLI、检查认证、在隔离工作区通过 PTY 执行 `/usage`。
- `CodexCollector`：启动 `codex app-server`，读取 JSON-RPC 速率限制。
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
- DeepSeek 标准化历史用量。

### Coordination

`RefreshCoordinator` 并发请求各采集器，将失败映射为可行动状态，并在需要时使用最近缓存。协调层不负责界面文案布局。

### Persistence and Security

- `SnapshotCache`：统一快照缓存；
- `DeepSeekHistoryStore`：标准化每日用量缓存；
- `KeychainStore`：DeepSeek API Key；
- `SensitiveTextRedactor`：写缓存、错误消息和通知前清理常见敏感形态；
- `ThresholdEvaluator`：跨刷新保存 70% / 90% 通知状态。

## AIMeterApp

`AIMeterApp` 负责 macOS 生命周期、系统服务与 SwiftUI 展示。

### AppModel

`AppModel` 是主线程状态中心：

- 创建采集协调器和 DeepSeek 网页会话；
- 启动后立即刷新，随后每 300 秒刷新；
- 应用 DeepSeek 余额基准和历史数据；
- 保存用户设置；
- 向窗口、通知和菜单栏发布状态。

### FloatingPanelController

负责无标题悬浮窗口、屏幕右侧定位、详情窗口、外部点击监听、自动隐藏任务和关闭时清理。右侧悬浮条使用不激活 App 的 `NSPanel`；详情使用可成为 Key Window、但不会成为 Main Window 的专用 `InteractivePanel`。`FloatingDetailInteractionPolicy` 规定只有 DeepSeek 需要激活 App 并把 First Responder 交给网页，Claude、Codex 继续被动显示。切换或关闭详情会先清理 First Responder，SwiftUI View 不直接管理全局事件监听。

### DeepSeekWebSession

负责隔离 WebKit 会话、官方域名限制、页面状态和相关 JSON 的标准化入口。网页层与 API Key 余额采集相互独立；网页历史失败不会让余额失效。

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
