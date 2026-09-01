# AI Token Meter WidgetKit 扩展设计

**日期：** 2026-09-01
**状态：** 视觉方案已确认；非视觉决策按用户预授权采用推荐方案
**目标平台：** macOS 14 及以上

## 目标

为 AI Token Meter 增加可放到 macOS 桌面和通知中心的原生 WidgetKit Widget。Widget 支持 `systemSmall`、`systemMedium`、`systemLarge` 三种尺寸，沿用浮动条的深海视觉语言，同时保持数据准确、隐私边界清晰，并在主应用未运行或数据过期时诚实降级。

## 已确认的视觉方向

用户确认：

- 视觉采用 **B「深海延续」**：深蓝底、低对比蓝色光带、独立 Provider 品牌色；
- 布局采用 **A「额度优先 + 分层详情」**；
- 小型 Widget 只显示 Claude、Codex、DeepSeek 三个 Logo 状态框和进度环，不显示名称、百分比或余额文字；
- 中型 Widget 横向显示三个 Provider 卡片：Logo、主值、短标签和进度条；
- 大型 Widget 左侧显示三个 Provider 行，右侧显示最近的额度重置和 Codex 重置券摘要；
- Provider Logo 使用应用现有本地资源，不下载网络图标；
- Claude、Codex、DeepSeek 正常状态继续使用各自既有品牌色；异常、缓存和不可用状态使用现有语义覆盖规则。

## 方案比较与选择

### 方案 1：主应用采集，App Group 共享脱敏快照（采用）

主应用继续负责 CLI、API、Keychain 和 WebKit 采集。每次刷新后，把专供 Widget 展示的最小脱敏快照写入 App Group 共享容器，并通知 WidgetKit 重载时间线。Widget 只读共享文件，不接触任何凭证，也不自行调用 Claude、Codex 或 DeepSeek。

优点是单一事实来源、准确性与当前应用一致、隐私边界最清晰，也符合 Apple 对应用与 Widget 扩展共享数据的推荐方式。

### 方案 2：Widget 直接读取旧 Application Support 缓存（不采用）

Widget 扩展默认受 App Sandbox 约束。让它读取主应用旧目录需要临时文件访问例外，不利于长期安全、签名和分发；本机探针也证明临时签名无法创建标准 App Group 容器。因此不使用临时例外绕过平台安全边界。

### 方案 3：Widget 自行采集三个服务（不采用）

这会让扩展接触 CLI 登录态或 DeepSeek Keychain 凭证，并受 WidgetKit 刷新预算、后台运行和沙箱约束影响。它会形成第二套采集逻辑，增加数据分叉与泄密风险，因此排除。

## 架构

```text
Claude / Codex CLI ─┐
DeepSeek API/WebKit ┼─> AppModel / RefreshCoordinator
Keychain ───────────┘              │
                                   ├─> 当前应用界面
                                   └─> WidgetSnapshotWriter
                                           │
                                  App Group JSON（脱敏、原子写入）
                                           │
                                  Widget TimelineProvider
                                           │
                                  Small / Medium / Large View
```

### 共享展示模型

新增 `WidgetSnapshotEnvelope`，版本固定为 `1`。它不直接持久化完整 `UsageSnapshot`，只包含 Widget 真正需要的展示数据：

- `generatedAt`：主应用生成时间；
- 固定顺序的三个 `WidgetProviderSnapshot`：
  - Provider 标识；
  - 已格式化主值，例如 `18%`、`31%`、`¥77.99`；
  - 短标签，例如 `Session`、`Weekly`、`Balance`；
  - `0...1` 的可选进度比例；
  - `normal`、`warning`、`critical`、`stale`、`unavailable` 语义；
  - 获取时间和过期时间；
- `nextReset`：Claude/Codex 所有可解析重置时间中最早的未来项，包含 Provider、标签和时间；
- `codexResetCredits`：只包含可用数量和最近到期时间，不包含 redeem ID 或任何可兑换操作。

