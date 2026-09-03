# 架构决策记录

本页记录仍约束当前实现的长期决策。每项包含动机、代价和重新评估条件；历史视觉细节查[设计记录](../design/README.md)。

## D001：macOS 保持原生 SwiftUI + AppKit

- **状态：** 接受。
- **决定：** 使用 Swift 6、SwiftUI、AppKit、WebKit 和 WidgetKit，不引入 Electron 或常驻 Web 服务。
- **原因：** 需要桌面层窗口、菜单栏模板着色、Keychain、登录项、Space 行为和 WidgetKit 的原生控制。
- **代价：** 不能把 macOS UI 代码直接复用到 Windows，窗口层级和签名验收必须在真实系统上完成。
- **重新评估：** Apple 平台能力可在不降低 Widget、Space、Keychain 和菜单栏体验的前提下由共享壳层替代时。

## D002：统一 `UsageSnapshot` 是展示唯一数据入口

- **状态：** 接受。
- **决定：** Collector 把各外部格式转换成统一快照，UI、缓存、通知和 Widget 不直接解析 CLI/API/Web 数据。
- **原因：** 防止圆环、百分比、通知和详情分别计算而产生口径分裂。
- **代价：** 新 Provider 或新指标需要先扩展领域模型和兼容解码。
- **重新评估：** 快照模型无法表达某类服务，但应优先扩展而非绕过。

## D003：Claude Code 与 OpenAI Codex 凭证归官方 CLI

- **状态：** 接受。
- **决定：** 应用只启动固定官方登录命令和读取允许的账户/额度结果，不复制 OAuth Token、配置文件或浏览器登录。
- **原因：** 减少凭证责任并沿用官方 MFA、更新和撤销机制。
- **代价：** CLI 缺失、登录过期、格式变化或工作区信任都可能使采集不可用。
- **重新评估：** Provider 发布稳定、授权应用使用且有更小权限范围的官方 API。

## D004：Claude Code `/usage` 在私有空工作区执行

- **状态：** 接受。
- **决定：** 使用 `Application Support/AI Meter/ClaudeUsageWorkspace`，首次信任由用户本人批准。
- **原因：** 隔离用户项目中的 MCP、指令、插件和信任状态，避免改变额度命令。
- **代价：** 多一步首次授权，且历史兼容目录继续使用旧产品名。
- **重新评估：** Claude Code 提供无需工作区的稳定机器可读额度接口。

## D005：DeepSeek API Key 使用 Keychain，两阶段替换

- **状态：** 接受。
- **决定：** Key 只存 `AfterFirstUnlockThisDeviceOnly` Keychain；候选 Key 先调用官方余额接口验证，成功后才覆盖旧值。
- **原因：** 普通偏好和缓存不适合保存密钥，失败替换不能让现有配置失效。
- **代价：** ad-hoc 重签名可能改变访问控制身份；验证依赖网络。
- **重新评估：** DeepSeek 提供系统 OAuth 或安全代理授权。

## D006：DeepSeek 历史使用隔离 WebKit 会话

- **状态：** 接受但依赖未公开官网实现。
- **决定：** 只接受 `platform.deepseek.com` HTTPS 来源，等待用量与费用分片完整后标准化保存；不读取 Safari/Chrome Cookie。
- **原因：** 余额 API 不包含官网 30 天图表，隔离会话能让用户自行登录且不污染其他浏览器。
- **代价：** 官网接口变化会使图表暂时不可用；必须清楚显示缓存和登录状态。
- **重新评估：** DeepSeek 发布正式历史用量 API。

## D007：失败可缓存降级，但不能伪装实时

- **状态：** 接受。
- **决定：** 三个 Collector 并发；单项失败不阻塞其他项。最近成功快照可展示，但必须标记 cached/stale/unavailable 和更新时间。
- **原因：** 网络和 CLI 短暂故障不应让整个应用失去价值，也不能把旧数据冒充当前数据。
- **代价：** UI 必须同时表达数据值与采集状态。
- **重新评估：** 不再需要离线可见性时。

## D008：浮岛保持桌面层，详情短暂置前

- **状态：** 接受。
- **决定：** 浮岛位于 desktop icon level + 1，由普通/全屏应用自然覆盖；用户点击产生的详情使用标准 floating 层并在关闭后立即退出窗口栈。
- **原因：** 满足“只在桌面存在”与“点击后详情看得见”两个目标。
- **代价：** Mission Control、Space、多显示器和辅助功能需要额外真实系统验收。
- **重新评估：** 产品改为常驻置顶工具时。

## D009：可见品牌改名但保留兼容身份

- **状态：** 接受。
- **决定：** UI 和构建产物使用 AI Token Meter；保留 Bundle ID、可执行文件名、URL 身份、Keychain 服务、UserDefaults 和旧 Application Support 目录。
- **原因：** 视觉改名不应造成密钥、缓存、设置和工作区批准丢失。
- **代价：** 代码和磁盘中会继续出现 `AIMeter`/`AI Meter`，维护者必须区分显示名与兼容标识。
- **重新评估：** 有测试完备、可回滚的数据迁移版本时。

## D010：Widget 只读脱敏 App Group 快照

- **状态：** 代码接受，真实安装延期。
- **决定：** Widget 不联网、不运行 CLI、不读 Keychain；主应用写入版本化、原子、脱敏 JSON，再请求 WidgetKit 更新时间线。
- **原因：** 扩展生命周期短且权限边界应最小化。
- **代价：** 需要 Apple Development/发布签名和双方一致的 App Group，更新频率受系统预算控制。
- **重新评估：** WidgetKit 或平台共享机制发生变化。

