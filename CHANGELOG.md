# Changelog

本文件记录 AI Token Meter 面向使用者的主要变化。格式参考 [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)，版本号遵循语义化版本思路。

## Unreleased

## 0.1.2 - 2026-09-02

### Added

- OpenAI Codex 确实未安装时，Services 显示可操作的 OpenAI 官方 CLI 安装指南入口。
- 增加公开项目所需的 MIT License、行为准则、支持说明、Issue/PR 模板、macOS CI 与脱敏产品截图。
- README 增加英文摘要、GitHub Release 安装说明和作者 Miller；About 页面同步显示作者信息。

### Fixed

- 修复完整并发测试或高负载下，PTY 子进程退出等待同步占用 Swift 并发线程池，进而导致 Claude/Codex 输出丢失或误报超时的问题；退出通知现在采用真正异步的 continuation，并覆盖 32 路并发回归。
- 修复 Finder 启动的 AI Token Meter 无法发现通过 nvm 安装在 `~/.nvm/versions/node/*/bin` 中的 OpenAI Codex CLI，因而错误显示 `CLI not installed` 的问题。
- 启动 Codex `app-server` 和登录脚本时把所选 CLI 的同目录放到 PATH 首位，使 `#!/usr/bin/env node` 能找到匹配的 Node 运行时。
- 增加已安装 ChatGPT/Codex 桌面应用内置原生 `codex` 的安全后备路径，并保持用户显式 CLI 优先。

### Security

- 增加公开发布门禁，对当前文件、完整 Git 历史和 Release ZIP 执行高置信度凭据检查，并在可用时使用 Gitleaks 复核且不回显秘密。

## 0.1.1 - 2026-09-02

### Added

- 新增当前项目状态、架构决策、维护手册与全项目复盘文档，并把断链、版本、测试基线和目录治理纳入自动检查。
- 新增原生 macOS WidgetKit 桌面组件，支持 Small、Medium、Large；最小尺寸仅显示 Claude、Codex、DeepSeek 三个 Logo 状态环，中型显示三张额度卡，大型追加最近重置与 Codex 重置券摘要。
- 新增隐私安全的 App Group Widget 快照、30 分钟系统时间线建议、过期状态降级和点击唤醒主应用深链；Widget 本身不联网、不调用 CLI、不访问 Keychain。
- 构建脚本新增 `AI_METER_INCLUDE_WIDGET=auto|0|1`、Apple Development 身份检测、嵌套扩展签名及 App Group 一致性验证；无开发签名时普通主应用仍可构建。
- Settings 采用 Appearance、Monitoring、Services、About 四个顶部 Tab，并按职责安置现有选项。
- Services 新增 Claude、Codex、DeepSeek 常驻账户状态；Claude/Codex 支持通过官方 CLI 一键登录或重新登录、有限自动回查和手动检查状态。
- DeepSeek 设置新增遮罩 Key 身份和两阶段替换：候选 Key 先通过官方余额接口验证，验证成功后才更新 Keychain。
- Appearance 新增全局显示字体选择：System Default、Antonio、DIN Condensed，以及 `Restore Default Font`；可用字体会即时应用到 App 自绘文字并持久化，缺失字体禁用且安全回退到系统字体。第三方字体须由用户预先安装，AI Token Meter 不下载或分发字体文件。
- 显示字体目录新增 Alimama FangYuanTi VF、Fira Code、Leigo、Menlo、Alimama DaoLiTi，支持已安装检测、别名解析与系统字体安全回退。
- 贴边浮岛内部新增静态黑蓝「深海波纹」背景；左右贴边时仅纹理随轮廓镜像，Logo、品牌进度色、点击和拖动行为保持不变，资源缺失时自动回退到原玻璃底色。
- 可配置的详情自动隐藏时间：3、5、8、15 或 30 秒，默认 8 秒。
- 点击悬浮条和详情以外区域立即关闭详情；悬停和 DeepSeek 登录交互暂停倒计时。
- Claude 专用空工作区和一次性批准入口，减少用户项目、MCP 与工作区信任对 `/usage` 的干扰。
- Claude 新增额度优先专用详情页，并补充明确标注为 `This Mac` 的 Claude Code 最近 30 天会话、活跃日、Token 总量与每日趋势。
- Codex 可用重置额度、名称和到期日的只读展示。
- Codex 额度优先详情页，以及近 30 天本机 Token、当前连续使用天数和最长会话三项聚合。
- DeepSeek 可配置余额基准，默认 ¥100。
- DeepSeek 最近 30 天成本、API 请求数、Token 数和每日成本图表。
- DeepSeek App 内隔离官网登录会话、标准化历史缓存与缓存降级。
- 三个悬浮圆环统一为大尺寸品牌 Logo，并保留完整无障碍描述。
- 支持 Automatic、Left、Right 三种贴边模式；自动模式可在拖动结束时吸附最近侧边。
- 记住浮岛所在显示器、最终侧边和垂直位置，显示器布局变化时重新夹紧到可见区域。
- 新增无文字仪表指针 App Icon，并在本地构建时生成全部 macOS 图标尺寸。
- 完整 GitHub 风格文档体系：用户指南、架构、隐私、安全、开发、测试、发布和提交历史。
- 可移植测试脚本，把 SwiftPM 与 Clang 缓存隔离到临时目录，兼容 Dropbox 和受限执行环境。

