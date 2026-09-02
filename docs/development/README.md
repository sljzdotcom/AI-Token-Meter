# 开发日志索引

开发日志用于记录每个开发阶段的目标、证据、关键决定、测试结果、安装验收和 Git 节点。它与 `CHANGELOG.md` 的区别是：

- `CHANGELOG.md` 面向使用者，按发布版本总结可见变化；
- 开发日志面向维护者，记录过程、失败证据和技术取舍；
- [提交历史](commit-history.md) 面向仓库审计，列出实际 Git 提交。

## 日志

| 日期 | 内容 | 结果 |
| --- | --- | --- |
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
| [2026-09-03](2026-09-03-ci-pty-exit-race.md) | GitHub runner 高负载下 PTY 退出回调、输出尾部与测试隔离修复 | 361 项完整回归、11 项聚焦测试和连续 10 轮压力复验通过；完整 CI 证据待回填 |

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