## D011：Widget 构建必须条件签名

- **状态：** 接受。
- **决定：** `AI_METER_INCLUDE_WIDGET=auto|0|1`；没有有效身份时默认明确跳过，显式要求时失败，不生成不可用的 ad-hoc Widget。
- **原因：** 能编译 `.appex` 不等于系统会注册它；伪成功会误导用户。
- **代价：** 无证书环境只能交付普通主应用。
- **重新评估：** Apple 允许自签 App Group Widget。

## D012：菜单栏 Quantum Dial 以模板图像输出

- **状态：** 接受。
- **决定：** 18×18pt 黑色透明画布先渲染为 `NSImage`，设置 `isTemplate = true` 后交给系统着色。
- **原因：** SwiftUI `Color.primary` 直接绘制在动态菜单栏上可能与背景融合；模板图像能适配深浅、动态和高对比度菜单栏。
- **代价：** 菜单图标保持单色，不能沿用应用内品牌渐变。
- **重新评估：** macOS 提供对动态 SwiftUI 菜单栏自绘前景色的可靠合同。

## D013：需求台账和设计资料各只有一个入口

- **状态：** 接受。
- **决定：** `docs/requirements-backlog.md` 是唯一需求队列；所有规格和计划统一位于 `docs/design`；开发日志只记录实施证据。
- **原因：** 多个“下一阶段”文件和内部目录会让状态分叉，时间一长无法判断哪个有效。
- **代价：** 每个新请求都必须先登记，完成后必须回填证据。
- **重新评估：** 引入能保留同等审计链的正式 Issue/项目管理系统时。

## D014：CLI 发现不依赖交互式 Shell

- **状态：** 接受。
- **决定：** Finder 启动的应用自行检查用户级、Node 管理器、Homebrew、系统和已知 OpenAI App 内置路径；启动 Node 脚本时把所选可执行文件同目录置于子进程 PATH 首位。
- **原因：** 图形应用不加载 `.zshrc`，nvm 安装的 `codex` 和相邻 `node` 不会自然出现在 launchctl PATH；写死某个 Node 版本升级后会再次失效。
- **代价：** 需要维护有限的可信安装约定，并以真实可执行权限过滤候选。
- **重新评估：** OpenAI 提供不依赖外部 CLI 的稳定授权额度 API，或 macOS 为 GUI App 提供标准的用户 Shell 命令发现接口。

## D015：应用更新必须由用户发起并由 EdDSA 签名授权

- **状态：** 接受。
- **决定：** 使用固定版本的 Sparkle 读取 GitHub `appcast.xml`；只在用户点击 `Check for Updates` 时联网，只有用户点击 `Update Now` 才下载和安装。Release ZIP 必须通过内置公开键的 EdDSA 验证，生产私钥只保存在维护者 Keychain。
- **原因：** GitHub 提供稳定公开分发，但 HTTPS、Release 页面或 SHA-256 本身都不能代替发布者签名；手动触发同时符合本地优先产品的网络边界。
- **代价：** 发布过程必须维护 appcast 和离线签名密钥；`0.1.2` 到 `0.2.0` 需要手动替换一次，且 ad-hoc/not notarized 分发仍可能触发 Gatekeeper 提示。
- **重新评估：** 项目获得 Developer ID、公证和可信自动发布基础设施，或分发渠道迁移到 Mac App Store 时。

## D016：Windows 使用独立 Tauri/Rust 壳层，共享合同而非 UI 代码

- **状态：** 接受，Preview 真机验收中。
- **决定：** 保留 macOS SwiftUI/AppKit；Windows 使用 Tauri 2、Rust、React 与 Win32。双方通过根 `VERSION`、JSON Schema、展示合同、脱敏 fixture 与功能对等矩阵保持语义一致，不共享凭据或平台进程。
- **原因：** Windows 需要 Credential Manager、ConPTY、Job Object、WebView2、系统托盘和 Win32 窗口区域；直接移植 AppKit 不可行，把 macOS 改成跨平台 Web 壳又会破坏已验证能力。
- **代价：** 同一功能需要两份平台实现和各自真机验收；合同门禁只能证明数据/配置一致，不能证明像素或操作系统行为一致。
- **重新评估：** 两个平台壳层产生无法维护的长期重复，或出现能覆盖双方系统集成且不降低隐私边界的成熟共享框架时。

## D017：双平台共用版本与草稿 Release，更新信任根分离

- **状态：** 接受，首次 Preview 待演练。
- **决定：** macOS Sparkle 私钥留在维护者 Keychain，Windows Tauri 私钥只在 GitHub Actions Secret；同一版本/tag/Release 先保持草稿，macOS 资产复验和 Windows 签名构建均成功后才公开。macOS 读取 appcast，Windows 读取 `latest.json`。
- **原因：** 两平台签名工具和密钥托管条件不同，但用户需要一次 Release 获得同版本资产；草稿门禁避免任何平台先看到半成品更新。
- **代价：** 发布需要本机 macOS 签名步骤和云端 Windows job；缺少任一密钥时只能保留草稿。Windows 无 Authenticode 时仍会有 SmartScreen 发布者提示。
- **重新评估：** 建立可审计的硬件/云密钥托管、Developer ID 公证与 Authenticode 后，可把双方签名完全自动化，但仍保留双 job 发布门禁。