### Changed

- 需求状态统一由 `docs/requirements-backlog.md` 管理，全部设计规格和实施计划统一归档到 `docs/design`；删除已被正式记录覆盖的旧需求副本与临时代理报告。
- 当前界面、辅助功能、通知、Widget 与现行文档中的服务名称统一为 **Claude Code**、**OpenAI Codex** 和 **DeepSeek**；CLI 命令、路径、缓存标识与历史记录保持兼容。
- 菜单栏顶部的通用仪表 SF Symbol 改为自绘 18×18pt Quantum Dial；断环进度与指针跟随三项服务中的最高有效已用比例，保留精确百分比文字，无数据时使用中性状态，并自动适配 macOS 菜单栏前景色。
- 产品显示名称改为 **AI Token Meter**，副标题改为 **Private AI usage monitor**，构建产物改为 `dist/AI Token Meter.app`；Bundle Identifier、可执行文件名、Keychain 身份和旧数据目录保持兼容。
- Claude 详情页移除 Token composition、Top models 以及底部的内联隐私说明，保留官方额度、本机三项统计和每日趋势；底层隐私保护、采集与旧缓存兼容性不变。
- Settings 现在固定使用 macOS 系统字体，字体选项只显示名称；浮动条、三个详情页和菜单点击面板的产品文字统一增大 1pt。
- Claude、Codex、DeepSeek 现在分别使用黄橙、玫红紫、薄荷紫品牌渐变，并同步到圆环、菜单卡片、详情进度条、标题和关键数据；Claude 与 Codex 的异常语义色仍优先，DeepSeek 始终保留用于表达余额消耗的原薄荷紫渐变。
- Codex 重置券改为分层卡片：突出可用数量、完整到期时间和自然日剩余状态，并按券数量自适应详情高度。
- 悬浮条由带边距的圆角矩形改为左右可镜像的无缝贴边浮岛；详情始终向桌面内部展开。
- Claude、Codex、DeepSeek 详情页统一为深色玻璃表面、青绿至蓝紫重点色和一致的卡片层级。
- Claude、Codex、DeepSeek Logo 使用同一套光学校正规则，使三个图形在 60 点圆环中的视觉重量接近。
- Claude 交互命令以终端 Enter 对应的 CR 提交，并在读取到额度结果后尽快结束。
- Claude 解析器忽略促销说明中的百分比，只接受明确的已用/剩余额度行。
- Codex 优先展示顶层通用速率限制，不再被模型专属窗口覆盖。
- 圆环和中央数值共享同一展示指标；0% 或无百分比指标时不再绘制虚假最小弧。
- DeepSeek 圆环由“余额文本”改为相对余额基准的已消耗比例。
- DeepSeek API Key 输入框不再回显旧值；替换失败时保留输入与原 Key，成功后才清空并刷新额度。
- 历史设计资料从内部命名的 `docs/superpowers` 迁移到 `docs/design`。

### Fixed

