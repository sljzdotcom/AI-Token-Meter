# Windows 语言、字体和详情密度实现计划

> **面向 AI 代理的工作者：** 使用测试驱动流程逐任务实现；通过 dispatching-parallel-agents 与 macOS 独立分工，单一 Windows 实现者负责共享文件。

**目标：** 提供可持久化的中英文选择、三个中文字体和统一更小的详情字号。

**架构：** Rust 设置保存 locale 和 displayFont，事件广播给所有窗口；前端集中字典及格式化工具，原生 Rust 消息使用同语义翻译键。Settings 字体隔离继续保留。

**技术栈：** React/TypeScript、CSS、Rust/Tauri、Vitest。

## 任务 1：持久化与字体目录

文件：windows/src-tauri/src/persistence/settings.rs；新增 windows/src/localization.ts、windows/src/displayFonts.ts 及同名测试。

- [x] 写旧设置保留 Antonio、缺失字体默认 Microsoft YaHei、未知 locale 回退 en 测试；cargo test 观察红灯。

~~~rust
// 旧用户选择保持，新安装默认值单独验证。
assert_eq!(settings.display_font, "Antonio");
assert_eq!(AppSettings::default().display_font, "Microsoft YaHei");
~~~

- [x] 实现 locale en/zh-CN 与字体增补、校验和即时事件；字体存在性检测采用真实字体可用性查询，CSS 回退不等于安装成功。
- [x] 测试存取/非法值/缺失字体结果并提交。

## 任务 2：界面与原生文本本地化

文件：windows/src/Shell.tsx、settings/SettingsWindow.tsx、详情组件；windows/src-tauri/src/platform/windows/tray.rs、lib.rs、collectors/application.rs。

- [x] DOM 测试切换为简体中文后 Settings 标题、动作、详情状态改变且现有偏好保持；字典键集合必须完整。
- [x] 集中翻译与 locale 日期数字工具，组件不得按翻译文本判断状态；后端更新已有托盘/上下文菜单并翻译自有通知/错误。
- [x] 验证所有窗口语言立即同步，字体仅用于内容；第三方 CLI/官网保持原文。
- [x] 运行前端、Rust 和构建，提交 3312537；后续 a8d7a0f 关闭迟到设置及生成标签问题。

## 任务 3：详情缩小一号

文件：windows/src/styles.css、详情字体组件与浏览器样式测试。

- [x] 在真实渲染的三个详情里记录各文本角色计算字号，再断言新值为原值减 1；同时断言 Settings 和浮动条字号不变，先见红灯。
- [x] 每个详情字体 token/显式 CSS 大小减 1px，消除跳过继承规则的局部硬编码；不得使用 zoom。
- [x] 验证中文/英文、三种新字体与原 Antonio，空态/错误态/图表标签不裁切，提交 3312537；632 项计算样式通过。

## 任务 4：交付验证

- [x] npm test、npm run build、cargo fmt --check、cargo test、cargo clippy --all-targets -- -D warnings。
- [x] 将红绿证据与环境限制交给父任务，父任务更新统一文档和需求；不自行宣称 Windows 真机验收或发布新版本。

代码阶段完成；最终原生 CI 和合并状态统一记录在[多屏计划](2026-09-07-multidisplay.md)与[开发日志](../../development/2026-09-07-multidisplay-and-windows-localization.md)，不另建需求列表。
