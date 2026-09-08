# AI Token Meter 待完成需求与需求历史

**最后更新：** 2026-09-08
**用途：** 统一记录用户在开发过程中随时提出的碎片化需求，避免任务耗时较长或对话切换后遗漏。

## 使用规则

- 新需求先登记，再开始分析、设计或实现。
- 当前工作未结束时收到新需求：先登记，不自动打断当前工作；用户明确要求调整优先级时例外。
- 状态使用：`待处理`、`进行中`、`待用户确认`、`受环境限制`、`已延期`、`已完成`。
- 完成项不删除，补充完成日期、文档和 Git 证据。
- 每个开发阶段结束后重新读取本列表，并继续处理最高优先级的可执行事项。

## 当前队列

| ID | 类别 | 需求摘要 | 优先级 | 状态 | 登记日期 | 下一步/阻塞 | 证据 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| REQ-20260908-005 | Windows CI 门禁 | 原生 Windows 严格 Clippy 在新增初始化命令的 Windows 条件分支发现多余 return，阻止后续运行测试 | 高 | 已完成 | 2026-09-08 | 2026-09-08 完成：等价尾表达式、无 lint 忽略；原生严格 Clippy、229 项 Rust 与 NSIS 构建通过 | `d3c57f2` · [成功 CI](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/34185344676) · [开发日志](development/2026-09-08-windows-cli-post-login.md) |
| REQ-20260908-004 | 自定义 CLI 回归 | 最终审查发现手工选择官方 npm Codex 包装器时，会把内部 JS 启动目标保存为配置，后续发现却拒绝该目标 | 高 | 已完成 | 2026-09-08 | 2026-09-08 完成：保存用户包装器身份，执行目标独立；旧行为变异失败，保存→重载→再次发现原生回归通过；未放开任意 JS | `d3c57f2` · 定向复审无阻塞 · [成功 CI](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/34185344676) · [开发日志](development/2026-09-08-windows-cli-post-login.md) |
| REQ-20260908-003 | Windows 回归测试 | npm/Node 原生回归已执行入口，但 PATH 断言将 Windows 长路径与同一目录的 8.3 短路径直接按字符串比较而失败 | 高 | 已完成 | 2026-09-08 | 2026-09-08 完成：完整 PATH 规范化比较保留单一 Node 目录约束，原生真实进程、入口/参数/退出码断言通过 | `3955bb2` · [成功 Windows CI](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/34185344676) · [开发日志](development/2026-09-08-windows-cli-post-login.md) |
| REQ-20260908-002 | 发布测试回归 | 完整验证发现 appcast 测试固定要求旧版 0.2.2，更新源滚动移除旧项后失败；改为验证真实发布条目的版本、签名与下载路径契约 | 中 | 已完成 | 2026-09-08 | 2026-09-08 完成：逐条有效元数据通过，六种损坏负例拒绝；不改更新源，签名格式检查不冒充密码学验签 | `06b5fe4`、`d3c57f2` · [macOS CI](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/34185344720) · [开发日志](development/2026-09-08-windows-cli-post-login.md) |
| REQ-20260908-001 | Windows CLI 采集缺陷 | 用户已在终端登录 Claude Code 和 OpenAI Codex，但详情分别显示需要设置/未安装；Settings 已识别 Claude 原生 CLI 2.1.263 及账号，Codex 账户状态暂不可用。核对发现、登录、采集路径和错误展示不一致，恢复正常额度显示 | 高 | 受环境限制 | 2026-09-08 | 实现、独立复审及双平台 CI 已通过；本轮未发布，公开版仍为 0.5.0。真实 Windows/WSL 的用户账号额度及显式工作区初始化需在新版交付后验收；不重装、不重复登录、不自动信任 | `65d2b19`、`c9af3c3`、`b68a020`、`d3c57f2` · [PR #12](https://github.com/sljzdotcom/AI-Token-Meter/pull/12) · [调查/开发及验收边界](development/2026-09-08-windows-cli-post-login.md) |
| REQ-20260907-011 | 双平台更新发布 | 发布包含 CLI 安装/登录引导、Windows 页签/顶部 Logo 与双平台 About 社交链接的新稳定版，让各台机器通过应用内检查更新安装 | 高 | 已完成 | 2026-09-07 | 2026-09-07 完成：0.5.0/build13 已公开为 latest；双平台 CI、签名资产、匿名 SHA-256/签名和三个更新入口全部通过；不改变账号或设置，真机受限项独立保留 | [计划](design/implementation-plans/2026-09-07-v0.5.0-release.md) · [发布日志](development/2026-09-07-v0.5.0-release.md) · [Release](https://github.com/sljzdotcom/AI-Token-Meter/releases/tag/v0.5.0) · Tag `de117b0` · appcast `f76ea0f` · workflow `34093997980` |
| REQ-20260907-010 | About CI 回归 | 原生 macOS CI 的失败提示渲染断言未通过；定位真实视图更新与测试读取时序，保留提示断言和有界失败 | 高 | 已完成 | 2026-09-07 | 2026-09-07 完成：两秒有界等待实际渲染条件，原 URL/状态/提示断言保留；本机 432 项通过，原生 CI 两项 About 测试通过。该次既有 CLI 耗时断言失败单独归入 REQ-20260906-003，不冒充全套 CI 通过 | [失败 CI](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/34088536876) · [About 复验](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/34088933770) · `48cbd0f` · [日志](development/2026-09-07-about-branding.md) |
| REQ-20260907-008 | CLI 安装分发核对 | 确认 CLI/安装器是否内嵌软件包、是否增大安装体积；若内嵌大型文件则改为在线下载 | 中 | 已完成 | 2026-09-07 | 2026-09-07 核对完成：现有实现已在点击后在线获取官方安装脚本与 CLI，不内嵌大型 CLI，无需修改安装逻辑 | [核对记录](development/2026-09-07-about-branding.md) · CLI 代码合并 `266b1f3` · 规格 `a10481e` |
| REQ-20260907-009 | 关于与品牌展示 | macOS/Windows 关于页在 Miller 作者后增加 Twitter @MillerPanYue 与项目 GitHub 链接、小图标；Windows Settings 顶部名称旁增加现有软件 Logo，与 macOS 呼应 | 中 | 已完成 | 2026-09-07 | 2026-09-07 完成并合入 main：两端作者图标链接、Windows 本地 Logo；432 项 Swift、80 项前端、218 项原生 Windows Rust、双平台 CI 与构建通过。已随 0.5.0 发布（REQ-20260907-011） | [规格](design/specifications/2026-09-07-about-branding-design.md) · [计划](design/implementation-plans/2026-09-07-about-branding.md) · [日志](development/2026-09-07-about-branding.md) · [PR #11](https://github.com/sljzdotcom/AI-Token-Meter/pull/11) · 合并 `12ad5e2` |
| REQ-20260907-006 | CLI 安装与账户引导 | Settings 未安装 CLI 时显示变色安装按钮协助一键安装；已安装自动定位，未登录/过期以变色登录按钮提醒；已登录显示当前账号并提供重新登录或退出登录操作，macOS/Windows 同步 | 高 | 已完成 | 2026-09-07 | 2026-09-07 完成并合入 main：用户点击才运行官方安装器；状态变色、防重复、自动发现、已连接显示账号与重新登录；检测失败不误重装。真实上游安装/账号操作未执行，已随 0.5.0 发布（REQ-20260907-011） | [规格](design/specifications/2026-09-07-cli-onboarding-design.md) · [计划](design/implementation-plans/2026-09-07-cli-onboarding.md) · [开发与验证](development/2026-09-07-cli-onboarding.md) · `7fd6d45`、`8dfbeac`、`f6c4493` · [PR #10](https://github.com/sljzdotcom/AI-Token-Meter/pull/10) · 合并 `266b1f3` |
| REQ-20260907-007 | Windows Settings 页签图标 | Windows 设置的外观、监测、服务、关于等页签名称增加图标便于区分，保留中英文文字和系统默认字体 | 中 | 已完成 | 2026-09-07 | 2026-09-07 完成并合入 main：四个 16px 线性图标，中英文无截断、键盘切换/焦点循环通过；macOS 不变。真实 Windows 字形/DPI 仍属现场验收边界；已随 0.5.0 发布（REQ-20260907-011） | [规格](design/specifications/2026-09-07-cli-onboarding-design.md) · [计划](design/implementation-plans/2026-09-07-cli-onboarding.md) · [开发与验证](development/2026-09-07-cli-onboarding.md) · `43e418b` · [PR #10](https://github.com/sljzdotcom/AI-Token-Meter/pull/10) · 合并 `266b1f3` |
| REQ-20260907-005 | 双平台更新发布 | 用户要求直接发布当前多显示器、Windows 中英文/字体及详情字号改进，让现有 macOS 与 Windows 设备通过应用内更新获取新版 | 高 | 已完成 | 2026-09-07 | 2026-09-07 完成：0.4.0/build 12 公开且为 latest，双平台 CI/安装包、公网 SHA-256/两端签名及三个更新源全部验证通过；物理多屏/DPI 验收继续独立追踪 | [发布记录](development/2026-09-07-v0.4.0-release.md) · [Release](https://github.com/sljzdotcom/AI-Token-Meter/releases/tag/v0.4.0) · Tag `15d80e2` · appcast `2d2254f` · workflow `34076547278` |
| REQ-20260907-004 | Windows CI 稳定性 | 最终 Windows CI 的真实浏览器字号门禁在 15 秒后超时，需定位冷启动/执行/收尾边界，保留有界失败与进程清理，不以跳过门禁代替验证 | 高 | 已完成 | 2026-09-07 | 2026-09-07 完成门禁加固：有界阶段诊断、45 秒 Windows 浏览器预算、readiness/taskkill 清理时限和原错误保留；21 项回归及 Windows 原生 CI 全绿，无断言跳过/自动重试。旧超时确切阶段仍未知，不宣称已证实冷启动根因 | [失败 CI](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/34073429483)；验收：真实 Windows 门禁完整通过，卡死仍有界失败并清理进程 · `d303c11` · [成功 CI](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/34073917953) |
| REQ-20260907-003 | Windows 详情字号 | Windows 三个产品的所有详情页文字在当前字号基础上统一再减少 1 号；macOS 不变 | 中 | 已完成 | 2026-09-07 | 2026-09-07 完成：全角色减 1 CSS px，真实 Windows 浏览器 632 项计算样式通过；Settings、托盘菜单、浮动条及 macOS 不变。已随 0.4.0 发布到更新通道 | [规格](design/specifications/2026-09-07-windows-localization-design.md)；英文/中文及四种字体覆盖；不冒充 Windows 原生字体/DPI 验收 · `3312537` · [CI](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/34073917953) |
| REQ-20260907-001 | 多显示器与拖动 | M4 Max 接入主副两屏后浮动条启动在副屏且无法拖到主屏；修复跨屏拖动，提供明确显示器选择，并设计仅一屏/所有屏显示选择与断屏回退、位置记忆 | 高 | 已完成 | 2026-09-07 | 2026-09-07 完成代码验收：主屏/指定/所有屏、跨屏拖动和逐屏记忆，复审全部关闭；macOS 405+13、本机 Windows 57/197 及原生 CI/NSIS 通过。已随 0.4.0 发布；物理多屏补验归 REQ-20260901-004 | [调查与开发日志](development/2026-09-07-multidisplay-and-windows-localization.md)；关联 REQ-20260903-009，物理多屏验收仍独立追踪 · `255e8b3`、`a8d7a0f` · [PR #9](https://github.com/sljzdotcom/AI-Token-Meter/pull/9) |
| REQ-20260907-002 | Windows 本地化与字体 | Windows 增加简体中文/English语言选择（默认英文）；字体增加微软雅黑、黑体、楷体，默认微软雅黑；macOS 不增加语言选择 | 中 | 已完成 | 2026-09-07 | 2026-09-07 完成代码验收：English/简体中文即时同步，微软雅黑默认及黑体/楷体，旧偏好保留、缺失回退；Settings 系统字体和 macOS 不变。已随 0.4.0 发布；真实 Windows CI 通过；实际字形/DPI 仍需现场验收 | [规格](design/specifications/2026-09-07-windows-localization-design.md) · [计划](design/implementation-plans/2026-09-07-windows-localization.md)；安装成功不等于全部 Windows 专项验收通过 · `3312537`、`a8d7a0f` · [原生 CI](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/34073917953) |
| REQ-20260906-004 | 双平台更新发布 | 发布最新紧凑浮动条版本，让现有macOS与Windows机器通过应用内检查更新下载并安装；验证实际更新通道、签名、公开资产，不泄露密钥 | 高 | 已完成 | 2026-09-06 | 2026-09-06 完成；稳定 macOS、Windows stable 与旧 Preview 更新源均指向 0.3.0；旧 Mac Preview 实际检查发现新版。Windows 真机原位升级仍按既有验收项追踪 | [发布记录](development/2026-09-06-v0.3.0-release.md) · [Release](https://github.com/sljzdotcom/AI-Token-Meter/releases/tag/v0.3.0) · Tag `bb215c3` · appcast `f9a1f83` · [成功 workflow](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/34035797098) |
| REQ-20260901-001 | 服务认证 | Settings 始终显示 Claude、Codex 当前账户和登录按钮；支持官方 CLI 重新登录；DeepSeek 显示遮罩 Key，并安全替换 API Key | 高 | 已完成 | 2026-09-01 | 稳定签名发布后重录一次旧 DeepSeek Key，可解除 ad-hoc CDHash 变化造成的钥匙串访问限制 | [设计规格](design/specifications/2026-09-01-service-account-relogin-design.md)、[实施计划](design/implementation-plans/2026-09-01-service-account-relogin.md)、[开发与验收记录](development/2026-09-01-service-account-relogin.md)、`f95c6cf`–`bfc7412`、合并 `cd77e25` |
| REQ-20260901-002 | 项目治理 | 建立项目级“待完成需求”列表；以后每条新需求先登记，可分类、标记完成/待确认，并在当前任务结束后继续读取处理 | 高 | 已完成 | 2026-09-01 | 后续所有新需求继续遵循本机制 | 本文件、`AGENTS.md`、`641f74c` |
| REQ-20260901-003 | Widget | Apple Development 证书、Widget 安装、Gallery 与 Small/Medium/Large 真实桌面验收 | 中 | 已延期 | 2026-09-01 | 用户明确要求先放一放；取得证书且用户恢复该事项后继续 | [Widget 开发日志](development/2026-09-01-widgetkit-extension.md) |
| REQ-20260901-004 | 真实验收 | Mission Control、第二个普通 Space、真实指针拖动和多显示器补验 | 低 | 受环境限制 | 2026-09-01 | 需要可操作的系统界面、第二个普通 Space 或额外显示器 | [真实桌面验收](development/2026-09-01-real-desktop-acceptance.md) |
| REQ-20260901-005 | 窗口交互 | 桌面已有其他应用窗口时，点击浮动条 Provider 后弹出的详情应位于所有普通应用窗口上方，而不是落在窗口栈底部 | 高 | 已完成 | 2026-09-01 | 无 | [设计规格](design/specifications/2026-09-01-detail-panel-frontmost-design.md)、[实施计划](design/implementation-plans/2026-09-01-detail-panel-frontmost.md)、[开发与验收记录](development/2026-09-01-detail-panel-frontmost.md)、`5ad9ce6`、`4617184` |
| REQ-20260901-006 | Claude 详情 | 丰富 Claude 详情页，增加明确标记为本机口径的最近 30 天 Claude Code 统计 | 中 | 已完成 | 2026-09-01 | 无 | [设计规格](design/specifications/2026-09-01-claude-detail-local-activity-design.md)、[实施计划](design/implementation-plans/2026-09-01-claude-detail-local-activity.md)、[开发与验收记录](development/2026-09-01-claude-detail-local-activity.md)、`dca5c1d`–`3303ce1`、合并 `6298205` |
| REQ-20260901-007 | 字体 | 在显示字体选择器中增加 Alimama FangYuanTi VF、Fira Code、Leigo、Menlo、Alimama DaoLiTi，并保持 Settings 自身永远使用系统字体 | 中 | 已完成 | 2026-09-01 | 未安装字体的真实字形对比待用户安装对应字体后按需进行，不影响目录功能完成 | [设计规格](design/specifications/2026-09-01-display-font-catalog-expansion-design.md)、[实施计划](design/implementation-plans/2026-09-01-display-font-catalog-expansion.md)、[开发与验收记录](development/2026-09-01-display-font-catalog-expansion.md)、`ff051c3`、合并 `6298205` |
| REQ-20260901-008 | Claude 详情 | Claude 详情页不再显示 Token composition 与 Top models 两张卡片 | 中 | 已完成 | 2026-09-01 | 2026-09-02 完成；294 项测试、Release 安装与实机自动隐藏验收通过 | [设计规格](design/specifications/2026-09-02-claude-detail-card-removal-design.md) · [实施计划](design/implementation-plans/2026-09-02-claude-detail-card-removal.md) · [验收日志](development/2026-09-02-claude-detail-card-removal.md) |
| REQ-20260902-009 | Claude 详情 | 移除本机活动区域底部的隐私说明文字及锁形图标 | 中 | 已完成 | 2026-09-02 | 2026-09-02 完成；295 项测试、Release 安装与真实辅助功能树验收通过 | [设计规格](design/specifications/2026-09-02-claude-detail-privacy-note-removal-design.md) · [实施计划](design/implementation-plans/2026-09-02-claude-detail-privacy-note-removal.md) · [验收日志](development/2026-09-02-claude-detail-privacy-note-removal.md) |
| REQ-20260902-010 | 菜单栏视觉 | 重新设计菜单栏状态图标，使其更现代、更具极客感，并保持 macOS 菜单栏小尺寸下清晰可辨 | 中 | 已完成 | 2026-09-02 | 2026-09-02 完成；299 项测试、双外观像素渲染、Release 安装哈希与真实刷新链路验收通过 | [设计规格](design/specifications/2026-09-02-menu-bar-quantum-dial-design.md) · [实施计划](design/implementation-plans/2026-09-02-menu-bar-quantum-dial.md) · [开发与验收记录](development/2026-09-02-menu-bar-quantum-dial.md) · `b67821e`–`e93223f` |
| REQ-20260902-011 | 服务命名 | 将所有当前用户界面中的 Claude 统一显示为 Claude Code，将 Codex 统一显示为 OpenAI Codex | 中 | 已完成 | 2026-09-02 | 2026-09-02 完成；304 项测试、60 个测试组、Release/安装哈希、真实辅助功能与 Settings 验收通过；Widget 桌面安装继续由既有延期证书事项管理 | [设计规格](design/specifications/2026-09-02-provider-visible-name-standardization-design.md) · [实施计划](design/implementation-plans/2026-09-02-provider-visible-name-standardization.md) · [开发与验收记录](development/2026-09-02-provider-visible-name-standardization.md) · `e09aa8c`–`5cb3ab8` |
| REQ-20260902-012 | 菜单栏缺陷 | 已安装应用的菜单栏状态图标不可见或对比度不足 | 高 | 已完成 | 2026-09-02 | 2026-09-02 完成；用户确认真实菜单栏可见，分支及合并后 `main` 均通过 305 项测试/60 个测试组 | [开发与验收记录](development/2026-09-02-menu-bar-icon-visibility.md) · `3d5bd87`–`b85a04e` |
| REQ-20260902-013 | 项目治理 | 对整个项目进行完整复盘，补齐架构、功能、数据口径、构建发布、测试、运维、决策与遗留事项文档，并安全删除已证明无用的文件 | 高 | 已完成 | 2026-09-02 | 无 | [设计规格](design/specifications/2026-09-02-project-retrospective-and-documentation-governance-design.md)、[实施计划](design/implementation-plans/2026-09-02-project-retrospective-and-documentation-governance.md)、[复盘报告](development/2026-09-02-project-retrospective.md)、`b309ea7`–`685746b` |
| REQ-20260902-014 | 发布交付 | 从当前 `main` 生成可拷贝到 MacBook Pro M4 Max 使用的 Apple Silicon Release ZIP，并提供完整性校验和与安装说明 | 高 | 已完成 | 2026-09-02 | `0.1.0` 包后续确认不可跨 Mac 启动，已被 `REQ-20260902-015` 的 `0.1.1` 修复包替代 | [失败包历史记录](development/2026-09-02-macbook-arm64-package.md)、源码基线 `7a93c66`、旧 ZIP SHA-256 `262f13f9…d91783` |
| REQ-20260902-015 | 发布缺陷 | 修复 MacBook 分发包启动时因 SwiftPM 资源包无法加载而在 `NSBundle.module → FloatingStripView.body` 崩溃，重新生成可迁移验证的修复版 | 高 | 已完成 | 2026-09-02 | 无 | [修复与 0.1.1 交付记录](development/2026-09-02-portable-resource-crash-fix.md)、`8cb8a8b`、`1fbc1dd`、`dd10409`、ZIP SHA-256 `1b2cf19b…9fa72` |
| REQ-20260902-016 | 服务发现 | 修复 M4 Max 上 OpenAI Codex 显示 `CLI not installed`；区分 CLI 确实缺失与 GUI App 启动环境找不到用户安装路径，并提供可操作恢复入口 | 高 | 待用户确认 | 2026-09-02 | 安装 `0.1.2` 后在 M4 Max 完整退出/重开并点击 Check Status，确认 nvm `0.148.0` 的账户、额度和详情恢复 | [设计规格](design/specifications/2026-09-02-codex-cli-discovery-design.md)、[实施计划](design/implementation-plans/2026-09-02-codex-cli-discovery.md)、[开发与交付记录](development/2026-09-02-codex-cli-discovery.md)、`6fe2382`、`d7b4467`、`55b5251`、ZIP SHA-256 `a2c76017…d143f` |
| REQ-20260902-017 | 公开发布 | 将项目安全发布到用户 GitHub 账户：建立公开仓库，补齐标准开源文档和产品截图，提供可下载 Release，确保源码、历史和产物不含个人 Key，并在 About 中标注作者 Miller | 高 | 已完成 | 2026-09-02 | 无 | [设计规格](design/specifications/2026-09-02-public-github-release-design.md)、[实施计划](design/implementation-plans/2026-09-02-public-github-release.md)、[发布日志](development/2026-09-02-public-github-release.md)、[v0.1.2 Release](https://github.com/sljzdotcom/AI-Token-Meter/releases/tag/v0.1.2)、[最终 CI](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/33642220151)、`24ab4a5` |
| REQ-20260902-018 | 发布缺陷 | 修复 GitHub Actions 在 macOS 15 runner 上编译 Claude 本机活动 token 数组时的 Swift 类型推断失败，恢复公开仓库 CI | 高 | 已完成 | 2026-09-02 | 无 | [首次失败 CI](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/33634543141)、[中间成功 CI](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/33637095658)、[复验失败 CI](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/33637436534)、[最终成功 CI](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/33638241625)、`616ce76` |
| REQ-20260902-019 | 应用更新 | 在 Settings 增加“检查更新”和“立即更新”；从 GitHub 发现高于当前版本的新 Release 后，可安全自动下载、校验、替换应用并重新启动 | 高 | 已完成 | 2026-09-02 | 无 | [设计规格](design/specifications/2026-09-02-github-app-update-design.md) · [实施计划](design/implementation-plans/2026-09-02-github-app-update.md) · [开发与验收记录](development/2026-09-02-github-app-update.md) · [v0.2.2 窗口终验](development/2026-09-03-update-status-window-frontmost.md) · [`v0.2.2` Release](https://github.com/sljzdotcom/AI-Token-Meter/releases/tag/v0.2.2) |
| REQ-20260902-020 | 运行稳定性 | 修复 macOS 并发分配 PTY 时 `openpty` 偶发失败，导致并发 CLI 命令误报 `transportFailure` 和发布回归不稳定 | 高 | 已完成 | 2026-09-02 | 无 | [PTY 分配竞态日志](development/2026-09-03-pty-allocation-race.md) · `d6cbe76` · 361 项测试 · [最终 CI](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/33655946917) |
| REQ-20260903-001 | 发布缺陷 | 修复 GitHub macOS runner 高负载下 PTY 父进程退出等待超时及并发输出尾部丢失，恢复公开发布 CI | 高 | 已完成 | 2026-09-03 | 无 | [开发记录](development/2026-09-03-ci-pty-exit-race.md) · `2c6a194` · [macOS main CI 33745691851](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/33745691851) · [Windows main CI 33745691724](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/33745691724) |
| REQ-20260903-002 | 更新交互缺陷 | 下载完成后的 Sparkle “Install and Relaunch”窗口被 Settings 遮挡，导致界面看似长期停在 Preparing；安装流程窗口应自动置前 | 高 | 已完成 | 2026-09-03 | 无 | [设计规格](design/specifications/2026-09-03-update-status-window-frontmost-design.md) · [实施计划](design/implementation-plans/2026-09-03-update-status-window-frontmost.md) · [真实升级终验](development/2026-09-03-update-status-window-frontmost.md) · `6057b3e` · [`v0.2.2`](https://github.com/sljzdotcom/AI-Token-Meter/releases/tag/v0.2.2) |
| REQ-20260903-003 | 测试稳定性 | 修复 GitHub Actions 高负载下详情自动隐藏测试依赖固定 50ms 等待、偶发尚未收到异步超时回调而失败的问题 | 高 | 已完成 | 2026-09-03 | 无 | `f14f14a` · [失败 CI 33700918921](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/33700918921) · [PR CI 33701250078](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/33701250078) · [main CI 33702415007](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/33702415007) |
| REQ-20260903-004 | 跨平台 | 制作功能对等的 Windows 版本；此后产品功能、版本号、文档、测试和 GitHub Release 原则上保持 macOS 与 Windows 同步 | 高 | 待用户确认 | 2026-09-03 | 双平台代码、独立复审、main CI、Tauri Secret、`preview.0` 至 `preview.3` 同版本签名 Release 和固定 Preview 更新源均已完成；请在交互式 Windows 11 x64 会话完成视觉/DPI/拖动/全屏/三服务/DeepSeek 官网与 `preview.2 → preview.3` 原位更新验收，并补产品截图 | [设计规格](design/specifications/2026-09-03-windows-platform-design.md)、[实施计划](design/implementation-plans/2026-09-03-windows-platform.md)、[开发日志](development/2026-09-03-windows-platform.md)、[v0.3.0-preview.3](https://github.com/sljzdotcom/AI-Token-Meter/releases/tag/v0.3.0-preview.3)、[成功 workflow](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/33887131319) |
| REQ-20260903-005 | 文档门禁缺陷 | 新增 Windows npm/Rust 依赖后，文档和秘密检查器错误扫描 `node_modules`/`target` 内第三方文件并报告无关坏链或二进制假阳性 | 高 | 已完成 | 2026-09-03 | 无 | [Windows 开发日志](development/2026-09-03-windows-platform.md)、回归测试与 Windows 骨架检查点 |
| REQ-20260903-006 | 发布文档缺陷 | Windows 实施计划将仓库目录作为公开发布脚本的位置参数传入，脚本会误把目录当成 Release ZIP 并报告归档不存在 | 中 | 已完成 | 2026-09-03 | 无 | [Windows 实施计划](design/implementation-plans/2026-09-03-windows-platform.md)、[Windows 开发日志](development/2026-09-03-windows-platform.md)、文档门禁回归测试 |
| REQ-20260903-007 | 共享合同门禁缺陷 | Task 5 计划新增 Windows CLI 位置 fixture，但门禁把 `contracts/fixtures` 中全部 JSON 都当作四份用量快照，按计划新增必然失败 | 高 | 已完成 | 2026-09-03 | 无 | [辅助 CLI fixture](../contracts/fixtures/auxiliary/windows-cli-locations.json)、[Windows 实施计划](design/implementation-plans/2026-09-03-windows-platform.md)、[Windows 开发日志](development/2026-09-03-windows-platform.md)、`d481a44`、合同门禁仍确认直接目录恰好 4 份用量快照 |
| REQ-20260903-008 | macOS CI 稳定性 | 修复高并发 CI 中 DeepSeek 即时 Keychain/SecretStore 读取被低优先级任务饿死、错误等待满 2 秒并报告超时或钥匙串失败的问题 | 高 | 已完成 | 2026-09-03 | 无 | [开发日志](development/2026-09-03-deepseek-secret-read-priority.md) · `eea044b` · `2c6a194` · [macOS main CI 33745691851](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/33745691851) · [Windows main CI 33745691724](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/33745691724) |
| REQ-20260903-009 | 浮动条位置缺陷 | 修复浮动条重启或运行一段时间后丢失用户摆放位置、从左侧自行回到右侧或从主屏跳到副屏的问题；持久保存具体显示器、贴边侧和垂直位置，多屏变单屏时临时回到当前主屏 | 高 | 已完成 | 2026-09-03 | 无；真实物理拔插验收继续由 `REQ-20260901-004` 记录，不影响实现交付状态 | [设计规格](design/specifications/2026-09-03-floating-strip-placement-persistence-design.md) · [实施计划](design/implementation-plans/2026-09-03-floating-strip-placement-persistence.md) · [开发日志](development/2026-09-03-floating-strip-placement-persistence.md) · [PR #4](https://github.com/sljzdotcom/AI-Token-Meter/pull/4) · 合并 `c2d2e64` · [macOS main CI 33766955625](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/33766955625) · [Windows main CI 33766955622](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/33766955622) |
| REQ-20260903-010 | macOS CI / DeepSeek 韧性 | 修复高并发下阻塞 Keychain 读取使超时任务被饿死、读取结束后错误进入 DeepSeek 网络请求并返回 `transportFailure` 的竞态 | 高 | 已完成 | 2026-09-03 | 无 | [开发日志](development/2026-09-03-deepseek-timeout-starvation.md) · `0ae1cbe` · [macOS PR CI 33766095915](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/33766095915) · [macOS main CI 33766955625](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/33766955625) |
| REQ-20260903-011 | 发布交付 | 将已合入 `main` 的稳定显示器位置修复及当前跨平台功能打包为下一版可安装 Preview；macOS 与 Windows 使用同一版本和 GitHub Release，提供校验、更新清单及标准发布文档 | 高 | 已完成 | 2026-09-03 | 无；Windows 真机交互与 `preview.0 → preview.1` 升级闭环继续由 `REQ-20260903-004` 管理 | [Release v0.3.0-preview.0](https://github.com/sljzdotcom/AI-Token-Meter/releases/tag/v0.3.0-preview.0) · [成功 workflow](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/33816010376) · [发布记录](development/2026-09-04-v0.3.0-preview.0-release.md) |
| REQ-20260904-001 | 发布门禁缺陷 | 修复跨平台合同检查器在 Windows runner 上把 Git 已跟踪为可执行的发布入口误判为不可执行，导致双平台 Preview 发布在代码测试通过后被阻断 | 高 | 已完成 | 2026-09-04 | 无 | [失败 workflow](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/33813010911) · [Windows 复验](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/33814393456) · `scripts/test-cross-platform-contracts.sh` |
| REQ-20260904-002 | Windows 发布缺陷 | 修复 Tauri 已成功生成签名 NSIS 资产后，Release workflow 在资产标准化与哈希阶段无法识别实际输出而中止的问题 | 高 | 已完成 | 2026-09-04 | 无 | [失败 workflow](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/33814393456) · [成功 workflow](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/33816010376) · `704342e` · `scripts/test-normalize-windows-release-assets.sh` |
| REQ-20260904-003 | macOS 更新通道 | 核对 M4 Max 手动检查更新时未发现 `0.3.0-preview.0` 的原因，并明确稳定版与预览版的安装入口 | 中 | 已完成 | 2026-09-04 | 无；当前 macOS 应用只读取稳定 `appcast.xml`，预览版需从 GitHub Release 手动安装，待未来明确提出后再设计可选 Preview 通道 | `SUFeedURL`、稳定 `appcast.xml`、[v0.3.0-preview.0](https://github.com/sljzdotcom/AI-Token-Meter/releases/tag/v0.3.0-preview.0) |
| REQ-20260904-004 | Windows 浮动条缺陷 | 修复 Windows 真机浮动条外侧白框和上下白条、轮廓锯齿/凹凸、反向半圆缺失以及靠边位置不稳定；视觉应与已确认的 macOS 轮廓和背景连续性一致 | 高 | 待用户确认 | 2026-09-04 | 用户安装 `0.3.0-preview.1` 后，在 Windows 11 真机确认 100%/125%/150%/200% DPI 边缘、左右肩部、拖动、多屏位置和 `preview.0 → preview.1` 原位升级 | [设计规格](design/specifications/2026-09-04-windows-floating-strip-parity-fix-design.md) · [实施计划](design/implementation-plans/2026-09-04-windows-floating-strip-parity-fix.md) · [开发日志](development/2026-09-04-windows-floating-strip-parity-fix.md) · `b9743c0` · `2614186` · `e78ad48` · `9213fd6` · [成功 workflow 33826484923](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/33826484923) · [公开 Release](https://github.com/sljzdotcom/AI-Token-Meter/releases/tag/v0.3.0-preview.1) |
| REQ-20260904-005 | Windows 启动窗口缺陷 | Windows 真机启动 AI Token Meter 时自动出现标题为 “AI Token Meter” 的巨大空白 Windows Terminal 窗口；该窗口并非点击 Provider 详情产生，启动后只应显示桌面浮动条和系统托盘 | 高 | 已完成 | 2026-09-04 | 无 | [设计规格](design/specifications/2026-09-04-windows-console-window-suppression-design.md) · [实施计划](design/implementation-plans/2026-09-04-windows-console-window-suppression.md) · [开发与发布记录](development/2026-09-04-windows-console-window-suppression.md) · [v0.3.0-preview.2](https://github.com/sljzdotcom/AI-Token-Meter/releases/tag/v0.3.0-preview.2) · [workflow 33833843964](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/33833843964) · 合并/Tag `c3cba89` · Windows 11 真机安装启动确认 |
| REQ-20260904-006 | Windows DeepSeek 与界面缺陷 | Windows 点击 DeepSeek 后除详情页外还出现无法关闭的空白独立窗口；“Sync official history”无响应；Windows 三个 Provider 详情页和 Settings 整体字号明显过大，尤其详情标题；Settings 字体选择下拉框白底白字、只有悬停选项才可辨识。应在不改变 macOS 样式的前提下恢复可关闭、可同步、紧凑且清晰的 Windows 体验 | 高 | 待用户确认 | 2026-09-04 | 代码、独立复审、双平台 CI、`main` 合并及 `0.3.0-preview.3` 公开交付已完成；请在 Windows 11 真机安装/更新该版本后确认登录、同步、关闭、复用聚焦、30 日聚合、紧凑字号与原生字体下拉 | [原设计规格](design/specifications/2026-09-04-windows-deepseek-history-and-density-design.md) · [最终门禁修复规格](design/specifications/2026-09-04-windows-deepseek-final-quality-gate-design.md) · [最终门禁修复计划](design/implementation-plans/2026-09-04-windows-deepseek-final-quality-gate.md) · [开发记录](development/2026-09-04-windows-deepseek-history-and-density.md) · [v0.3.0-preview.3](https://github.com/sljzdotcom/AI-Token-Meter/releases/tag/v0.3.0-preview.3) · [PR #6](https://github.com/sljzdotcom/AI-Token-Meter/pull/6) · 合并 `e62193c` · [Release workflow 33887131319](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/33887131319) |
| REQ-20260904-007 | 更新交付确认 | 核对其他机器能否通过应用内更新直接取得刚合入 `main` 的 Windows DeepSeek、窗口生命周期和紧凑界面修复，并明确稳定版与 Preview 的实际可更新边界 | 中 | 已完成 | 2026-09-04 | 无；Windows Preview 机器现可检查并安装 `0.3.0-preview.3`；macOS 稳定通道仍保持 `0.2.2`，macOS Preview 需手动下载 | 稳定 `appcast.xml` 当前为 `0.2.2` · 最新公开 Preview/Tag 为 [`0.3.0-preview.3`](https://github.com/sljzdotcom/AI-Token-Meter/releases/tag/v0.3.0-preview.3) · [固定 Windows Preview feed](https://github.com/sljzdotcom/AI-Token-Meter/releases/tag/windows-preview-feed) |
| REQ-20260904-008 | 双平台 Preview 发布 | 将当前 `main` 中已完成的 Windows DeepSeek、窗口生命周期与紧凑界面修复发布为新的 macOS/Windows 同版本 Preview；提供可下载 Release、SHA-256、签名更新清单，并验证其他机器可发现新版 | 高 | 已完成 | 2026-09-04 | 无；Windows 11 的真实交互验收继续保留在 `REQ-20260904-006`，macOS Preview 按既定边界手动安装 | [Release notes](releases/v0.3.0-preview.3.md) · [发布记录](development/2026-09-04-v0.3.0-preview.3-release.md) · 发布/Tag `dac10b9` · [Release workflow 33887131319](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/33887131319) · [公开 Release](https://github.com/sljzdotcom/AI-Token-Meter/releases/tag/v0.3.0-preview.3) · [固定 Windows Preview feed](https://github.com/sljzdotcom/AI-Token-Meter/releases/tag/windows-preview-feed) |
| REQ-20260906-001 | 竞品研究与视觉方向 | 调研 CodeNotch 可供 AI Token Meter 借鉴的视觉密度、信息层级、交互和功能，重点解释其“更瘦但仍清晰”的原因，并提出适合本项目的优先级建议 | 中 | 已完成 | 2026-09-06 | 无；本轮仅完成研究与建议，不代表紧凑模式、闲置折叠或新功能已经开发 | [CodeNotch](https://github.com/vinzdg/codenotch) · [竞品研究](development/2026-09-06-codenotch-competitive-review.md) · Git `ef84950` |
| REQ-20260906-002 | 紧凑浮动条与渐进式交互 | 按 CodeNotch 研究中的推荐方向改进 AI Token Meter：双平台增加 Compact/Comfortable 密度并默认使用 Compact、保留深海背景和 Logo-only；增加可选闲置折叠、可验证的 Refreshing/Waiting/Idle 双轨状态、Provider 显示与排序、统一数据可信度/新鲜度、持久化刷新退避和浮动条右键快捷菜单 | 高 | 受环境限制 | 2026-09-06 | 代码、双平台CI、构建与文档已完成，PR #7 已合入main，已随 0.3.0 发布。剩余Windows11多DPI/读屏/指针/多屏真实交互需对应设备验收 | [竞品研究](development/2026-09-06-codenotch-competitive-review.md) · [Compact/折叠规格](design/specifications/2026-09-06-compact-floating-strip-and-idle-fold-design.md) · [Provider/菜单规格](design/specifications/2026-09-06-floating-strip-provider-controls-design.md) · [状态/退避规格](design/specifications/2026-09-06-provider-state-and-refresh-resilience-design.md) · 规格 Git `7c85d5e` · macOS检查点 `9c30b45` · 合并 `c67112e` · [开发记录](development/2026-09-06-compact-progressive-strip.md) |
| REQ-20260906-003 | CI可靠性 | 追踪偶发的macOS PTY父进程退出超时/并发输出缺失、CLI超时测试PID读取失败及Windows ConPTY采集超时；不删除断言或跳过测试 | 低 | 受环境限制 | 2026-09-06 | 2026-09-07 同类父进程退出超时再次出现，已在 83c54c6 补白名单测试时序诊断，保留原两秒截止与全部断言；本机 PTY 13/13、全量 405+13 通过。尚未稳定复现确切原因，后续以新诊断定位，不能标为根因已修复 | [开发记录](development/2026-09-06-compact-progressive-strip.md) · 原失败CI `34033000263`、`34033000265` · [本次](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/34073917970) · [本轮诊断记录](development/2026-09-07-multidisplay-and-windows-localization.md) |

## 分类索引

### 服务认证

- `REQ-20260907-006`：按未安装/待登录/已连接状态提供官方 CLI 安装与账户恢复引导。
- `REQ-20260901-001`：账户显示、Claude/Codex 重新登录、DeepSeek 安全换 Key。
- `REQ-20260902-016`：M4 Max 上 OpenAI Codex CLI 的可靠发现、缺失诊断与恢复入口。

### 项目治理

- `REQ-20260901-002`：需求登记与持续处理机制。
- `REQ-20260902-013`：全项目复盘、文档补全、无用文件清理与长期知识留存。

### 竞品研究与视觉方向

- `REQ-20260906-001`：对照 CodeNotch，提炼可迁移的紧凑视觉、交互和功能方向。
- `REQ-20260906-002`：把确认后的紧凑密度、可选折叠、状态表达、Provider 管理和快捷交互同步落地到 macOS/Windows。

### Widget

- `REQ-20260901-003`：开发证书与真实 Widget 验收，当前延期。

### 真实验收

- `REQ-20260901-004`：需要额外系统或硬件条件的补验矩阵。

### 窗口交互

- `REQ-20260901-005`：点击浮动条后，详情窗口临时置于所有普通应用窗口上方。

### Claude 详情

- `REQ-20260901-006`：评估 Claude 详情页增加近 30 天用量与本机活动统计。
- `REQ-20260901-008`：移除 Token composition 与 Top models 两张本机活动卡片。
- `REQ-20260902-009`：移除本机活动区域底部的隐私说明及锁形图标。

### 字体

- `REQ-20260901-007`：扩充显示字体选择器，新增五个用户指定字体。

### 菜单栏视觉

- `REQ-20260902-010`：重新设计菜单栏状态图标，提升现代感、极客感与小尺寸辨识度。
- `REQ-20260902-012`：修复已安装应用菜单栏状态图标不可见或对比度不足。

### 服务命名

- `REQ-20260902-011`：统一 Claude Code 与 OpenAI Codex 的当前用户可见名称。

### 发布交付

- `REQ-20260907-005`：0.4.0 多显示器与 Windows 本地化双平台稳定更新交付。
- `REQ-20260902-014`：生成供 MacBook Pro M4 Max 使用的 Apple Silicon Release 分发包。
- `REQ-20260902-015`：修复跨 Mac 分发包的 SwiftPM 资源包启动崩溃并重新交付。
- `REQ-20260902-017`：安全发布公开 GitHub 仓库、标准文档、截图与可下载 Release，并补充作者信息。
- `REQ-20260902-018`：修复公开仓库首次 GitHub Actions 的 Swift 跨工具链类型推断失败。
- `REQ-20260902-019`：Settings 内检查 GitHub 新版本并安全自动下载安装。
- `REQ-20260902-020`：串行化 macOS PTY 分配，消除并发 CLI 采集的瞬时 `openpty` 失败。
- `REQ-20260903-001`：消除 GitHub macOS runner 高负载下 PTY 退出与尾部读取竞态。
- `REQ-20260903-002`：让 Sparkle 安装状态窗口从 Settings 启动时自动置前。
- `REQ-20260903-003`：移除详情自动隐藏回归测试中的固定墙钟假设。
- `REQ-20260903-011`：把稳定显示器位置与当前跨平台能力交付为 macOS/Windows 同版本 Preview Release。

### 跨平台

- `REQ-20260903-004`：新增 Windows 桌面版本，并建立 macOS/Windows 同版本、同功能、同 Release 的同步发布规则。

### 文档门禁

- `REQ-20260903-005`：文档检查器必须忽略 npm 第三方依赖目录，只审计项目维护的 Markdown。
- `REQ-20260903-006`：发布安全脚本的仓库检查示例必须使用具名 `--repository` 参数，不能把目录误当归档。
- `REQ-20260903-007`：共享 fixture 门禁须允许经过声明的非用量辅助 fixture，不能与四份快照数量约束冲突。

### macOS CI 稳定性

- `REQ-20260903-008`：避免 DeepSeek 凭据读取任务在并发 CI 中发生优先级反转并误报超时。
- `REQ-20260903-010`：让 DeepSeek Keychain 读取超时不受高并发任务饥饿影响，迟到凭据不得触发网络请求。

### 浮动条位置

- `REQ-20260903-009`：持久保存并稳定恢复浮动条的具体显示器、左右贴边和垂直位置；禁止主屏/副屏间自行漂移，多屏变单屏时临时回到当前主屏。

### Windows 详情与官方历史

- `REQ-20260907-007`：Windows 设置四个页签增加可辨识图标，保留文字与系统字体。
- `REQ-20260904-006`：修复 DeepSeek 官方历史空白且无法关闭、同步无响应，统一缩小 Windows 详情与 Settings 字号，并恢复字体下拉框的可读配色。

### 更新与发布状态

- `REQ-20260906-004`：0.3.0 双平台公开资产与稳定/旧 Preview 更新源同步，旧版可在应用内发现新版。

### 多显示器与 Windows 本地化

- `REQ-20260907-001`：修复 Mac 跨屏拖动，设计明确屏幕选择及可选所有显示器模式。
- `REQ-20260907-002`：Windows 英文/简体中文、微软雅黑/黑体/楷体；macOS 不增加语言选择。
- `REQ-20260907-003`：Windows 三服务详情全角色字号统一减少 1 CSS px，Settings、浮动条和 macOS 不变。
- `REQ-20260907-004`：真实浏览器门禁的阶段诊断与有界超时/清理加固，不跳过字号断言。
- `REQ-20260904-007`：区分源码已合并、公开 Release 已发布和应用内更新源已更新，避免其他机器误以为能立即取得尚未发布的修复。
- `REQ-20260904-008`：把 `main` 中尚未交付的修复制作为下一版双平台 Preview，并验证公开下载和应用内更新链路。

## 状态变更记录

| 日期 | ID | 变化 | 说明 |
| --- | --- | --- | --- |
| 2026-09-08 | REQ-20260908-001 | 进行中 | 任务 3 代码与本机回归完成：Claude 原生/WSL 认证、采集和显式初始化共用应用专用工作区；仅显式检查解除可恢复阻塞并触发单服务额度重试，限流等待保留。完整 Rust、85 项前端、构建、严格 Clippy、格式和 180 份文档门禁通过；等待原生 Windows CI，整体需求继续保持进行中。证据：Task 3 报告及提交 `feat(windows): isolate Claude usage workspace`。 |
| 2026-09-08 | REQ-20260908-001 | 进行中 | 任务 3 开始：以失败先行回归锁定 Windows Claude 采集与显式初始化共用应用专用空工作区、原生/WSL 安全启动、初始化反馈与仅手动检查重试额度；不修改凭据、不自动信任、不重装、不改 macOS 生产代码。 |
| 2026-09-08 | REQ-20260908-001 | 进行中 | 任务 1 开始：以失败先行回归锁定 Windows 全局 npm Codex 包装器与独立 Node 目录的显式入口恢复；保留受限环境白名单，不修改账户或采集业务。 |
| 2026-09-08 | REQ-20260908-001 | 进行中 | 任务 1 代码与本机回归完成：官方 npm 身份/入口、独立 Node、空格路径和缺失/错误身份边界已覆盖，33 项定向 Rust 回归与严格 Clippy 通过。Windows-only 真实进程用例已添加，等待原生 Windows runner 执行；整体需求继续保持进行中。 |
| 2026-09-07 | REQ-20260907-011 | 已完成 | 0.5.0/build13 已公开为 latest，CLI 引导与 About 品牌需求一并交付；本机/标签 macOS 432 项、Windows 原生 Rust 218 项及完整双平台发布通过。匿名公开资产哈希、两端签名和三个更新源全部验证，Tag `de117b0`、appcast `f76ea0f`、workflow `34093997980`。README/指南/版本历史同步。阶段末当前队列无其他进行中/待处理项；受限、延期与待用户确认项原样保留。 |
| 2026-09-07 | REQ-20260906-003 | 受环境限制 | 发布前主 Windows CI `34089957031` 正常父进程三秒预算超时、本机 Codex 测试函数级 transportFailure 均保留证据；后者符合 fixture PID 未就绪窗口，不能写成 timedOut 断言不匹配。定向 15 项及一次原样完整本机复验通过，0.5.0 标签双平台完整门禁全部通过；未放宽/跳过断言，不因此认定根因已修复。详见本版发布日志。 |
| 2026-09-07 | REQ-20260907-008 / 009 / 010 | 已完成 | 在线分发核对无需修改；About 与 Windows Logo 完成；异步视图测试等待修正并通过原生 CI。PR #11 合并 12ad5e2，合并树与验证头 48cbd0f 一致。macOS CI 34088933770 复验通过、Windows CI 34088933785 通过，218 项原生 Rust、NSIS 与 GUI 子系统成功；完整文档和日志同步。未发布新安装包，既有受限/延期/待用户确认项保留，阶段末无其他可直接继续的进行中/待处理项。 |
| 2026-09-07 | REQ-20260906-003 | 受环境限制 | 原生 CI `34088933770` 中既有 codexEarlyTimeoutIsReplayed 一秒耗时断言得到 1.006975s；timedOut/PID/进程退出没有报告错误，相关代码本轮未改。记录一次同提交完整复验，保留原断言，不因后续通过关闭历史时序根因。新 About 两项测试在该轮均通过，独立记录于 REQ-20260907-010。 |
| 2026-09-07 | REQ-20260907-006 / 007 | 进行中 → 已完成 | 最终复审全部阻断已关闭；本机 macOS 416+13、前端 75、Rust 206、密度 21/632 通过；Windows 原生 CI Rust 216、NSIS 和 GUI 子系统通过，macOS CI 通过。PR #10 合并 `266b1f3`，合并树与验证头 `84d1955` 完全一致。文档、需求、计划和证据同步；公开版本仍为 0.4.0，本轮未发布。阶段末无其他可直接继续的进行中/待处理项，既有延期、受环境限制和待用户确认事项保留。 |
| 2026-09-07 | REQ-20260907-006 | 进行中 | 最终整分支审查发现 macOS 不可执行既有文件仍可能被当缺失，以及 Windows Auto 的坏 Native 会提前阻断可用 WSL 回退；登记为同一安装/发现边界修复，关闭并通过原生 CI 后再合并。 |
| 2026-09-07 | REQ-20260907-006 | 进行中 | 初版 `7fd6d45` 经控制者独立 415+13 项与 Release 构建通过。首轮审查要求修复 Windows 安装环境（OS/System32）和健康检查失败被误当缺失的边界，修复后复审再继续页签图标；未发布。 |
| 2026-09-07 | REQ-20260906-003 | 受环境限制 | CLI 引导全量 Swift 回归中 `CLICollectorTests.codexTimeoutIsBounded()` 第 226 行在约 1.916s 得到 `transportFailure`；相关测试/collector 未修改，单独 15/15 通过。保留原断言、完整失败日志与后续整套复验，不因单测重跑成功宣称时序根因已修复。 |
| 2026-09-07 | REQ-20260907-006 / 007 | 进行中 | 需求登记 `96a0bc4`、规格/计划 `95a4d3c`、说明 `3f9874c`；推荐固定官方安装器、点击后执行、已连接提供重新登录。006 正在实施，007 排队待任务审查后执行；不宣称公开 0.4.0 已包含。 |
| 2026-09-07 | REQ-20260907-005 | 已完成 | 0.4.0/build 12 已公开，Tag `15d80e2`、workflow `34076547278`、appcast `2d2254f`；匿名重下七份资产、SHA-256、两端签名及三个更新源一致通过。阶段末重新核对当前队列，没有其他可在当前环境继续的进行中或待处理项；原物理验收/Widget/偶发时序事项保持原状态。 |
| 2026-09-07 | REQ-20260907-001 / 002 / 003 / 004 | 已完成 | 完成实现、独立复审及代码级验收；macOS 完整 405+13 项，Windows 57/197/21/632 项及原生 CI 34073917953、NSIS 全绿。Git 检查点 255e8b3、a8d7a0f、d303c11、83c54c6；PR #9 保留集成证据。未发布，物理双屏/DPI 与 Widget 证书不冒充完成。 |
| 2026-09-07 | REQ-20260906-003 | 受环境限制 | 复发后已补测试专用白名单阶段/耗时/PID 诊断，保留原断言和超时，PTY 13/13 及完整回归通过；历史间歇性根因仍缺少可复现证据，继续追踪，不关闭。 |
| 2026-09-07 | REQ-20260907-004 | 进行中 | 最终 Windows CI 的浏览器字号门禁 15 秒超时，没有样式断言结果。先登记、保存原始失败证据，进行一次原样复验并检查有界超时和诊断；不能跳过字号门禁。 |
| 2026-09-07 | REQ-20260907-001 / 002 / 003 | 进行中 | 用户要求继续执行；macOS 实现与定向复审关闭、405+12 项及 Release 构建通过；Windows `3312537` 本地 56/190/12/632 项门禁通过，PR #9 原生 CI 进行中。独立复审发现协调锁、拖动提交和迟到设置事件竞态，合并前补测试修正。 |
| 2026-09-07 | REQ-20260907-003 | 进行中 | 登记“Windows 详情全部再减少一号”；采用全角色减 1 CSS px，字号测试先失败后通过，代码 `3312537`；不影响 Settings、浮动条、macOS，不以宿主浏览器测试冒充 Windows 字体实机验收。 |
| 2026-09-07 | REQ-20260907-001 | 待用户确认 → 进行中 | 用户“按照你的推荐执行”，确认自由跨屏、系统主屏/指定屏/所有屏、默认单屏、每屏位置及共享采集方案；Windows 本地化和字体按同批已确认范围执行，无需再作常规确认。 |
| 2026-09-07 | REQ-20260907-001 | 新建 → 进行中 → 待用户确认 | 用户报告 M4 Max 双屏启动在副屏且跨屏拖动失败；读取控制器、屏幕解析、位置存储及旧规格后定位两处实现限制。提出单屏修复/选择与可选多屏实例的架构分歧，等待是否本轮加入所有显示器模式，未实现或发布修复。 |
| 2026-09-07 | REQ-20260907-002 | 新建 → 待处理 | 登记 Windows 中英文切换与三款中文字体；用户另确认 Windows 安装成功，仅记录安装，不关闭 DPI、官网同步等未明确确认的专项验收。 |
| 2026-09-06 | REQ-20260906-004 | 已完成 | 用户反馈“macos 已测试成功，另外一台机器也更新成功”，补充 0.3.0 发布后的实际更新成功证据；未提供另一台机器的具体系统与型号，不据此关闭 Windows 或其他专项验收项。详见本版发布记录。 |
| 2026-09-06 | REQ-20260906-004 | 进行中 → 已完成 | `v0.3.0` / build 11 公开，tag `bb215c3`、workflow `34035797098` 全绿；公网重下两包 SHA-256 与签名通过，稳定 appcast 提交 `f9a1f83`，两 Windows feed 内容一致。旧 Mac Preview 实际检查显示 0.3.0 可用；未代用户安装。 |
| 2026-09-06 | REQ-20260906-004 | 新建 → 进行中 | 用户要求发布，便于其他机器直接更新；选择双平台稳定 0.3.0/build 11，沿用已有签名公钥，兼容旧 Windows Preview 通道。 |
| 2026-09-06 | REQ-20260906-002 | 进行中 → 受环境限制 | 功能、403项Swift/51项前端/179项宿主Rust回归、Windows原生CI与NSIS构建、macOS Release资源及签名校验完成，独立复审无Critical/Important，PR #7合并 `c67112e`；真实Windows桌面交互验收仍保留，未发布新版。 |
| 2026-09-06 | REQ-20260906-003 | 新建 → 受环境限制 | 登记间歇性终端测试失败和已做复查；最终CI原断言通过，本机PTY连续三次通过，但缺少稳定复现，保留为可靠性追踪项。 |
| 2026-09-06 | REQ-20260906-002 | 待用户确认 → 进行中 | 用户确认三份规格后，按推荐直接实施并保存阶段Git检查点。 |
| 2026-09-06 | REQ-20260906-002 | 进行中 → 待用户确认 | 已把大范围拆为三份可独立实施和验收的规格：Compact/Comfortable 与可选折叠、浮动条 Provider 显示排序与原生右键菜单、可验证的 Refreshing/Waiting/Idle 状态和持久化退避。规格明确不冒充真实会话运行监控、不改变 Widget/详情数据、不删除深海背景，并锁定双平台尺寸、状态机、错误恢复、隐私和测试边界。 |
| 2026-09-06 | REQ-20260906-002 | 新建 → 进行中 | 用户接受 CodeNotch 研究中的推荐实施方向；登记为跨平台分阶段改版，先锁定 Compact/Comfortable、可选闲置折叠、粗粒度运行状态、可信度/新鲜度、刷新退避、Provider 显示排序和右键菜单的设计与回归边界，再按测试驱动实施。 |
| 2026-09-06 | REQ-20260906-001 | 进行中 → 已完成 | 已核对 CodeNotch 官方 README、设计规格和 `NotchLayout`、`ProviderRing`、`NotchRootView` 实现，并与 AI Token Meter 当前 108pt 主体、60pt 环、Logo-only、深海背景、点击详情和双平台约束对照；形成紧凑密度、可选闲置折叠、粗粒度运行状态、可信度、新鲜度、退避、Provider 排序和右键菜单的分级建议，未直接修改产品。 |
| 2026-09-06 | REQ-20260906-001 | 新建 → 进行中 | 用户要求研究 CodeNotch，重点比较“更瘦但仍清晰”的视觉原因和可借鉴功能；先做官方资料与现有实现对照，不直接修改产品。 |
| 2026-09-04 | REQ-20260903-004 | 进行中 → 待用户确认 | Windows 功能、四个公开 Preview、签名资产和更新源的自动化交付已完成；当前没有可在 macOS 环境继续执行的开发步骤，保留 Windows 11 真机视觉、交互、账号、DeepSeek WebView2 和 `preview.2 → preview.3` 原位更新确认。 |
| 2026-09-04 | REQ-20260904-008 | 进行中 → 已完成 | `v0.3.0-preview.3` 已公开为双平台 prerelease；workflow `33887131319` attempt 2 全绿。公网回下载的 macOS/Windows SHA-256、Sparkle EdDSA、严格 App 签名、Tauri minisign 均通过；Release `latest.json` 与固定 Windows Preview feed 逐字节一致且指向本版本，稳定 macOS appcast 仍保持 `0.2.2`。 |
| 2026-09-04 | REQ-20260904-006 | 待用户确认 | `0.3.0-preview.3` 已公开交付本需求并推进 Windows Preview 更新源；自动化、签名与公网交付闭环完成，保留 Windows 11/WebView2 真实登录、同步、关闭、视觉与原位更新确认。 |
| 2026-09-04 | REQ-20260904-008 | 进行中 | 已确认 macOS 失败点仅为 `PTYCommandRunnerTests.parentExitClosesReader` 在 GitHub runner 上单次达到 2 秒超时；相同测试随后在本机独立连续运行 20 次全部通过（约 0.78–0.83 秒），版本、375 项主测试、签名和发布资产均未失败。Windows 安装包继续构建，完成后仅重跑失败门禁。 |
| 2026-09-04 | REQ-20260904-008 | 进行中 | 发布提交 `dac10b9`、Tag `v0.3.0-preview.3`、本机签名 macOS 资产与草稿 Release 已生成；workflow `33887131319` 的 macOS 标签源码复验失败，Windows 构建仍在运行，因此 Release 保持草稿且更新源未切换。进入系统化诊断，失败关闭前不公开。 |
| 2026-09-04 | REQ-20260904-008 | 进行中 | `0.3.0-preview.3` 发布提交前门禁通过：macOS 375 项主测试 + 12 项 PTY 测试、Windows 43 项前端 + 12 项进程生命周期、真实 Chrome 计算样式、production build、跨平台版本合同、150 份 Markdown 与公开安全检查均成功；本地预览服务器首次受沙箱 `EPERM` 阻止后，在获准的本机回环环境中原命令通过。 |
| 2026-09-04 | REQ-20260904-008 | 进行中 | 发布版本确定为 `0.3.0-preview.3`（build `10`）；根、macOS、Widget、Windows npm/Cargo/Tauri 元数据和 Release Notes 已同步，进入完整测试、签名打包与发布前安全门禁。 |
| 2026-09-04 | REQ-20260904-008 | 新建 → 进行中 | 用户明确要求直接发布最新版；开始核对上一版发布合同、同步版本号、Release Notes、双平台签名资产和 Preview 更新源，发布完成前不宣称其他机器已可更新。 |
| 2026-09-04 | REQ-20260904-007 | 新建 → 已完成 | 已核对稳定 `appcast.xml`、公开 Preview/Tag 和 README 发布声明：稳定更新源仍为 `0.2.2`，最新公开 Preview 仍为 `0.3.0-preview.2`，而 `REQ-20260904-006` 修复是在其后才合入 `main`，所以其他机器目前不能通过应用内更新取得本次修复。 |
| 2026-09-04 | REQ-20260904-006 | 待用户确认 | 合并记录提交 `d520752` 的 Windows main CI `33880527388` 与 macOS main CI `33880527365` 均全绿；Windows 在 `main` 再次通过真实 Chrome 密度、严格 Rust、169 项原生测试、Release Tauri/NSIS、PE GUI subsystem 与安装器上传。 |
| 2026-09-04 | REQ-20260904-006 | 待用户确认 | PR #6 已以合并提交 `e62193c` 并入 `main`；代码、文档、审查与 CI 阶段收口。当前公开 `0.3.0-preview.2` 不含本修复，下一步是制作明确列出本需求的双平台 Preview，并在 Windows 11 完成真实交互确认。 |
| 2026-09-04 | REQ-20260904-006 | 进行中 → 待用户确认 | 四项 Important 已以失败先行测试关闭，独立复审无 Critical/Important；本机 Windows 43 项前端、12 项密度生命周期、169 项 Rust、macOS 375+12、production build、Release App、148 份文档与公开安全门禁通过。PR #6 的 Windows CI `33878105470` 进一步完成真实 Chrome、Windows-only/Tauri/NSIS/PE 验证，macOS CI `33878105480` 同时全绿；自动化阶段完成，保留下一版 Preview 的 Windows 11 真实交互确认。 |
| 2026-09-04 | REQ-20260904-006 | 进行中 | PR #6 第六轮 Windows runner 通过独立 profile 返回 4996 字符完整已渲染 DOM，证明浏览器复用已排除；报告节点仍为空，根因收敛为 Windows headless 在 `--dump-dom` 前未调度 `requestAnimationFrame`。密度 fixture 将改为 React 同步提交后立即读取计算样式，不再依赖后台绘制帧。 |
| 2026-09-04 | REQ-20260904-006 | 进行中 | PR #6 第五轮 Windows runner 已等待浏览器 stdout/stderr 关闭，但 `chrome.exe --dump-dom` 仍以成功状态返回且输出不含 `density-report`；进入浏览器独立 profile、实际 DOM 诊断与跨平台启动参数核对，当前不把密度门禁失败误判为产品代码失败。 |
| 2026-09-04 | REQ-20260904-006 | 进行中 | PR #6 第四轮 Windows runner 已通过随机端口解析并启动 headless 浏览器，但进程 `exit` 先于 stdout 管道关闭，脚本过早返回空/不完整 DOM。新增“exit 后仍有 stdout、close 后才完成”的回归，改用流关闭事件收齐输出。 |
| 2026-09-04 | REQ-20260904-006 | 进行中 | PR #6 第三轮 Windows runner 已能直接启动 Vite，但 Vite 的 Windows 彩色输出在 `Local`、URL 与端口之间插入 ANSI 控制码，原纯文本正则因而等待满 10 秒。将为 ANSI 装饰的真实输出增加解析回归，去除终端控制码后再识别随机端口。 |
| 2026-09-04 | REQ-20260904-006 | 进行中 | PR #6 第二轮 Windows runner 通过 43 项前端行为后，在 production 密度门禁复现 Node 24/Windows 不能直接 spawn `npm.cmd` 的 `EINVAL`；新增平台测试并改为用当前 Node 直接启动固定的 Vite CLI，避免 shell 和参数拼接，等待第三轮 runner。 |
| 2026-09-04 | REQ-20260904-006 | 进行中 | PR #6 的首轮 Windows runner 在前端测试阶段暴露跨平台性能波动：`opens one rich detail and closes it on an outside click` 使用多次可见性/可访问性全树计算，在 Windows jsdom 上超过 5 秒；生产行为未失败。按同一质量门禁移除该用例不必要的样式计算并重跑 runner。 |
| 2026-09-04 | REQ-20260904-006 | 进行中 | 用户明确要求继续修复直到完成；开启新的质量修复周期，按推荐技术方案依次关闭分片停滞计时、前端 attempt/generation 所有权、活跃会话保留和已有历史时的清理恢复入口，并重新执行独立审查、全量门禁和 Windows runner 验证。 |
| 2026-09-04 | REQ-20260904-006 | 待用户确认 → 进行中 | 最终定向复审重新打开 4 项 Important：分片传输缺少独立 20 秒停滞计时器；初始状态查询缺少 attempt/epoch 所有权；命令拒绝后的失败查询可能覆盖已确认 active 会话；历史已存在时销毁失败没有可达的恢复入口。当前分支保留至 `5982d5c`，禁止合并或发布，须开启新的修复周期并重新完成审查与全量门禁。 |
| 2026-09-04 | REQ-20260904-006 | 待用户确认 | 整分支最终审查的九项修复及独立复审补强已提交为 `4953206`；执行代理报告前端 37/37、密度生命周期 7/7、Rust 160/160、macOS 375+12/387 项（72 组）、production build、rustfmt、严格 Clippy、146 份 Markdown 与公开安全门禁通过。该阶段“无 Critical/Important 遗留”的判断已被随后定向复审推翻，以上仅保留为历史记录。 |
| 2026-09-04 | REQ-20260904-006 | 待用户确认 | 整分支最终审查修复波开始：集中处理登录与分片期限分离、原子历史合并、带 generation 的可恢复状态通道、官网可用性握手、销毁失败重试、详情所有权、进程组回收及密度门禁证据表述；仍不改变 Windows 11 真机待验收状态。 |
| 2026-09-04 | REQ-20260904-006 | 待用户确认 | 任务 5 审查修复第 1 轮完成：README、Provider 指南与排障文档已明确 `0.3.0-preview.2` 不含本次修复，验收前必须安装 Release Notes 列出本需求的下一版 Preview；开发索引去重并纠正发布事实，macOS 基线纠正为 386 项/72 组，重复日志目标回归门禁、文档门禁、公开安全检查和 diff 检查均通过。 |
| 2026-09-04 | REQ-20260904-006 | 待用户确认 | 任务 5 审查修复第 1 轮开始：严格区分已发布的 `0.3.0-preview.2` 与尚未发布的 Windows DeepSeek/紧凑密度修复，纠正开发日志重复索引与自动化测试计数，并增加开发日志重复目标门禁；真实 Windows 验收状态不变。 |
| 2026-09-04 | REQ-20260904-006 | 进行中 → 待用户确认 | 方案 A 的代码、竞态加固、24 项前端、4 项密度生命周期、141 项 Rust、production build、rustfmt/严格 Clippy 及本机双平台文档/安全门禁完成，文档检查点为 `5f7a141`；macOS 样式未改。Windows MSVC/Tauri 壳需 runner 构建，真实 DeepSeek 登录、关闭、复用聚焦、完成恢复与原生字体下拉仍待 Windows 11 用户验收，未标已完成。 |
| 2026-09-04 | REQ-20260904-006 | 进行中 | 任务 5 开始：补齐根因、验证、安全边界和 Windows 11 真机验收文档，运行仓库既有双平台本机门禁并准备最终文档提交；无 Windows 真机证据的登录、关闭、聚焦与字体下拉验收继续保留为待确认。 |
| 2026-09-04 | REQ-20260904-005 | 待用户确认 → 已完成 | 用户已在 Windows 11 成功安装并启动 `0.3.0-preview.2`，未再报告启动时弹出 Windows Terminal；启动级控制台窗口缺陷完成真机闭环。 |
| 2026-09-04 | REQ-20260904-006 | 新建 → 进行中 | Windows 真机确认 DeepSeek Key 与余额可用，但点击 DeepSeek 后同时出现空白独立窗口，“Sync official history”无响应；三项 Provider 详情页字号相较 macOS 过大，进入官方历史 WebView/桥接和 Windows 独立字体尺度的系统化调试。 |
| 2026-09-04 | REQ-20260904-006 | 进行中 | 用户补充：空白 DeepSeek 官方历史窗口弹出后无法关闭；Settings 自身字号同样过大，字体选择的原生下拉列表出现白底白字、仅鼠标悬停时可辨识。修复范围扩展到该窗口完整生命周期与 Windows Settings/select 专属样式。 |
| 2026-09-04 | REQ-20260904-006 | 进行中 | 用户选择方案 A：查看详情不再自动建窗；显式同步使用隐藏加载、置前、可关闭、成功/失败恢复的托管官网窗口；Windows 详情与 Settings 使用紧凑字号，字体下拉框明确深色文字和白色背景，macOS 不变。规格与测试驱动计划已建立。 |
| 2026-09-04 | REQ-20260904-006 | 进行中 | 任务 2 开始：以纯 Rust 失败先行测试锁定 DeepSeek 官网窗口的隐藏加载、复用聚焦、关闭/失败恢复和成功销毁状态机，再接入 WebView2 生命周期。 |
| 2026-09-04 | REQ-20260904-006 | 进行中 | 任务 2 完成：官网 WebView2 现由六态状态机托管，隐藏加载后显示聚焦，重复请求复用窗口，关闭/30 秒加载超时/失败/成功均清理会话并恢复详情；15 项定向与 138 项完整 Rust 测试、rustfmt、严格 Clippy 通过，等待后续前端状态反馈任务。 |
| 2026-09-04 | REQ-20260904-006 | 进行中 | 任务 2 审查修复第 1 轮开始：补齐会话代次隔离、nonce 绑定官网 ready 握手、completion 终止权和可注入窗口动作失败边界，防止旧回调/计时器污染重开会话或在关键窗口动作失败时误报成功。 |
| 2026-09-04 | REQ-20260904-006 | 进行中 | 任务 2 审查修复第 1 轮完成：生命周期、分片组装器、会话代次与可取消 30 秒计时器已统一托管；仅 nonce 绑定官网 ready 握手可激活窗口；completion 先取得唯一终止权；窗口动作与数据发布失败统一降级为 failed。18 项定向与 141 项完整 Rust 测试、rustfmt、严格 Clippy 和文档门禁通过，等待 Windows 真机整体验收。 |
| 2026-09-04 | REQ-20260904-006 | 进行中 | 任务 3 开始：Windows 详情页将订阅脱敏的 `deepseek-history-status` 状态，并将同步触发、失败反馈与自动隐藏暂停纳入前端失败先行测试；macOS 保持不变。 |
| 2026-09-04 | REQ-20260904-006 | 进行中 | 任务 3 完成：Windows 详情页已消费固定同步状态、命令拒绝会显示可重试错误且同步中暂停自动隐藏；19 项前端测试、生产构建和文档门禁通过。整体需求仍等待后续任务与 Windows 真机验收。 |
| 2026-09-04 | REQ-20260904-006 | 进行中 | 任务 3 审查修复第 1 轮开始：处理延迟命令 reject 覆盖新同步状态、部分事件监听注册失败后遗留订阅，以及后端 `failed` 终态恢复详情倒计时三项前端竞态边界。 |
| 2026-09-04 | REQ-20260904-006 | 进行中 | 任务 3 审查修复第 1 轮完成：同步尝试代次隔离、独立监听清理和 `failed` 终态倒计时回归均已补齐；22 项前端测试、生产构建和文档门禁通过。整体需求仍等待后续任务与 Windows 真机验收。 |
| 2026-09-04 | REQ-20260904-006 | 进行中 | 任务 4 完成：Windows Provider 详情和 Settings 以明确紧凑密度语义收紧字号/间距；Settings 继续锁定 Segoe UI，原生下拉框使用白底深色文字并声明浅色配色。24 项前端测试、生产构建和浏览器详情视觉检查通过；整体需求仍等待后续任务与 Windows 真机验收。 |
| 2026-09-04 | REQ-20260904-006 | 进行中 | 任务 4 审查修复第 1 轮开始：现有组件测试只验证密度类和节点，未加载 production CSS，无法阻止字号、控件尺寸或 select 可读配色回归；增加最小 Vite/Chromium 计算样式门禁。 |
| 2026-09-04 | REQ-20260904-006 | 进行中 | 任务 4 审查修复第 1 轮完成：新增可重复的 Vite/Chrome headless 计算样式门禁，覆盖详情 14/20/24/13/18px、Settings 14/20/13/32px、系统字体隔离、浅色 scheme 与 select/option 深字白底；临时将详情正文变为 15px 时门禁按预期失败，恢复后通过。 |
| 2026-09-04 | REQ-20260904-006 | 进行中 | 任务 4 审查修复第 2 轮开始：字体隔离 fixture 以默认 Segoe 根字体掩盖 Settings 专属规则缺失，浏览器门禁也未进入 Windows PR/Release；同时修复浏览器探测和 Vite 子进程的并发/清理风险。 |
| 2026-09-04 | REQ-20260904-006 | 进行中 | 任务 4 审查修复第 2 轮完成：浏览器 fixture 现在显式继承 Antonio，Settings 仅可由系统字体规则覆盖；移除规则时门禁按预期失败。门禁已接入 Windows CI/Release，优先支持 BROWSER_BIN/CHROME_BIN、Chrome 和 Edge，并用 Vite `--port 0`、退出等待及 Windows `taskkill /T` 消除端口/子进程风险。 |
| 2026-09-04 | REQ-20260904-006 | 进行中 | 任务 4 审查修复第 3 轮开始：Unix/macOS 的 Vite 清理仅终止 npm 父 PID，且浏览器无外部超时；将以独立进程组、有限 TERM/KILL 与可测试的浏览器超时回收修复泄漏风险。 |
| 2026-09-04 | REQ-20260904-006 | 进行中 | 任务 4 审查修复第 3 轮完成：Vite 与 headless 浏览器在 Unix/macOS 均作为独立进程组启动，清理对整个组执行 TERM 后有限等待再 KILL；Windows 保留 `taskkill /T /F`。浏览器增加 15 秒硬超时并无论超时、失败或正常退出都在 finally 回收进程树；4 项 Node 生命周期测试及完整前端/构建/文档门禁通过。 |
| 2026-09-01 | REQ-20260901-001 | 新建 → 进行中 | 设计规格已确认，用户授权计划完成后直接开发。 |
| 2026-09-01 | REQ-20260901-001 | 进行中 → 已完成 | 267 项测试、Release 签名/安装哈希和真实 Settings 验收完成；临时签名导致的旧 Keychain 项访问限制已如实记录，未修改旧 Key。 |
| 2026-09-01 | REQ-20260901-002 | 新建 → 进行中 | 用户要求所有未来需求都先进入统一列表。 |
| 2026-09-01 | REQ-20260901-002 | 进行中 → 已完成 | 已建立根级强制工作规则、分类台账、状态流转、文档入口和 Git 证据。 |
| 2026-09-01 | REQ-20260901-003 | 新建 → 已延期 | 用户要求 Widget 证书先放一放。 |
| 2026-09-01 | REQ-20260901-004 | 新建 → 受环境限制 | 当前只有一个普通 Space，自动化环境无第二显示器且无法稳定控制 Mission Control/非激活 NSPanel 指针。 |
| 2026-09-01 | REQ-20260901-005 | 新建 → 进行中 | 用户反馈桌面已有窗口时详情弹到窗口栈底部，要求点击后显示在所有普通应用窗口上方。 |
| 2026-09-01 | REQ-20260901-005 | 进行中 → 已完成 | 角色化窗口层级已上线；268 项测试、Release 安装哈希和已安装 App 实时窗口层级验收通过。 |
| 2026-09-01 | REQ-20260901-006 | 新建 → 进行中 | 用户希望 Claude 详情页不再单薄，优先评估最近 30 天用量等可验证数据。 |
| 2026-09-01 | REQ-20260901-006 | 进行中 → 待用户确认 | 已确认个人订阅无公开稳定的跨设备 30 天用量接口；本机 Claude Code 记录可安全聚合 token、会话与活跃日统计。 |
| 2026-09-01 | REQ-20260901-007 | 新建 → 进行中 | 用户希望字体选择器新增 Alimama FangYuanTi VF、Fira Code、Leigo、Menlo、Alimama DaoLiTi。 |
| 2026-09-01 | REQ-20260901-006 | 待用户确认 → 进行中 | 用户选择方案 A：官方额度优先，下方补充明确标注 This Mac 的近 30 天 Claude Code 活动。 |
| 2026-09-01 | REQ-20260901-007 | 范围确认 | 用户确认 Leigo 指 Ricardo Medina 的 Leigo Regular。 |
| 2026-09-01 | REQ-20260901-006 | 进行中 → 待用户确认 | 方案 A 的数据口径、隐私边界、布局、错误处理与测试规格已完成并通过自检。 |
| 2026-09-01 | REQ-20260901-007 | 进行中 → 待用户确认 | 方案 A 的安装检测、家族映射、回退、应用范围与测试规格已完成并通过自检。 |
| 2026-09-01 | REQ-20260901-006 | 待用户确认 → 进行中 | 用户确认书面规格；实施计划已完成，进入测试先行开发。 |
| 2026-09-01 | REQ-20260901-007 | 待用户确认 → 进行中 | 用户确认书面规格；实施计划已完成，进入测试先行开发。 |
| 2026-09-01 | REQ-20260901-006 | 进行中 → 已完成 | Claude 专用详情、本机 30 日聚合、流式与超时加固、295 项回归、Release 安装哈希与真实快照验收完成。 |
| 2026-09-01 | REQ-20260901-007 | 进行中 → 已完成 | 八项字体目录、别名与回退、资源扫描、295 项回归和真实 Settings 验收完成。 |
| 2026-09-01 | REQ-20260901-008 | 新建 → 待用户确认 | 用户要求 Claude 详情页不再显示 Token composition 与 Top models，等待确认精简布局规格。 |
| 2026-09-02 | REQ-20260901-008 | 待用户确认 → 进行中 | 用户选择方案 A：仅移除两张展示卡片，保留采集与缓存兼容；书面规格进入最终确认。 |
| 2026-09-02 | REQ-20260901-008 | 进行中 | 用户确认书面规格；完成实施计划并进入隔离分支开发。 |
| 2026-09-02 | REQ-20260901-008 | 进行中 → 已完成 | 移除两张展示卡片，保留采集与缓存兼容；294 项测试、正式构建、安装哈希和真实自动隐藏验收通过。 |
| 2026-09-02 | REQ-20260902-009 | 新建 → 待用户确认 | 用户要求移除 Claude 详情底部隐私说明；等待确认完整移除、不留占位的最小方案。 |
| 2026-09-02 | REQ-20260902-009 | 待用户确认 | 用户选择方案 A：完整移除文字和锁形图标，不增加占位、提示或设置开关。 |
| 2026-09-02 | REQ-20260902-009 | 待用户确认 → 进行中 | 用户确认书面规格，进入实施计划和测试先行开发。 |
| 2026-09-02 | REQ-20260902-009 | 进行中 | 完成测试驱动实施计划，准备在隔离分支执行。 |
| 2026-09-02 | REQ-20260902-009 | 进行中 → 已完成 | 完整移除隐私说明和锁图标；295 项测试、Release 签名、安装哈希及真实辅助功能树验收通过。 |
| 2026-09-02 | REQ-20260902-010 | 新建 → 待用户确认 | 用户希望菜单栏状态图标更现代、更具极客感；等待确认视觉方案。 |
| 2026-09-02 | REQ-20260902-010 | 待用户确认 | 用户选择方案 A「Quantum Dial」作为菜单栏图标正式视觉方向。 |
| 2026-09-02 | REQ-20260902-010 | 待用户确认 | 用户确认动态图标：外圈和指针反映最高已用比例，保留精确百分比文字，无数据时显示中性状态。 |
| 2026-09-02 | REQ-20260902-010 | 待用户确认 | 完整设计已口头确认；书面规格已完成，等待最终审阅。 |
| 2026-09-02 | REQ-20260902-010 | 待用户确认 → 进行中 | 用户确认书面规格，进入实施计划和测试先行开发。 |
| 2026-09-02 | REQ-20260902-010 | 进行中 | 完成测试驱动实施计划，准备在隔离分支执行。 |
| 2026-09-02 | REQ-20260902-010 | 进行中 → 已完成 | Quantum Dial 接入菜单栏；299 项测试、双外观像素渲染、Release 严格签名、安装哈希与已安装应用真实刷新链路验收通过。 |
| 2026-09-02 | REQ-20260902-011 | 新建 → 待用户确认 | 用户要求 Claude 统一显示为 Claude Code，Codex 统一显示为 OpenAI Codex；先确认当前界面与历史记录的边界。 |
| 2026-09-02 | REQ-20260902-011 | 待用户确认 | 用户批准推荐边界：统一当前界面、辅助功能、通知、Widget 和现行文档；保留历史记录、技术标识、路径与 CLI 命令。 |
| 2026-09-02 | REQ-20260902-011 | 待用户确认 | 集中式正式名称设计已完成并通过占位符、内部一致性、范围与模糊性自检。 |
| 2026-09-02 | REQ-20260902-011 | 待用户确认 → 进行中 | 用户确认书面规格；进入实施计划和测试先行开发。 |
| 2026-09-02 | REQ-20260902-011 | 进行中 | 完成测试驱动实施计划，准备在隔离分支执行。 |
| 2026-09-02 | REQ-20260902-011 | 进行中 | 核心正式名称、主应用、通知、Widget 与现行维护文档已同步；进入完整验证与安装验收。 |
| 2026-09-02 | REQ-20260902-011 | 进行中 → 已完成 | 当前 UI、辅助功能、通知、Widget 源码和现行文档已统一；分支与合并后的 `main` 均通过 304 项测试/60 个测试组，Release 安装哈希和真实 Settings/浮动条验收通过。 |
| 2026-09-02 | REQ-20260902-012 | 新建 → 进行中 | 用户反馈菜单栏图标看不见；先按系统化调试收集已安装版本、真实菜单栏和动态图标生成链路证据。 |
| 2026-09-02 | REQ-20260902-012 | 进行中 → 待用户确认 | 根因是自绘 SwiftUI 图形未作为菜单栏模板图像输出；修复候选已安装，305 项测试、Release 签名和安装哈希通过，等待用户肉眼确认。 |
| 2026-09-02 | REQ-20260902-012 | 待用户确认 → 进行中 | 用户确认已能看到菜单栏图标，真实视觉验收通过；进入主分支集成。 |
| 2026-09-02 | REQ-20260902-012 | 进行中 → 已完成 | 模板图像修复已合入 `main`；用户视觉确认、Release 安装哈希及合并前后 305 项测试/60 个测试组全部通过。 |
| 2026-09-02 | REQ-20260902-013 | 新建 → 进行中 | 用户要求完整复盘项目、补齐文档、删除无用内容，并确保长期可追溯；先执行全仓库与 Git 历史盘点。 |
| 2026-09-02 | REQ-20260902-013 | 进行中 → 已完成 | 单一需求/设计入口、项目状态、13 项架构决策、维护手册和文档门禁已落地；308 项测试、61 个测试组、104 份 Markdown、Release 严格签名和合并后复验通过；清理约 628 MB 可重建残留。 |
| 2026-09-02 | REQ-20260902-014 | 新建 → 进行中 | 用户要求生成可迁移到 MacBook Pro M4 Max 使用的应用包；采用 Apple Silicon arm64 ZIP、SHA-256 和无 Widget 明确边界。 |
| 2026-09-02 | REQ-20260902-014 | 进行中 → 已完成 | 308 项测试与 104 份文档检查通过；arm64、macOS 14+、ad-hoc Release 构建、ZIP 完整性、解压后严格签名和 SHA-256 验证通过。 |
| 2026-09-02 | REQ-20260902-015 | 新建 → 进行中 | 用户反馈 0.1.0 分发包在 MacBook 上启动即崩溃；按资源包装载、跨目录启动与签名顺序开展系统化修复。 |
| 2026-09-02 | REQ-20260902-015 | 进行中 → 已完成 | 修复 SwiftPM 资源绝对路径回退并升级 0.1.1；312 项测试、标准资源门禁、最终 ZIP 解压签名与隐藏构建目录启动验收通过。 |
| 2026-09-02 | REQ-20260902-016 | 新建 → 进行中 | M4 Max 上 Claude Code 与 DeepSeek 正常，但 OpenAI Codex 显示 `CLI not installed`；先区分真实缺失与 PATH/安装位置发现失败。 |
| 2026-09-02 | REQ-20260902-016 | 进行中 | M4 Max 详情页同步显示 `Not installed`、官方额度空白和本机活动不可用，确认三个表现来自同一个 Codex 可执行文件定位失败。 |
| 2026-09-02 | REQ-20260902-016 | 进行中 | M4 Max 终端确认 Codex CLI `0.148.0` 位于 nvm 的 Node `v25.2.0` bin，Finder/launchctl 未提供该 PATH；修复范围扩展为发现脚本并保证其 `/usr/bin/env node` 可执行。 |
| 2026-09-02 | REQ-20260902-016 | 进行中 → 待用户确认 | 已发布 0.1.2 候选：318 项测试、64 个测试组、Release 资源/签名、ZIP 跨目录启动及正常权限下最小 PATH 的真实 Codex 冒烟通过；等待 M4 Max nvm 环境安装确认。 |
| 2026-09-02 | REQ-20260902-017 | 新建 → 进行中 | 用户要求将项目公开发布到其 GitHub 账户，包含标准文档、截图和 Release 下载；必须先完成个人 Key、身份信息与历史泄露审计，并在 About 标注作者 Miller。 |
| 2026-09-02 | REQ-20260902-017 | 进行中 → 待用户确认 | 用户确认方案 A 和 MIT License；完整书面规格已覆盖公开边界、文档、截图、About、历史/产物扫描、CI、Release 与停止条件，等待复核。 |
| 2026-09-02 | REQ-20260902-017 | 待用户确认 → 进行中 | 用户确认书面规格并授权实施；详细计划拆分为安全门禁、品牌许可、社区文档、脱敏截图、本地审计、GitHub 发布和最终复核七个检查点。 |
| 2026-09-02 | REQ-20260902-018 | 新建 → 进行中 | 首次公开 CI 暴露 macOS 15 runner 上 `[Int64? ?? 整数字面量]` 的类型推断差异；本机测试通过但远程编译失败，进入跨工具链兼容修复。 |
| 2026-09-02 | REQ-20260902-018 | 进行中 → 已完成 | 显式 `Int64`、跨 SDK 严格并发、Xcode 26.3 工具链及三项 CI 时序竞态均已修复；本机 330 项测试/67 个测试组和公开 GitHub Actions 33637095658 全部通过。 |
| 2026-09-02 | REQ-20260902-018 | 已完成 → 进行中 | 标签候选复验 33637436534 再现 PTY 输出丢失和两项测试时钟竞态，证明前一状态过早；需求重新打开并改为异步退出等待与事件驱动测试。 |
| 2026-09-02 | REQ-20260902-018 | 进行中 → 已完成 | 用 continuation 取代并发线程池内的同步阻塞退出等待，保留必要的同步清理入口；增加确定性睡眠夹具与 32 路 PTY 回归，本机 331 项测试及公开 GitHub Actions 33638241625 全部通过。 |
| 2026-09-02 | REQ-20260902-017 | 进行中 → 已完成 | 公开仓库、MIT、作者 Miller、标准社区文档、三张脱敏截图、完整历史与 ZIP 安全扫描、331 项测试、两轮最终公开 CI、`v0.1.2` 标签和双资产 Release 均已完成；匿名访问 README/截图与重新下载 SHA-256 复验通过。 |
| 2026-09-02 | REQ-20260902-019 | 新建 → 进行中 | 用户要求在 Settings 增加检查更新和立即更新，并在 GitHub 有新版本时自动下载和安装；先进入安全更新设计。 |
| 2026-09-02 | REQ-20260902-019 | 进行中 | 用户选择方案 A：仅在手动点击“检查更新”时联网；发现新版后由用户点击“立即更新”下载安装，不做后台检查或静默安装。 |
| 2026-09-02 | REQ-20260902-019 | 进行中 → 待用户确认 | Sparkle 2、GitHub appcast、EdDSA 签名、About 双按钮、失败回退、首次手动升级与测试验收的书面规格已完成并通过自检。 |
| 2026-09-02 | REQ-20260902-019 | 待用户确认 → 进行中 | 用户确认书面规格；进入实施计划和测试先行开发，不再等待额外计划审批。 |
| 2026-09-02 | REQ-20260902-020 | 新建 → 进行中 | v0.2.0 主分支发布验证中，32 路 PTY 测试稳定复现 `transportFailure`；最小 `openpty` 并发实验确认分配调用存在竞争。 |
| 2026-09-03 | REQ-20260903-001 | 新建 → 进行中 | v0.2.0 发布提交的公开 CI 失败；完整日志排除 Homebrew 警告，定位为高负载下 PTY 父进程退出超时与并发输出尾部丢失。本地 360 项测试和发布包验证不受影响。 |
| 2026-09-03 | REQ-20260902-020 | 进行中 → 已完成 | 串行化唯一 PTY 分配临界区；361 项本机回归、11 项独立 PTY 测试和 v0.2.1 最终公开 CI 全部通过。 |
| 2026-09-03 | REQ-20260903-001 | 进行中 → 已完成 | 增加独立退出确认、有限尾部排空并隔离 PTY 系统资源测试；两轮公开 CI 通过，v0.2.1 补丁已发布且资产复验一致。 |
| 2026-09-03 | REQ-20260902-019 | 进行中 → 待用户确认 | v0.2.0 与 v0.2.1 Release、公开 appcast、EdDSA ZIP、远端 CI 和公开下载复验均通过；隔离 0.2.0 已发现 0.2.1，等待实际安装动作确认。 |
| 2026-09-03 | REQ-20260903-002 | 新建 → 进行中 | 用户授权真实安装后，界面停在 Preparing；系统化诊断确认 EdDSA 已通过、代码签名差异日志不是阻断，真正等待的是被 Settings 遮挡的 Ready to Install 窗口。手动置前并点击后，0.2.1 原位替换与重启通过。 |
| 2026-09-03 | REQ-20260903-003 | 新建 → 进行中 | v0.2.2 同一提交的 PR CI 已通过，但 main CI 中 `The active provider is cleared when its timeout expires` 在固定等待后仍看到 DeepSeek；进入时序证据与事件驱动测试修复。 |
| 2026-09-03 | REQ-20260903-001 | 已完成 → 进行中 | v0.2.2 的第二次 main CI 在独立 PTY 进程中再现并发输出缺失；原结论过早，发布暂停并重新进入系统化调试。 |
| 2026-09-03 | REQ-20260903-003 | 进行中 → 已完成 | 改用可控睡眠器和事件驱动等待；聚焦测试连续 30 轮、本机完整回归、PR CI 33701250078 和最终 main CI 33702415007 均通过。 |
| 2026-09-03 | REQ-20260903-001 | 进行中 → 已完成 | 确认复发来自 PTY 测试 fixture 的 64 个额外子进程扇出；改用 Shell 内建读取并断言退出码后，本机并发用例连续 200 轮、PR CI 33702291460 与 main CI 33702415007 均通过。 |
| 2026-09-03 | REQ-20260903-002 | 进行中 → 已完成 | v0.2.2 从 Settings 启动时自动隐藏 Settings 并激活 Sparkle；隔离 0.2.1 从公开 appcast 下载、置前、原位替换和自动重启通过，落盘版本、签名及二进制哈希与公开 Release 一致。 |
| 2026-09-03 | REQ-20260902-019 | 待用户确认 → 已完成 | 用户授权真实安装；公开 v0.2.2 EdDSA 更新在隔离 0.2.1 上完成检查、下载、双确认、原位替换与自动重启，应用内更新端到端闭环成立。 |
| 2026-09-03 | REQ-20260903-004 | 新建 → 进行中 | 用户要求制作 Windows 版本，并把后续更新调整为 macOS 与 Windows 同步开发、测试和发布；先完成现有架构审计与跨平台设计。 |
| 2026-09-03 | REQ-20260903-004 | 进行中 | 用户确认首版采用推荐范围：Windows 11 x64、系统托盘、贴边浮动条、三服务、详情、Settings、自动更新和同版本 Release；Widget 暂不纳入。 |
| 2026-09-03 | REQ-20260903-004 | 进行中 → 待用户确认 | 完成当前 SwiftUI/AppKit 架构审计与 Windows 技术验证；推荐保留 macOS 原生应用、在同仓库新增 Tauri 2 + Rust Windows 应用，并以共享合同和双平台发布门禁保持同步，等待一次重大架构确认。 |
| 2026-09-03 | REQ-20260903-004 | 待用户确认 | 用户选择方案 A；已将代码边界、共享合同、原生/WSL Provider、Windows 窗口、凭据、更新、双平台 CI/Release、测试与停止条件写入正式规格并完成自检。 |
| 2026-09-03 | REQ-20260903-004 | 待用户确认 → 进行中 | 用户确认正式书面规格；进入详细实施计划、测试驱动开发和分阶段 Git 检查点。 |
| 2026-09-03 | REQ-20260903-004 | 进行中 | 根版本、共享 Schema/fixture/展示合同/对等矩阵及门禁完成；365 项 macOS 回归通过，Windows Tauri 前端与 Rust 骨架本机构建通过。 |
| 2026-09-03 | REQ-20260903-005 | 新建 → 进行中 | 安装 Windows npm 依赖后稳定复现第三方 README 坏链噪声；错误路径全部位于 `windows/node_modules`，检查器排除列表没有该目录。 |
| 2026-09-03 | REQ-20260903-005 | 进行中 → 已完成 | 文档检查排除 npm 依赖；Gitleaks 只扫描 Git 公开候选快照而非 Rust 构建二进制，同时保留未忽略源码和完整历史扫描；两项失败先行回归均通过。 |
| 2026-09-03 | REQ-20260903-004 | 进行中 | Windows 领域模型、共享展示合同、过期判定、原子设置/缓存和日志脱敏完成；16 项 Rust 测试、格式与零警告 Clippy 通过，进入安全凭据和 DeepSeek 采集。 |
| 2026-09-03 | REQ-20260903-006 | 新建 → 进行中 | Task 3 验证时按计划中的 `check-public-release.sh .` 执行，稳定复现仓库目录被误判为 ZIP；正确的具名仓库参数调用已单独验证通过。 |
| 2026-09-03 | REQ-20260903-006 | 进行中 → 已完成 | 新增失败先行的文档门禁回归并修正计划命令；368 项 macOS 回归、Windows Rust/前端测试、文档和公开仓库安全扫描通过。 |
| 2026-09-03 | REQ-20260903-004 | 进行中 | DeepSeek 固定官方端点、响应上限、超时缓存、验证后换 Key、零化 Secret 和 Windows Credential Manager 已实现；24 项 Rust 测试及 Windows API 源码目标编译通过，真实凭据往返等待 Windows runner。 |
| 2026-09-03 | REQ-20260903-007 | 新建 → 进行中 | 开始 Task 5 前对照计划发现 `windows-cli-locations.json` 会被现有“恰好四份 JSON 快照”门禁误判；先修复合同边界再新增 fixture。 |
| 2026-09-03 | REQ-20260903-007 | 进行中 → 已完成 | 将非用量 fixture 放入显式 `auxiliary/` 子目录；四份用量快照计数、Rust fixture 测试、128 份文档和公开安全门禁均通过。 |
| 2026-09-03 | REQ-20260903-004 | 进行中 | 原生/WSL CLI 发现完成：环境候选、注册表 PATH、Node/CMD 显式 launcher、UTF-16 WSL 列表和参数隔离均有测试；32 项 Rust、Windows 目标源码编译和全仓门禁通过，进入安全进程与 ConPTY。 |
| 2026-09-03 | REQ-20260903-004 | 进行中 | 受限非交互 runner、Job Object、ConPTY 进程附加、固定输入/等待和 Claude/Codex 固定登录动作已实现；本机行为测试及 Windows 目标源码编译通过，已提前加入 Windows CI，等待真实 runner 结果。 |
| 2026-09-03 | REQ-20260903-004 | 进行中 | 首次 Windows CI `33714907252` 已证明前端、构建、格式、Clippy、Rust 编译及 ConPTY 创建/缩放通过；输入往返在 CRLF 终端序列下超时，现改用单 CR 并增加“仅终端回显/真实进程响应”诊断，等待真实 runner 复验。 |
| 2026-09-03 | REQ-20260903-004 | 进行中 | 第二次 Windows CI `33715538395` 仍在任何受控文本出现前超时；结合微软 ConPTY 实现确认启动会先发 `ESC[6n` 光标查询，宿主不回复便暂停输入。现已实现 `ESC[1;1R` 最小终端握手，并把真机测试拆为 `ready → input → received`。 |
| 2026-09-03 | REQ-20260903-004 | 进行中 | 第三次 Windows CI `33716133098` 在运行测试前被 Windows 条件编译专属的 Clippy `items_after_test_module` 截止；测试模块已移至文件末尾，并补充 Windows 目标静态检查，ConPTY 握手仍待下一轮 runner 证实。 |
| 2026-09-03 | REQ-20260903-004 | 进行中 | 第四次 Windows CI `33716538082` 显示夹具 `ready` 泄露到父 CI 控制台而非 ConPTY pipe；根因是 STARTUPINFO 标准句柄虽置空却遗漏 `STARTF_USESTDHANDLES`，Windows 因而复制父标准句柄。现已按生产级 ConPTY 做法补齐标志。 |
| 2026-09-03 | REQ-20260903-004 | 进行中 | 第五次 Windows CI `33717040377` 已通过 ConPTY 完整输入输出、Credential Manager、Job Object 和其余 Windows 运行测试；唯一失败是同一临时目录的 8.3 短路径与 verbatim 长路径字符串不相等。测试已改为比较规范化后的目录身份，等待最终复验。 |
| 2026-09-03 | REQ-20260903-004 | 进行中 | 最终 Windows CI `33717589098` 全绿，真实 runner 已验证 ConPTY 完整往返、Credential Manager、Job Object、受限进程和固定登录动作；Task 6 关闭，进入 Claude Code/OpenAI Codex 采集。 |
| 2026-09-03 | REQ-20260903-004 | 进行中 | Claude/Codex 跨平台解析 fixture 与 6 项失败先行测试完成：促销百分比、登录/信任提示、未知通知、额度窗口和 Reset Credit 均有明确口径，不再把解析失败显示为 0%。 |
| 2026-09-03 | REQ-20260903-004 | 进行中 | Claude 固定 `/usage` ConPTY 采集与 Codex initialize/account/rateLimits JSON-RPC 会话已接入受限进程边界；Codex 跨平台进程测试通过，Claude Windows-only 端到端 fixture 等待 runner。 |
| 2026-09-03 | REQ-20260903-004 | 进行中 | Windows CI `33718718700` 的 Claude Transport 定位为 ESM 测试夹具含非法顶层 `return`，已修正；同时完成 Claude/Codex 白名单本机活动与 300 秒并发刷新协调，本机 56 项 Rust 回归及零警告 Clippy 通过。 |
| 2026-09-03 | REQ-20260903-004 | 进行中 | Windows CI `33719315118` 进一步证明 Claude 夹具把 ConPTY 光标握手回复误当成唯一业务输入并提前退出；夹具改为累计终端输入直至固定 `/usage`，本机终端序列、56 项 Rust、5 项前端和正式构建通过，等待 runner 复验。 |
| 2026-09-03 | REQ-20260903-004 | 进行中 | Windows 前端浮动条、统一 Logo 圆环、三品牌色、丰富详情、Reset Credit、30 日本机统计、DeepSeek 历史占位、Settings tabs、字体目录、自动隐藏和点击外部关闭完成；安全桥只订阅固定脱敏事件，进入托盘与 Win32 窗口阶段。 |
| 2026-09-03 | REQ-20260903-004 | 进行中 | Windows CI `33720000356` 将 Claude 失败限定为测试替身输出后 100ms 退出与 ConPTY 读取竞争；替身改为持续交互并交由 Job Object 回收，等待 runner 复验。 |
| 2026-09-03 | REQ-20260903-004 | 进行中 | Win32 三窗口、动态托盘、左右镜像 S 曲线、拖动与显示器位置持久化、全屏隐藏、详情临时置顶和真实 Settings 同步已实现；本机门禁通过，等待 Windows runner 桌面壳构建及真机视觉验收。 |
| 2026-09-03 | REQ-20260903-004 | 进行中 | Windows CI `33722059809` 否定 Claude 替身退出竞态为主因；根因收敛为 Windows 规范化 `\\?\` 路径不应原样传给 Node/CMD 解释器，已增加本地/UNC 失败先行测试并在命令边界安全转换，等待 runner 复验。 |
| 2026-09-03 | REQ-20260903-004 | 进行中 | Windows CI `33722886734` 全绿，验证 Claude/Codex 采集、Credential Manager、GDI 区域和 Tauri 多窗口桌面壳；DeepSeek 30 日历史完成独立 WebView2、官方域名白名单、nonce 分片、严格脱敏聚合和详情图表，本机 73 项 Rust/9 项前端通过，等待 Windows 真机登录验收。 |
| 2026-09-03 | REQ-20260903-004 | 进行中 | Windows CI `33724498418` 在提交 `a80f72b` 上全绿，确认 DeepSeek 历史的 Windows 条件编译、桥接测试和完整 Tauri 壳均通过；真实账户登录后的官方网页兼容性仍保留为真机验收。 |
| 2026-09-03 | REQ-20260903-004 | 进行中 | 阶段审计发现库级采集器尚未接入应用生命周期，已新增明确计划项；启动缓存、启动/定时/托盘刷新、失败隔离、刷新世代防回写和详情实时更新已按测试先行实现，等待 Windows runner 编译运行后关闭该缺口。 |
| 2026-09-03 | REQ-20260903-004 | 进行中 | Windows CI `33725559642` 在功能测试前被 Windows-only Clippy 截止；根因是 `collectors::application` 同时在父模块与文件内重复声明 `cfg(windows)`，已按最小差异移除文件级重复属性并进入完整复验。 |
| 2026-09-03 | REQ-20260903-004 | 进行中 | Windows CI `33726552828` 全绿，确认真实应用生命周期、Windows-only Clippy、完整 Rust 运行测试和 Tauri 壳均通过；Task 7 关闭。Settings Services 已按 macOS 语义接通当前账号、CLI 来源/版本、固定登录命令和验证后替换 DeepSeek Key，等待下一轮 runner。 |
| 2026-09-03 | REQ-20260903-004 | 进行中 | Windows CI `33728220354` 已通过前端测试与构建，但 Services 命令的两个 Windows-only 分支被严格 Clippy 判定为多余 `return`；已按编译器建议做最小修正，等待完整 runner 复验及 NSIS 产物。 |
| 2026-09-03 | REQ-20260903-004 | 进行中 | Windows CI `33728660845` 全绿：Services、更新器、完整 Rust/前端测试及真实 NSIS 构建通过，上传的 x64 debug 安装器已重新下载并核对为有效 PE/NSIS，SHA-256 为 `80d38f505cea337a38a340cc8bdc7d96e058991454ed9244fe2479149ba5aef8`；进入双平台 Release 门禁与公开文档阶段。 |
| 2026-09-03 | REQ-20260903-004 | 进行中 | 双平台草稿 Release workflow、本机 macOS Keychain 编排入口与 Windows Tauri 签名资产/`latest.json` 门禁完成；用户、架构、隐私、安全、测试、维护和发布文档已同步。macOS 371 项回归、Release Bundle/Sparkle 严格签名、128 份 Markdown 与公开安全检查通过；Windows 真机截图、交互验收和签名升级演练仍未完成。 |
| 2026-09-03 | REQ-20260903-004 | 进行中 | 合并前独立审查发现三项发布阻断：WSL 官方额度会错误拼接 Windows 本机活动、DeepSeek Key 曾进入 WebView/IPC、Preview 被写入稳定 appcast 且 GitHub `releases/latest` 不解析 prerelease；同时登记取消未下传、版本同步门禁和详情窗口状态等重要缺口。当前明确保持不可合并，按失败先行测试逐项修复。 |
| 2026-09-03 | REQ-20260903-004 | 进行中 | 三项阻断已完成代码级修正：WSL 不再读取 Windows 活动，DeepSeek Key 改由非持久化 Win32 Credential UI 在 Rust 内接收，稳定/Preview appcast 与 Windows feed 分离；tagged workflow 增加版本同步门禁。前端、Rust、Swift 发布合同及静态安全检查通过，等待 Windows CI 编译原生分支后进入 Important 修复。 |
| 2026-09-03 | REQ-20260903-004 | 进行中 | 阻断修复已由 Windows CI `33732500779` 全绿验证。继续完成审查中的 Important 项：详情关闭状态与小工作区/DPI 自适应、Claude/Codex 独立 Auto/Native/WSL/自定义路径设置、刷新取消下传到 CLI/HTTP/活动扫描、发现资源上限，以及 Monitoring 的刷新周期、DeepSeek 基准、70%/90% 通知和登录启动；本机前端/Rust 定向测试与严格 Clippy 已通过，等待新提交的 Windows runner 复验。 |
| 2026-09-03 | REQ-20260903-004 | 进行中 | Windows CI `33734781055` 在严格 Clippy 的 Windows-only WSL 列表分支发现 `Vec<String>` 与受限进程边界 `Vec<OsString>` 类型不匹配，Rust 测试与 NSIS 因此未运行；已在命令边界逐项转换为 `OsString`，不放宽参数隔离，进入 runner 复验。同期 macOS 完整门禁为 360 项主测试、11 项独立 PTY、跨平台合同、128 份 Markdown 与公开安全检查全部通过。 |
| 2026-09-03 | REQ-20260903-004 | 进行中 | 独立复审发现上一份 Native Windows 本机活动仍可能被缓存层补到后续 WSL 快照；失败先行测试已稳定复现并证明根因为 `UsageRuntime` 对 `localActivity = nil` 的无条件回填。现改为成功刷新始终采纳当前来源的本机活动结果，仅 DeepSeek 独立历史继续保留，彻底阻止运行方式切换或重启后混数。CI `33735360341` 同时暴露 Windows-only `needless_return`，已按严格 Clippy 最小修正。 |
| 2026-09-03 | REQ-20260903-004 | 进行中 | 合并前 Important 修复继续完成：更新安装会暂停并排空采集，登录/采集使用同一受限 profile 环境，自定义 CLI 保存前真实验证；CLI 发现具备总时限/进程预算/取消，本机活动独立限时 2 秒并可中断 SQLite；meter 小屏 DPI 等比缩放，Settings 内容可滚动。失败先行定向测试、严格 Clippy、前端 14 项和 production build 通过，等待 Windows runner。 |
| 2026-09-03 | REQ-20260903-004 | 进行中 | Windows CI `33737903436` 已验证上述 Important 修复、完整 Rust/前端测试、严格 Clippy、Tauri 壳与 NSIS 全绿。继续补齐 WSL 当前发行版活动、可唤醒刷新间隔、数字输入草稿、真实 updater minisign 复验、稳定 appcast 延后公开和 SmartScreen Release Notes 门禁；仍需新一轮 Windows runner 与最终审查。 |
| 2026-09-03 | REQ-20260903-004 | 进行中 | 收尾改动已完成本机全量门禁：Windows Rust、严格 Clippy/rustfmt、前端 14 项与构建，macOS 360 + 11 项、共享合同、128 份文档、公开安全扫描和 9 段 Release Bash 语法均通过；等待提交后的 Windows CI 对 Windows-only 分支及 NSIS 做最终确认。 |
| 2026-09-03 | REQ-20260903-004 | 进行中 | 最终复审新增两项 Important：整数设置暂存小数时 UI 与持久化值不一致；Release 公开后 appcast/preview feed 失败缺少旧 feed 恢复与公开版本重试。暂缓合并，按失败先行测试实现输入 step 校验及跨 feed 补偿回滚。 |
| 2026-09-03 | REQ-20260903-004 | 进行中 | 第二轮复审发现回滚仍可能在 feed 恢复失败或已公开版本重跑失败时盲目撤下 Release，并会把鉴权/网络错误误判为 Preview feed 不存在。现已增加 draft→public 尝试标记、两份 feed 反向引用验证、只在安全条件成立时 redraft、明确 HTTP 404 探测及线上资产恢复重用；进入全量门禁与再次独立复审。 |
| 2026-09-03 | REQ-20260903-004 | 进行中 | 第三轮独立复审未发现 Critical/Important；macOS 360+11、Windows Rust 全套、前端 14 项/构建、严格 lint、发布事务、10 段 workflow Bash、128 份文档与公开安全门禁全部通过。准备提交并由 Windows runner 复验 Windows-only 编译、运行测试与 NSIS。 |
| 2026-09-03 | REQ-20260903-004 | 进行中 | 提交 `96e6a92` 的最终 Windows CI `33741425083` 全绿，Windows-only 运行测试、Release Tauri/NSIS 和安装器上传通过；三轮独立复审无剩余 Critical/Important，进入文档证据提交与 `main` 合并。真实 Windows 11 交互、截图及签名 Preview 升级仍保持环境限制。 |
| 2026-09-03 | REQ-20260903-004 | 进行中 | 精确合并头 `5288df1` 的 Windows CI `33742313609` 全绿，合并节点 `3920dae` 已把 Windows 平台并入 `main`。代码/审查/CI 阶段完成；需求继续保持进行中，下一阶段为交互式 Windows 11 真机视觉与功能验收、产品截图及签名 Preview 升级演练。 |
| 2026-09-04 | REQ-20260903-011 | 进行中 | 同步 `0.3.0-preview.0`/build 7 的根、macOS、npm、Cargo 与 Tauri 元数据，补齐 Release notes、发布日志和下载说明；废弃无密码临时密钥，改为密码保护的 Tauri minisign keypair。私钥尚未外传，等待用户明确授权写入两个 GitHub Actions Secret。 |
| 2026-09-04 | REQ-20260903-011 | 进行中 | 用户明确授权把本次密码保护的 Tauri Windows 更新签名私钥和密码写入 `sljzdotcom/AI-Token-Meter` 的 GitHub Actions Secrets；进入 Secret 配置、临时文件销毁和正式发布。 |
| 2026-09-04 | REQ-20260903-011 | 进行中 | 两个 Actions Secret 名称已核对存在，内容未回读；本机临时 keypair、密码和生成日志已全部删除。开始提交 `0.3.0-preview.0` 发布候选。 |
| 2026-09-04 | REQ-20260904-001 | 新建 → 进行中 | 首次双平台 Release workflow `33813010911` 中 macOS 门禁、Windows 前端、严格 Clippy 与 122 项 Rust 测试均通过；跨平台合同检查器随后仅因 Windows 文件系统的可执行位语义报告发布入口不可执行，签名构建尚未开始。 |
| 2026-09-04 | REQ-20260904-001 | 进行中 | 新增真实 Git 仓库回归，证明工作区权限与索引模式冲突时必须以 Git `100755`/`100644` 为准；检查器改读 Git 索引后正反场景及完整合同检查均通过，等待 Windows runner 复验。 |
| 2026-09-04 | REQ-20260904-002 | 新建 → 进行中 | 第二次 Release workflow `33814393456` 已通过 macOS 门禁、Windows frontend、严格 Clippy、122 项 Rust、跨平台版本同步并成功构建签名 NSIS；随后仅在 `Normalize and hash Windows assets` 中止，资产尚未上传或公开。 |
| 2026-09-04 | REQ-20260904-001 | 进行中 → 已完成 | 第二次 Windows Release workflow `33814393456` 已越过跨平台版本合同并完成后续签名构建，证明 Git 索引模式修复在 `windows-latest` 生效。 |
| 2026-09-04 | REQ-20260904-002 | 进行中 | 完整 Runner 日志证实 Tauri v2 产出一个 NSIS `setup.exe` 与同名 `.exe.sig`，没有旧流程假定的 `.nsis.zip`。新增跨平台 Node 标准化器及真实文件回归，统一安装、更新、签名验证、Release 与 `latest.json` 使用同一安装器；374+12 项 macOS 测试、合同、发布 feed、135 份文档和公开安全门禁通过，等待 Windows runner 复验。 |
| 2026-09-04 | REQ-20260904-002 | 进行中 → 已完成 | 提交 `704342e` 的 Release workflow `33816010376` 在 Windows Runner 完成签名 NSIS、标准化、SHA-256、内嵌公钥验签和 Artifact 上传；发布作业生成 `latest.json` 并公开同版本双平台资产。公网重下后再次通过 Windows minisign 与 SHA-256 验证。 |
| 2026-09-04 | REQ-20260903-011 | 进行中 → 已完成 | `v0.3.0-preview.0` 已公开为 prerelease；macOS Sparkle ZIP/篡改拒绝、Windows NSIS minisign、两份 SHA-256、Preview manifest 与固定 feed 均从公开 URL 重新下载验证通过，稳定 `appcast.xml` 仍保持 `0.2.2`。 |
| 2026-09-04 | REQ-20260903-004 | 进行中 | 首个签名双平台 Preview 已公开，Tauri Secret、Windows minisign、固定 Preview feed 和恢复门禁完成；剩余范围收敛为真实 Windows 交互验收、产品截图及下一版签名原位升级/错误签名拒绝。 |
| 2026-09-03 | REQ-20260903-008 | 新建 → 进行中 | 文档证据提交触发的 macOS CI `33744143766` 中，两条普通即时 SecretStore 读取均在默认 2 秒边界超时；同一源码此前通过且失败集中在低优先级 detached 读取，进入失败先行复现与优先级反转修复。 |
| 2026-09-03 | REQ-20260903-008 | 进行中 | 两条失败先行测试稳定记录读取优先级 `21 < 25`；两个读取器改用 `.userInitiated` 后，6 项聚焦行为和 362+11 项完整 macOS 回归通过，原有阻塞读取超时语义保持不变。等待提交后的双平台 main CI。 |
| 2026-09-03 | REQ-20260903-001 | 已完成 → 进行中 | `eea044b` 的 macOS CI `33744944628` 中 DeepSeek 相关 362 项主测试全部通过，但独立 PTY 测试的 `A parent exit cannot leave the runner waiting on a descendant PTY` 在 2 秒边界再次超时；进入 fallback wait QoS 调度证据与修复。 |
| 2026-09-03 | REQ-20260903-001 | 进行中 | 失败先行测试固定 fallback wait 必须使用用户级 QoS；提升专用队列后 PTY 12/12、父进程退出压力 20/20、完整 362+12 回归通过。等待新提交的 main CI。 |
| 2026-09-03 | REQ-20260903-001 | 进行中 → 已完成 | 精确提交 `2c6a194` 的 macOS CI `33745691851` 全绿；同提交 Windows CI `33745691724` 也通过完整运行测试与 NSIS 构建，低 QoS 退出确认复发已关闭。 |
| 2026-09-03 | REQ-20260903-008 | 进行中 → 已完成 | `eea044b` 已使 CI 中原失败的 DeepSeek 两条即时读取和全部 362 项主测试通过；合并 PTY 调度修复后的精确提交 `2c6a194` 再获 macOS 与 Windows 双平台 main CI 全绿。 |
| 2026-09-03 | REQ-20260903-009 | 新建 → 进行中 | 用户反馈浮动条在左侧摆放后，会在重启或运行一段时间后自行回到右侧；先核对现有持久化字段、屏幕标识及所有自动重排入口，定位实际覆盖点。 |
| 2026-09-03 | REQ-20260903-009 | 进行中 | 用户确认方案 A，并补充多显示器口径：主屏摆放不得自行跳到副屏；保存具体显示器位置；多屏变单屏时浮动条应回到当前主屏，同时不得用临时回退覆盖原位置。 |
| 2026-09-03 | REQ-20260903-009 | 进行中 | 稳定显示器 UUID、旧配置迁移、目标屏优先、主屏无损回退、重连恢复和仅用户操作写入的正式规格已完成并通过占位符、矛盾、范围与模糊性自检。 |
| 2026-09-03 | REQ-20260903-009 | 进行中 | 测试驱动实施计划已覆盖 macOS 身份/解析、控制器写入边界、Windows 对等合同、完整验证、文档和 Git/CI 检查点；隔离 worktree 基线 362 项主测试、12 项 PTY 及全部门禁通过。 |
| 2026-09-03 | REQ-20260903-009 | 进行中 | macOS 已改用 Core Graphics 稳定 UUID，系统重排不会覆盖保存目标；旧数字配置只迁移标识，目标断开时保留原侧边和高度并临时回当前主屏。位置相关 31 项及 Windows monitor 合同 4 项通过，进入全量门禁。 |
| 2026-09-03 | REQ-20260903-009 | 进行中 | 首轮独立审查无 Critical，发现三项 Important：macOS Settings 改边未建立当前屏目标；Windows 尚无运行中拓扑变化监听；Windows `MONITORINFOEX.szDevice`/几何回退并非稳定物理身份。暂缓合并并进入补充 TDD。 |
| 2026-09-03 | REQ-20260903-009 | 进行中 | 审查三项 Important 已测试先行补齐：macOS Settings 主动改边会把当前解析屏保存为目标；Windows 以 Win32 设备接口路径的 SHA-256 标识物理屏幕，兼容旧运行时名称迁移，并以运行时拓扑监听实现断开回退与重连恢复。macOS 策略 5 项、Windows monitor 7 项及本机严格 Clippy/rustfmt 通过，进入全量验证。 |
| 2026-09-03 | REQ-20260903-009 | 进行中 | 二次独立审查要求继续处理三个 Windows 韧性边界：接口恢复后将 `runtime:` 安全降级标识迁移为 `device:`；旧标识迁移持久化失败不得中止 App；拓扑初始枚举与定位瞬时失败不得永久停止监听或提前提交状态。保持未完成并补失败先行测试。 |
| 2026-09-03 | REQ-20260903-009 | 进行中 | 二次审查问题已修复：`runtime:` 作为迁移别名支持接口恢复升级；标识迁移以候选副本事务式持久化且失败不阻断启动；拓扑监听只在成功定位后提交状态并对初始枚举/定位失败持续重试。非主目标、主屏角色、工作区变化、陈旧写入与磁盘错误均有回归测试，Windows 完整 122 项通过。 |
| 2026-09-03 | REQ-20260903-010 | 新建 → 进行中 | PR #4 macOS CI 首次运行及失败重跑均在 `blockedSecretReadTimesOut` 复现：高优先级阻塞读取先于协作式超时任务返回，导致迟到 API Key 发起网络请求。登记为独立韧性缺陷并按系统化调试处理。 |
| 2026-09-03 | REQ-20260903-010 | 进行中 | 用独立 GCD 单调时钟队列替代协作式 `Task.sleep` 截止时间，并让 DeepSeek 用量采集与 Settings 凭据状态共用实现；两个 200 ms 阻塞读取均在约 20 ms 安全降级，386 项 macOS 测试及全部文档/公开安全门禁通过。 |
| 2026-09-03 | REQ-20260903-009 | 进行中 → 已完成 | 三轮独立审查关闭全部 Critical/Important；PR #4 的 macOS `33766095915` 与 Windows `33766096437` 通过，合并提交 `c2d2e64` 的 main CI `33766955625`、`33766955622` 再次全绿。稳定物理屏身份、无损主屏回退与重连恢复已交付。 |
| 2026-09-03 | REQ-20260903-010 | 进行中 → 已完成 | 提交 `0ae1cbe` 修复截止时间饥饿，`4f6c3af` 完成复审术语更正；阻塞回归、本机 386 项与 macOS PR/main CI 均通过，原失败未复现。 |
| 2026-09-03 | REQ-20260903-011 | 新建 → 进行中 | 用户要求继续完成位置修复后的可安装交付；按既定跨平台规则选择首个 `0.3.0-preview.0`，要求 macOS/Windows 同版本、同 Release、各自签名更新清单和完整文档。 |
| 2026-09-04 | REQ-20260904-003 | 新建 → 已完成 | M4 Max 的 `SUFeedURL` 固定读取仓库根稳定 `appcast.xml`，该 feed 最新条目仍为 `0.2.2`；`0.3.0-preview.0` 是公开 prerelease，发布流程有意不改写稳定 feed，因此应用内检查不到属于预期的通道隔离，并非架构或签名故障。 |
| 2026-09-04 | REQ-20260904-004 | 新建 → 进行中 | Windows `0.3.0-preview.0` 真机截图显示浮动条窗口透明区被白色背景暴露、上下出现白条，轮廓存在明显锯齿且缺少已确认的反向半圆；用户同时报告贴边行为不稳定。进入窗口透明合成、裁剪几何、DPI 与显示器定位链路的系统化调试。 |
| 2026-09-04 | REQ-20260904-004 | 进行中 | 根因已收敛：Windows 无边框窗口默认阴影产生 1px 白色边框；CSS 粗粒度多边形与 Win32 整数 GDI Region 双重裁剪且几何不一致，导致白条、锯齿和肩部缺失；网页 `startDragging()` Promise 被误当成可靠拖动结束信号，且 750ms 显示器监控可在拖动期间重排窗口。用户确认采用方案 A——Windows 精确复用 macOS Bezier 轮廓，macOS 保持不变，并授权完成后发布。 |
| 2026-09-04 | REQ-20260904-004 | 进行中 → 待用户确认 | `0.3.0-preview.1` 已由标签提交 `9213fd6` 发布；workflow `33826484923` 的 Windows 原生构建、macOS 标签核验和同步发布全部通过。公开重下后 macOS Sparkle、Windows minisign、两平台 SHA-256、Preview 清单一致性和稳定 appcast 隔离均通过；自动化不替代 Windows 真机 DPI、轮廓和拖动验收。 |
| 2026-09-04 | REQ-20260904-005 | 新建 → 进行中 | 用户在 Windows `0.3.0-preview.1` 真机截图中反馈：点击浮动条后出现带 Edge 浏览器框架、标题为 “AI Token Meter” 的巨大黑色空白窗口，详情卡片未渲染；进入点击路由、窗口创建参数和启动 URL 的系统化调试。 |
| 2026-09-04 | REQ-20260904-005 | 进行中 | 用户更正：巨大空白 Edge 窗口在运行程序时自动弹出，并非点击详情产生。撤销点击路由假设，调查范围改为启动窗口清单、WebView2 安装/启动行为和自动浏览器副作用。 |
| 2026-09-04 | REQ-20260904-005 | 进行中 | 重新识别截图后确认窗口是 Windows Terminal（标签栏含加号/下拉入口），不是 Edge/WebView。Release 入口 `windows/src-tauri/src/main.rs` 缺少 `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]`，使生产可执行文件仍使用 console subsystem；根因与“主界面正常但同时出现空终端”完全一致。 |
| 2026-09-04 | REQ-20260904-005 | 进行中 | 采用编译时 GUI subsystem 方案：Windows 所有构建不分配控制台，CI/Release 直接解析实际 `.exe` 的 PE Optional Header 并要求 subsystem `2`；拒绝启动后隐藏终端和无关的 WebView 调整。下一 Preview 同步为 `0.3.0-preview.2`，macOS 只同步版本。 |
| 2026-09-04 | REQ-20260904-005 | 进行中 | TDD 红灯已由 Windows workflow `33830008532` 复现：前端测试、构建、Rust 格式、Clippy、运行时测试及 NSIS 构建全部通过，新增 PE 门禁读取真实 `ai-token-meter-windows.exe` 得到 subsystem `3` 并按预期失败。随后在 Windows 主入口加入 GUI subsystem 声明，等待同一门禁转绿。 |
| 2026-09-04 | REQ-20260904-005 | 进行中 | 修复后 Windows workflow `33831023542` 全绿，真实 PE 门禁报告 subsystem `2`；macOS workflow `33831023523` 同时通过。进入同步 `0.3.0-preview.2`（build 9）版本、文档、完整门禁与公开双平台发布阶段；最终状态仍需 Windows 真机启动确认。 |
| 2026-09-04 | REQ-20260904-005 | 进行中 → 待用户确认 | PR #5 合并提交与 `v0.3.0-preview.2` 标签均为 `c3cba89`；Release workflow `33833843964` 的 Windows 签名构建、PE GUI subsystem 门禁、macOS 标签校验及同步发布全绿。公网重下的 macOS/Windows SHA-256、Sparkle、Tauri minisign、Preview feed 与稳定 appcast 隔离全部复验通过；只保留 Windows 真机启动无 Terminal 的用户确认。 |