- 修复将 Release ZIP 复制到另一台 Mac 后，SwiftPM 图片资源仍回退到构建机绝对临时路径并导致应用启动崩溃的问题；主应用资源现直接嵌入标准 `Contents/Resources`，发布构建会强制验证完整资源布局。
- 修复自绘 Quantum Dial 在部分深色或动态菜单栏背景下与背景融合、看起来不可见的问题；图形现以 macOS 模板图像输出，由系统按当前菜单栏前景色可靠着色。
- Claude 本机活动改为受时限约束的可选数据源：官方额度失败会立即返回，官方额度成功也最多等待 2 秒；本机扫描采用流式读取、30 日文件时间过滤、总量/文件数/持续时间上限，避免大型历史目录拖慢统一刷新。
- 旧缓存与新采集中的 Claude 模型标识会执行长度、字符集和敏感文本校验；详情补充显式空状态和官方/本机分区无障碍标签，即使历史缓存仍含模型聚合也不会展示模型明细。
- Claude 详情页的标题与官方额度固定可见，只有本机活动区域按可用高度滚动。
- 修复桌面已有普通应用窗口时，点击 Provider 后详情页仍落在窗口栈底部的问题；浮岛继续保持桌面层，临时详情改用标准 floating 层级，关闭后立即移除。
- 将浮岛与详情从系统浮动层降到桌面层，移除全屏辅助行为，使普通应用窗口和全屏空间可自然覆盖 AI Token Meter。
- 修复切换 Space 后详情继续残留的问题；Space 变化现在关闭详情、撤销交互焦点并保持用户已保存的屏幕、侧边和垂直位置。
- 修复深海背景在浮岛上下肩部出现黑色断层的问题；背景改为 `1.22×` 等比裁切，左右仅水平镜像，原始 PNG 保持不变。
- 将浮岛上下跨度过大的长 S 肩部收窄为“短平台 + 紧凑圆弧”，减少贴边处过度外鼓，同时保持左右镜像、主体拖动、Logo 点击和详情布局不变。
- 修复 DeepSeek 在缓存、超时或余额告警状态下被橙色等语义色覆盖、看不到原薄荷紫配色的问题；采集状态继续通过详情文字、状态符号和无障碍描述表达。
- 浮岛肩部增加长切线收直缓冲并移除突兀的黑色外投影，使上下接边在浅色壁纸上更圆滑。
- 将贴边浮岛上下的分段平台肩部替换为从屏幕边缘尖点展开的连续 S 曲线，左右严格镜像并保留完整玻璃拖动区域。
- 修复贴边浮岛的反向半圆肩部退化成方形、顶部多余短横，以及贴边边缘的视觉接缝。
- 修复浮岛只能从很小的顶部手柄拖动的问题；现在 Logo 以外的玻璃空白区域均可拖动，Logo 点击仍只打开详情。
- 修复固定 Left/Right 模式下拖动命中区与显示布局不一致的问题，并保留位置重启后的恢复行为。
- 修复菜单栏中的 Settings 入口不能稳定打开或置前既有设置窗口的问题。
- 修复应用启动时同步读取钥匙串导致浮窗迟迟不出现的问题；DeepSeek 密钥读取超过 2 秒会独立降级，不再阻塞 Claude 与 Codex 刷新。
- 修复 DeepSeek 内嵌官网登录页的手机号、验证码等输入框无法获得键盘焦点的问题；Claude、Codex 详情仍保持不抢占当前应用。
- 修复 DeepSeek 官网 2026-08 拆分的按 API Key 用量/费用响应无法自动合并，导致原生 30 天图表不更新的问题。
- 修复 Claude 登录有效但因工作区信任或命令未提交而持续超时的问题。
- 修复 Codex 进程超时时丢失根因的问题。
- 修复外部点击监听只覆盖部分 App 或事件坐标的问题。
- 修复旧自动隐藏任务关闭新详情、退出后任务未取消的问题。
- 修复 Claude 促销百分比被误识别成真实周用量的问题。
- 修复 Codex 模型专属 0% 覆盖通用周额度的问题。
- 修复 0% 仍显示进度弧以及文字和圆环使用不同额度窗口的问题。
- 修复浮岛窗口边缘残留透明空白、损坏位置产生无效坐标，以及已保存显示器断开后设置无法重新定位的问题。
- 修复键盘或 VoiceOver 阅读详情时自动隐藏打断操作，以及旧详情交互状态影响新详情计时的问题。

### Security

- DeepSeek 官网响应仅接受官方 HTTPS 来源、受限负载大小和可识别结构。
- Codex 重置额度不保存兑换 ID，也不提供自动兑换或“立即使用”。
- DeepSeek 业务缓存只保存标准化逐日聚合，不保存 Cookie、授权头、登录字段或网页原始响应。
- Claude/Codex 账户邮箱与套餐、DeepSeek Key 后四位仅保留在 Settings 的内存状态，不进入快照、Widget、通知、日志或登录脚本；登录脚本权限固定为 `0700` 且只含批准的官方命令。
- Claude 本机活动读取采用 JSON 白名单字段、流式读取、文件时间过滤、总量/文件数/持续时间边界和安全模型标识规范化，不保存或展示提示词、回复、项目路径、标题和分支；本机读取失败或超时不会改变官方额度结果。

## 0.1.0 - 2026-08-28

### Added

- 原生 macOS 菜单栏应用和右侧悬浮用量条。
- Claude Code CLI `/usage` 采集与当前/次级额度解析。
- Codex CLI `app-server` 结构化速率限制采集。
- DeepSeek 官方余额 API 支持。
- DeepSeek API Key 的 macOS Keychain 存储。
- 三服务统一快照、5 分钟刷新、手动刷新和本地缓存降级。
- 70% / 90% 用量通知。
- 登录时启动、显示/隐藏悬浮条和设置窗口。
- SwiftPM 测试、release 构建、App Bundle 组装和 ad-hoc 签名脚本。

### Security

- CLI 凭证仍由官方工具管理，AI Meter 不读取凭证文件。
- API Key 不进入偏好、缓存或日志。
- 错误、缓存与通知在展示或持久化前进行敏感文本清理。

> 当前仓库尚未创建 `v0.1.0` tag。远程地址和正式发布确定后，再为版本标题添加实际的比较与 Release 链接。