共享模型只使用 Foundation 和 Codable，以便主应用与 Widget 扩展编译同一份源文件。所有显示字符串在写入前继续经过 `SensitiveTextRedactor`；共享文件不得包含 API Key、Cookie、Authorization、CLI 原始输出、会话内容、邮箱、手机号或 Codex 重置券 ID。

### 数据写入

主应用在以下时机写入：

1. 启动后首次得到缓存或实时快照；
2. 每次 `RefreshCoordinator` 完成且发布新快照；
3. DeepSeek 余额基准变化，导致环形比例变化；
4. 登录状态、缓存状态或不可用状态发生变化。

写入采用临时文件加原子替换。成功后调用 `WidgetCenter.shared.reloadTimelines(ofKind:)`。Widget 文件损坏或版本未知时，读取器返回空状态，不让扩展崩溃。

### Widget 时间线

Widget 不联网、不执行 CLI。`TimelineProvider` 每次只读当前共享快照：

- 主应用有新数据时，由 `WidgetCenter` 主动请求重载；
- 主应用不运行时，时间线以约 30 分钟为下次建议刷新点，用于更新过期状态和重置时间；
- WidgetKit 可根据系统预算合并或推迟刷新，因此界面必须显示缓存/过期语义，不能承诺精确到分钟的后台采集；
- `placeholder` 和 gallery `snapshot` 使用明确的示例数据，不读取真实账户。

