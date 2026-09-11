# 开发日志索引

- [2026-09-06：紧凑浮动条与刷新韧性](2026-09-06-compact-progressive-strip.md)：双平台偏好、折叠、状态、退避、独立审查与待补实机验收。

开发日志用于记录每个开发阶段的目标、证据、关键决定、测试结果、安装验收和 Git 节点。它与 `CHANGELOG.md` 的区别是：

- `CHANGELOG.md` 面向使用者，按发布版本总结可见变化；
- 开发日志面向维护者，记录过程、失败证据和技术取舍；
- [提交历史](commit-history.md) 面向仓库审计，列出实际 Git 提交。

## 日志

| 日期 | 内容 | 结果 |
| --- | --- | --- |
| [2026-09-11](2026-09-11-v0.8.2-release.md) | 0.8.2 悬浮条即时密度与轮廓修复稳定发布 | Settings密度立即调整已展开原生窗口，左右镜像圆角与反向肩弧；双平台签名资产和公网更新源已验收 |
| [2026-09-11](2026-09-11-floating-strip-live-density-left-contour.md) | 悬浮条密度即时生效与方案 B 轮廓修正 | 已展开密度切换立即调整原生窗口；左右镜像圆角与30pt/px反向肩弧通过双平台专项、完整门禁和真实Chrome验证，已随0.8.2公开 |
| [2026-09-11](2026-09-11-v0.8.1-release.md) | 0.8.1 浮动条轮廓与设置分类稳定发布 | 14pt内凹把手、镜像轮廓、底部Settings入口移除与第五个设置页签；发布证据随事务补记 |
| [2026-09-11](2026-09-11-floating-strip-settings-tab.md) | Settings新增Floating Strip/悬浮条页签并精简Appearance | 双平台五页签、三组悬浮条设置、右键路由和中英文已实现；偏好语义保持，完整门禁与独立复核见日志 |
| [2026-09-11](2026-09-11-floating-strip-contour-simplification.md) | 浮动条镜像轮廓与底部Settings入口移除 | 双平台14pt内凹收起把手、镜像肩部与内容区高度；底部弧、齿轮及命中区完全移除，完整门禁与独立复核0/0/0通过 |
| [2026-09-11](2026-09-11-v0.8.0-release.md) | 0.8.0 Mini浮动条稳定发布 | 三档浮动条、独立自动收起与Settings底弧修正；PR/main、双平台签名资产和公网更新源已验收 |
| [2026-09-11](2026-09-11-mini-floating-strip-followup.md) | Mini浮动条、自动收起与Settings底弧修正 | 新增65pt Mini、恢复78pt Compact、三档固定顺序并隔离底弧齿轮触发；双平台门禁、独立审查与0.8.0发布完成 |
| [2026-09-11](2026-09-11-v0.7.3-release.md) | 0.7.3浮动条交互与Antigravity视觉发布 | 65pt贴边浮动条、悬停双延迟、Settings入口与强调色恢复；双平台签名资产和公网更新源已验收 |
| [2026-09-10](2026-09-10-floating-strip-hover-redesign.md) | 65pt贴边浮动条与悬停交互 | 双平台新轮廓、内容安全过渡、快捷Settings与双延迟偏好 |
| [2026-09-10](2026-09-10-antigravity-accent-restoration.md) | Antigravity详情强调色恢复 | 青绿色标题、额度值与进度恢复，深海底板及数据语义保持 |
| [2026-09-10](2026-09-10-v0.7.2-release.md) | 0.7.2 Antigravity详情与Compact稳定修复发布 | 版本、完整门禁、PR/main、签名资产和公开更新源按发布事务留证 |
| [2026-09-10](2026-09-10-compact-strip-padding-halving.md) | Compact圆环两侧留白减半 | 双平台Compact宽度65→56.5，48圆环与纵向尺寸保持；专项原生、浏览器及同尺度截图通过 |
| [2026-09-10](2026-09-10-antigravity-detail-surface-fix.md) | Antigravity详情底板修复 | macOS恢复统一深色底板；Windows四Provider真实浏览器对等性通过 |
| [2026-09-10](2026-09-10-windows-frontend-test-reliability.md) | Windows发布前端测试文件调度可靠性 | 保留5秒单项上限并串行jsdom文件；PR/main/标签Windows原生门禁及0.7.1公开发布通过 |
| [2026-09-10](2026-09-10-v0.7.0-release.md) | 0.7.0 → 0.7.1 Google Antigravity与紧凑界面稳定发布 | 0.7.0首次候选未公开；0.7.1/build20已公开，七项资产、签名与三个更新源验收通过 |
| [2026-09-10](2026-09-10-compact-strip-width.md) | Compact浮动条横向收窄 | 用户已选65宽；PR #26已合入`1c0a1ed`并随0.7.1/build20公开交付 |
| [2026-09-10](2026-09-10-menu-panel-logo-enlargement.md) | 菜单面板顶部 Logo 放大 | macOS标题区应用Logo由32pt放大25%至40pt，其他图标、字号、间距及Windows保持 |
| [2026-09-10](2026-09-10-conpty-fixture-reliability.md) | Windows ConPTY测试夹具稳定性 | 保留真实ConPTY往返及3秒期限，用Cargo原生夹具消除外部Node冷启动波动；PR与main门禁全绿 |
| [2026-09-10](2026-09-10-codex-app-server-fixture-reliability.md) | Windows Codex app-server测试夹具稳定性 | 保留产品10秒期限和完整JSON-RPC断言，用Cargo原生夹具消除外部Node冷启动波动；PR与main门禁全绿 |
| [2026-09-10](2026-09-10-antigravity-cli-migration.md) | Google Antigravity CLI 迁移 | 第四项保留兼容 ID，双平台改用受限 `agy -p /usage` 和四窗口展示；已合入main，尚未发布 |
| [2026-09-09](2026-09-09-codex-timeout-pid-capture.md) | Codex 超时进程取证稳定性 | 父进程启动点直接记录 PID，保留超时与真实进程退出断言；完整门禁与发布恢复继续留证 |
| [2026-09-09](2026-09-09-v0.6.3-release.md) | 0.6.3 macOS About 精简稳定发布 | 版本/build18、完整门禁、原生CI、签名资产与三个公开更新源按顺序留证 |
| [2026-09-09](2026-09-09-macos-about-author-line-removal.md) | macOS About 独立作者行移除 | 仅精简可见摘要，版本、三条社交链接与开源归属保持；完整门禁和发布状态见记录 |
| [2026-09-09](2026-09-09-v0.6.2-release.md) | 0.6.2 菜单面板 Logo 双平台稳定发布 | 版本/build17、完整门禁、原生CI、签名资产与三个公开更新源按顺序留证 |
| [2026-09-09](2026-09-09-menu-panel-app-logo.md) | 双平台菜单面板应用 Logo | 本地main已整合；macOS真实面板渲染、Windows原生菜单项、491项Swift、124项前端、261项Rust及发布构建通过，暂未发布新版本 |
| [2026-09-09](2026-09-09-v0.6.1-release.md) | 0.6.1 四服务接入修复双平台稳定发布 | 版本/build16、完整门禁、双平台签名资产与公开更新源按顺序留证 |
| [2026-09-09](2026-09-09-four-provider-onboarding-consistency.md) | 四服务商新用户接入与详情恢复一致性 | PR #17合入main；490项Swift、124项前端、259项Rust、双平台原生CI与独立复核0/0/0通过 |
| [2026-09-08](2026-09-08-v0.6.0-release.md) | 0.6.0 Gemini 与累积改进双平台稳定发布 | 已公开；485项Swift、256项原生Windows Rust、双平台签名资产与三个更新源验证通过 |
| [2026-09-08](2026-09-08-gemini-collector.md) | Gemini CLI额度采集接入 | 已随0.6.0发布；自动化与原生Windows门禁通过，真实账号仍受环境限制 |
| [2026-09-08](2026-09-08-gemini-presentation.md) | Gemini四产品展示、配置迁移与集成回归 | 四按钮与迁移阶段通过；采集阶段见上一条，012仍受环境限制 |
| [2026-09-08](2026-09-08-gemini-cli-capability.md) | Gemini CLI 官方额度命令与渲染隔离探测 | /model 合成完整启动链通过；headless stats 会触发模型边界，真实账号与Windows待验 |
| [2026-09-08](2026-09-08-remove-strip-drag-hint.md) | 两端展开态顶部横线移除 | 保留背景拖动、Provider点击和折叠态竖线；列入 0.6.0 |
| [2026-09-08](2026-09-08-macos-refresh-interval.md) | macOS 五分钟用量刷新间隔不可修改 | 补原生编辑、保存及重排计时；列入 0.6.0 |
| [2026-09-08](2026-09-08-about-telegram.md) | 关于页精简与 Telegram 链接 | Windows 删除可见作者文字，两端增加固定 Telegram 入口；列入 0.6.0 |
| [2026-09-08](2026-09-08-windows-update-notice.md) | Windows 关于页新版提示强调 | 深红色加粗，仅 available 状态；列入 0.6.0 |
| [2026-09-08](2026-09-08-v0.5.1-release.md) | Windows CLI 修复补丁 0.5.1 | 已公开 0.5.1/build14；标签双平台测试、公开签名/哈希/篡改拒绝与三个更新入口验证通过 |
| [2026-09-08](2026-09-08-windows-cli-post-login.md) | Windows 登录成功但额度未显示 | 修复已合并 PR #12，229 项原生 Windows Rust/双平台 CI 通过；真实账号额度保留现场验收 |
| [2026-09-07](2026-09-07-v0.5.0-release.md) | 0.5.0 CLI 引导与 About 品牌展示双平台稳定发布 | 已公开；双平台签名、432 项 Swift/218 项原生 Windows Rust、公开资产与三个更新源验证通过 |
| [2026-09-07](2026-09-07-about-branding.md) | 在线安装包边界核对、关于页社交链接与 Windows 软件图标 | 已合并，432 项 Swift、218 项原生 Rust、双平台 CI 通过；列入 0.5.0 |
| [2026-09-07](2026-09-07-cli-onboarding.md) | CLI 安装/账户状态引导与 Windows Settings 页签图标 | 已随 0.5.0 发布；该阶段 macOS 429、Windows 原生 Rust 216、前端 75，双平台 CI/构建通过 |
| [2026-09-07](2026-09-07-v0.4.0-release.md) | 0.4.0 双平台稳定更新发布 | 已完成；Tag `15d80e2`、workflow `34076547278`、appcast `2d2254f`，公网双平台资产、签名及三个更新源一致 |
| [2026-09-07](2026-09-07-multidisplay-and-windows-localization.md) | 多显示器、跨屏拖动与 Windows 本地化 | 已实现并合并 PR #9；双平台 CI 通过，物理多屏/DPI 验收独立追踪 |
| [2026-09-06](2026-09-06-v0.3.0-release.md) | `0.3.0` 双平台正式更新通道发布 | 已完成；Tag `bb215c3`、workflow `34035797098`、公开资产/两端签名/三个更新源一致；旧 macOS Preview 实际检查发现新版 |
| [2026-09-06](2026-09-06-codenotch-competitive-review.md) | CodeNotch 竞品研究：紧凑视觉、渐进式展开、运行状态、数据可信度与快捷交互 | 已完成；对照公开 README、设计规格、关键源码和本项目现有尺寸，形成 P0/P1/P2 建议；未直接修改产品界面 |
| [2026-09-04](2026-09-04-v0.3.0-preview.3-release.md) | `0.3.0-preview.3` 双平台发布与 Windows Preview 更新源推进 | 已完成；Tag `dac10b9`、workflow `33887131319`、公开双平台资产、两套更新签名、公网重下与固定 Windows Preview feed 全部通过 |
| [2026-09-04](2026-09-04-windows-deepseek-history-and-density.md) | Windows DeepSeek 显式官网同步、托管窗口生命周期、同步反馈和紧凑界面密度 | 独立复审无 Critical/Important；Windows 43 项前端、12 项浏览器生命周期、169 项 Rust、production Chrome 密度、双平台 PR CI 与 NSIS 通过；真实 Windows 11 登录、关闭、聚焦和字体下拉待用户确认 |
| [2026-09-04](2026-09-04-windows-console-window-suppression.md) | Windows 启动空白 Terminal 的 PE subsystem 根因、编译期修复与真实产物门禁 | subsystem `3 → 2` 红绿门禁、双平台 CI、`v0.3.0-preview.2` 公开发布、公网资产复验与 Windows 11 真机启动确认通过 |
| [2026-09-04](2026-09-04-windows-floating-strip-parity-fix.md) | Windows 浮动条轮廓、白边和拖动稳定性修复及 `v0.3.0-preview.1` 发布 | 自动化、原生 Windows workflow、公开双平台资产与 Preview 更新通道通过；`preview.1` 已发布，真机 DPI/多屏/拖动及 `preview.0 → preview.1` 原位升级仍待用户确认 |
| [2026-08-28 至 2026-08-30](2026-08-28-development-log.md) | 初始架构、三服务采集、菜单栏与悬浮条、详情自动隐藏、Claude 隔离工作区、用量准确性、Codex 重置额度、DeepSeek 30 天图表 | 100 个测试通过，真实 CLI 3/3，通过构建、签名和本机验收 |
| [2026-08-31](2026-08-31-deepseek-focus.md) | DeepSeek 内嵌官网登录输入焦点修复 | 103 个测试、构建、签名、输入焦点与真实登录验收通过 |
| [2026-08-31](2026-08-31-codex-deepseek-details.md) | Codex 额度优先详情与本机统计；DeepSeek 30 天自动同步兼容 | 110 个测试、Codex 真实集成、DeepSeek 官网聚合、release 构建和安装校验通过 |
| [2026-08-31](2026-08-31-codex-reset-credit-card.md) | Codex 重置券分层卡片与自适应详情高度 | 113 个测试、双券视觉渲染、release 构建和安装校验通过 |
| [2026-08-31](2026-08-31-visual-system-edge-docking.md) | 左右贴边浮岛、拖动位置记忆、统一玻璃详情、Logo 光学校正与仪表指针 App Icon | 功能与打包节点已完成，最终真实界面和安装验收记录见日志 |
| [2026-08-31](2026-08-31-floating-strip-deep-sea-background.md) | 贴边浮岛黑蓝「深海波纹」背景、左右镜像与玻璃回退 | 156 个测试、Release 构建、签名、安装指纹、左右贴边和辅助功能实机验收通过 |
| [2026-08-31](2026-08-31-display-font-selection.md) | System Default、Antonio、DIN Condensed 全局显示字体、缺失回退与恢复默认 | 170 个测试、Release 构建/签名/安装指纹、Settings 切换和持久化通过；菜单与三个详情的字体视觉验收待手工完成，当前机器最终为 Antonio、右侧 97% |
| [2026-09-01](2026-09-01-floating-strip-desktop-layer-and-background-crop.md) | 浮动条/详情桌面层、Space 关闭详情、深海背景等比覆盖上下肩部 | 179 个测试、Release 构建/签名/安装指纹、普通/全屏 Edge 层级、Space 关闭详情、左右肩部和偏好保持通过；Mission Control、左右两个普通 Space、真实指针拖动和多显示器待人工环境补验 |
| [2026-09-01](2026-09-01-settings-font-isolation-and-content-size-step.md) | Settings 系统字体隔离、内容文字精确 `+1pt` 与 SF Symbol 语义基线 | 187 个测试、Release 构建/签名/安装指纹及 Settings 实机切换/持久化通过；caption2/body/ContentUnavailable 的 Symbol 映射与渲染回归已覆盖；菜单点击面板与 Claude/Codex 非激活详情的像素级字体和截断仍待可全屏捕获环境补验 |
| [2026-09-01](2026-09-01-settings-tabs-and-brand-migration.md) | Settings 四分类 Tab、AI Token Meter 品牌与兼容迁移 | 196 个测试、41 个测试组、Release 构建/签名/arm64、安装哈希和四 Tab 实机验收通过；已合入 `main` |
| [2026-09-01](2026-09-01-widgetkit-extension.md) | 原生 Small/Medium/Large Widget、脱敏 App Group 快照、时间线与条件签名打包 | 224 个测试、48 个测试组、Widget target 编译和无 Widget release 构建通过；当前无 Apple Development 身份，真实 Gallery/桌面验收明确待补 |
| [2026-09-01](2026-09-01-service-account-relogin.md) | 三服务账户常驻状态、Claude/Codex 官方 CLI 重新登录、DeepSeek 两阶段安全换 Key、需求台账机制 | 265 个测试、55 个测试组通过；Release 构建、安装与真实 Settings 验收记录见日志 |
| [2026-09-01](2026-09-01-detail-panel-frontmost.md) | 点击 Provider 后让临时详情位于普通应用窗口上方，同时保留浮岛桌面层 | 268 个测试、55 个测试组、Release 签名/安装哈希及已安装窗口实时层级验收通过 |
| [2026-09-01](2026-09-01-claude-detail-local-activity.md) | Claude 官方额度优先详情与本机 Claude Code 最近 30 天活动 | 295 个测试、58 个测试组、Release 签名/安装哈希、真实聚合数据与自动隐藏验收通过 |
| [2026-09-01](2026-09-01-display-font-catalog-expansion.md) | 显示字体扩展至八项、别名解析、安装检测与安全回退 | 295 个测试、字体资源扫描、真实 Settings 八项菜单和本机字体状态验收通过 |
| [2026-09-02](2026-09-02-claude-detail-card-removal.md) | Claude 详情移除 Token composition 与 Top models，保留额度、三项本机统计和每日趋势 | 294 个测试、58 个测试组、Release 签名、安装哈希与自动隐藏实机验收通过 |
| [2026-09-02](2026-09-02-claude-detail-privacy-note-removal.md) | Claude 详情移除底部隐私说明及锁图标，底层隐私边界不变 | 295 个测试、58 个测试组、Release 签名、安装哈希和真实辅助功能树验收通过 |
| [2026-09-02](2026-09-02-menu-bar-quantum-dial.md) | 菜单栏 18×18pt 动态 Quantum Dial、同源最高比例和精确百分比 | 299 个测试、59 个测试组、双外观像素渲染、Release 签名、安装哈希和真实刷新链路验收通过 |
| [2026-09-02](2026-09-02-provider-visible-name-standardization.md) | Claude Code、OpenAI Codex、DeepSeek 当前用户可见名称统一与兼容边界 | 304 个测试、60 个测试组、主应用 Release 严格签名、安装哈希和真实辅助功能验收通过；Widget Release target 通过，桌面安装仍受既有证书事项限制 |
| [2026-09-02](2026-09-02-menu-bar-icon-visibility.md) | Quantum Dial 菜单栏模板着色修复 | 305 个测试、60 个测试组、Release 严格签名、安装哈希、运行时刷新与用户真实菜单栏视觉确认通过 |
| [2026-09-02](2026-09-02-project-retrospective.md) | 全仓库复盘、文档单一来源、设计资料统一、自动一致性检查与本地残留清理 | 308 个测试、61 个测试组、文档检查、Release 构建/签名与清理证据见日志 |
| [2026-09-02](2026-09-02-macbook-arm64-package.md) | MacBook Pro M4 Max Apple Silicon 分发包 | 308 个测试、arm64 Release、ZIP 完整性、解压后严格签名和 SHA-256 验证通过 |
| [2026-09-02](2026-09-02-portable-resource-crash-fix.md) | 修复跨 Mac 的 SwiftPM 资源装载崩溃并重新发布 0.1.1 | 312 个测试、标准资源门禁、解压后隐藏构建目录启动、签名与 SHA-256 验证通过 |
| [2026-09-02](2026-09-02-codex-cli-discovery.md) | 修复 Finder 环境下 nvm Codex 误报未安装并重新发布 0.1.2 | 318 个测试、nvm Node shebang、桌面 App 后备、Release 与跨目录分发验收通过 |
| [2026-09-02](2026-09-02-public-github-release.md) | MIT 开源、作者信息、标准社区文档、脱敏截图、完整历史安全扫描与 v0.1.2 GitHub Release | 331 个测试、67 个测试组、公开 CI、正式标签、双资产 Release 和匿名 SHA-256 下载终验全部通过 |
| [2026-09-02](2026-09-02-github-app-update.md) | Settings 手动检查与安装 GitHub 稳定版、Sparkle EdDSA 信任链和可复现发布入口 | 360 个测试、70 个测试组、Release/Sparkle/归档/篡改门禁和 0.1.9 → 0.2.0 隔离真实更新通过 |
| [2026-09-03](2026-09-03-pty-allocation-race.md) | macOS 高并发 `openpty` 分配竞争修复 | 修复前 32 路回归可复现；修复后聚焦压力与 360 个完整测试通过 |
| [2026-09-03](2026-09-03-ci-pty-exit-race.md) | GitHub runner 高负载下 PTY 退出回调、输出尾部与测试隔离修复 | 第二次复发已定位并关闭；20/20 压力、12 项 PTY 回归及双平台 main CI 通过 |
| [2026-09-03](2026-09-03-update-status-window-frontmost.md) | Sparkle 安装状态窗口自动置前与 v0.2.2 公开升级终验 | 362 项回归、最终 CI 33702415007、公开资产/EdDSA/严格签名和隔离 0.2.1 → 0.2.2 原位升级与自动重启通过 |
| [2026-09-03](2026-09-03-windows-platform.md) | Windows 11 x64 Tauri/Rust/React 版本与双平台同步发布 | 核心平台能力已交付，`preview.0` / `preview.1` / `preview.2` 均已公开发布并通过发布 workflow；交互式 Windows 真机 DPI/多屏/拖动、签名升级演练及 `REQ-20260904-006` DeepSeek/字体下拉验收仍待完成 |
| [2026-09-03](2026-09-03-deepseek-secret-read-priority.md) | 高并发 macOS CI 中 DeepSeek Keychain 读取的优先级反转修复 | 失败先行测试、374 项完整回归及双平台 main CI 通过 |
| [2026-09-03](2026-09-03-floating-strip-placement-persistence.md) | 稳定保存浮动条的物理显示器、侧边和高度，多屏断开时无损回退主屏 | 三轮独立审查无剩余 Critical/Important；macOS/Windows PR 与精确合并头 CI 全绿，已合入 `main` |
| [2026-09-03](2026-09-03-deepseek-timeout-starvation.md) | DeepSeek Keychain 阻塞读取不再饿死截止时间 | 失败重跑复现、独立单调时钟修复、386 项回归及 macOS PR/main CI 通过 |
| [2026-09-04](2026-09-04-v0.3.0-preview.0-release.md) | 首个 macOS/Windows 同版本 Preview Release | 已完成；Actions Secret、双平台门禁、公开 prerelease、两平台签名与公网重下复验均有证据 |
## 新日志模板

新增日志时使用 `YYYY-MM-DD-short-topic.md`，至少包含：

1. 背景与目标；
2. 影响范围；
3. 失败或问题证据；
4. 实现与关键决定；
5. 自动化验证；
6. 本机/界面验收；
7. 安全与隐私检查；
8. Git 提交或合并节点；
9. 已知限制与后续工作。

日志不得保存 API Key、OAuth Token、Cookie、授权头、未经去敏的完整账户响应或个人身份信息。