Apple 说明 Widget 由独立进程渲染，时间线刷新受每日预算控制，常见刷新间隔约为 15–60 分钟；应用数据变化时可通过 `WidgetCenter` 请求重载。参考：[Keeping a widget up to date](https://developer.apple.com/documentation/widgetkit/keeping-a-widget-up-to-date/) 与 [TimelineProvider](https://developer.apple.com/documentation/widgetkit/timelineprovider)。

## 三种尺寸的精确内容

### Small

- 三个等宽 Logo 状态框横向排列；
- 框外环显示进度；无有效比例时只显示中性底环；
- 不显示任何可见文字；
- 无障碍标签完整读出 Provider、主值、状态和更新时间；
- 任一数据不可用时对应 Logo 仍可辨识，并增加非颜色状态符号。

### Medium

- 三个等宽 Provider 卡片；
- 每张卡显示 Logo、Provider 名称、主值、短标签、进度条；
- Claude/Codex 主值使用风险最高的官方额度窗口，与浮动条和菜单栏一致；
- DeepSeek 主值显示官方余额，进度表示相对设置基准的已消耗比例；
- 卡片空间不足时优先保留 Logo、主值和状态，重置说明不进入中型布局。

### Large

- 左侧三行 Provider：Logo、短标签、进度条和主值；
- 右上显示最早的未来额度重置；如果没有可解析时间，则显示 `Reset time unavailable`；
- 右下显示 Codex 可用重置券数量及最近到期日；没有摘要时显示 `No reset credit data`，显式为零时显示 `0 available`；
- 大型布局不加入 30 天 Token 图表、Codex 本机活动或兑换按钮，避免与详情页重复并保持一眼可读。

## 交互

- Widget 整体点击通过 `aitokenmeter://open` 打开 AI Token Meter；
- 首版不加入按钮、切换器、刷新操作或 Provider 配置；
- 打开主应用后沿用当前默认窗口行为，不自动弹出登录页或兑换流程；
- 主应用 Info.plist 增加 URL scheme，但不接受外部参数或敏感数据。

## 外观与无障碍

- Widget 背景复用当前深海背景资源或视觉等价的静态本地资源，使用系统 `containerBackground`；
- 默认显示字体采用系统字体。Settings 的字体选择不影响 Widget，避免扩展依赖本机未安装字体；
- Dynamic Type、Increase Contrast、Reduce Transparency 和不同桌面着色模式下保持可读；
- 所有 Provider 不能只靠颜色区分；缓存、严重和不可用状态必须具有状态符号及无障碍文本；
- 预览和占位符不使用用户真实余额或额度。

## 打包与签名

现有应用仍以 Swift Package 和 `scripts/build-app.sh` 构建。Widget 扩展使用仓库内受版本控制的 Swift 源文件、Info.plist、entitlements 和资源，输出嵌入：

`AI Token Meter.app/Contents/PlugIns/AITokenMeterWidget.appex`

主应用 Bundle ID 保持 `com.millerpan.AIMeter`；Widget 使用 `com.millerpan.AIMeter.Widget`。App Group 的最终标识以实际 Apple Team ID 为前缀，并在主应用与扩展的 entitlements 中完全一致。

本机当前 `security find-identity -p codesigning` 返回 `0 valid identities found`，Xcode 也没有已登录开发账户。临时签名探针可以携带 App Group entitlement，但 `containerURL(forSecurityApplicationGroupIdentifier:)` 返回 `nil`，因此不能伪装为可用共享容器。

实现与自动化测试可以先完成；最终安装带真实数据的桌面 Widget 前，需要在 Xcode 登录 Apple Account 并产生 Apple Development 签名。Apple 文档说明 App Group 用于主应用与扩展共享容器，能力与签名需为相关 target 配置。参考：[Configuring app groups](https://developer.apple.com/documentation/xcode/configuring-app-groups) 与 [Adding capabilities to your app](https://developer.apple.com/documentation/xcode/adding-capabilities-to-your-app)。

构建脚本不得静默降级为“看似安装成功但读不到数据”的 Widget：

- 普通临时签名构建继续生成不含 Widget 的主应用，保持现有开发流程；
- 请求 Widget 构建时若缺少签名或 Team ID，立即给出明确错误和一次性处理指引；
- 有有效签名时先签 Widget，再嵌入并签主应用，最后执行深度签名验证与 App Group entitlement 核对。

## 错误与降级

- 共享文件缺失：显示三项 Logo 的空状态，中/大型显示 `Open AI Token Meter to load data`；
- 文件损坏或版本未知：等同空状态，同时主应用日志只记录错误类别；
- 单 Provider 不可用：其他两项继续显示，失败项使用不可用状态；
- 数据超过自身 `staleAfter`：使用缓存视觉并保留上次值，不伪装成实时；
- 重置时间已过：重新从当前共享快照选择未来项；没有未来项则显示不可用；
- App Group 写入失败：主应用其他界面不受影响，日志不记录快照内容。

## 测试与验收

### 自动化

1. 共享模型编码、解码、版本拒绝和固定 Provider 顺序；
2. 从 `UsageSnapshot` 生成 Widget 展示数据，验证 Claude/Codex 最高风险窗口与 DeepSeek 余额/消耗比例；
3. 最早未来重置与 Codex 最近到期券选择；
4. 隐私回归：共享 JSON 不含凭证、邮箱、手机号、Cookie、原始输出或 redeem ID；
5. 缺失、损坏、过期和单项失败降级；
6. Small/Medium/Large 支持集合与内容契约；
7. 构建产物包含正确 `.appex`、Info.plist、Bundle ID、资源和 entitlements；
8. 普通无签名构建不伪造 Widget 成功，签名缺失错误可操作；
9. 完整现有测试继续通过。

### 本机验收

1. 构建并安装签名版应用，确认 Widget Gallery 可找到 **AI Token Meter**；
2. 分别添加小、中、大三种尺寸到桌面；
3. 对照主应用核对 Claude、Codex、DeepSeek 主值、进度和状态；
4. 刷新主应用，确认 Widget 在系统允许的短时间内更新；
5. 退出主应用，确认 Widget 保留脱敏缓存并在过期后显示缓存状态；
6. 点击 Widget，确认只打开主应用；
7. 切换浅色/深色、提高对比度和减少透明度，检查可读性；
8. 核查签名、嵌入扩展和 App Group entitlement；
9. 检查共享 JSON，不含敏感数据。

## 非目标

- 不在 Widget 内登录、输入 API Key、兑换 Codex 重置券或启动 CLI；
- 不给 Widget 单独的网络权限；
- 不加入交互按钮、Provider 筛选、背景选择或尺寸内自定义；
- 不把 DeepSeek 30 天图表或 Codex 本机活动复制进 Widget；
- 不迁移或删除现有 `Application Support/AI Meter` 缓存；
- 不改变浮动条、详情页、菜单栏和 Settings 的现有布局。
