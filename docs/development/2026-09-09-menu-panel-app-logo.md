# 2026-09-09：双平台菜单面板应用 Logo

关联 `REQ-20260909-002`。[设计规格](../design/specifications/2026-09-09-menu-panel-app-logo-design.md)、[实现计划](../design/implementation-plans/2026-09-09-menu-panel-app-logo.md)。

用户希望截图中的菜单弹出面板在“AI Token Meter”软件名旁显示应用 Logo，并与现有背景自然融合。范围收敛为 macOS 菜单栏弹出面板和 Windows 原生托盘菜单；设置页已有 Logo 不替代这两个入口。现有背景、标题文字、刷新位置、用量摘要、菜单动作、版本和更新源均保持。

## 实现

- macOS `MenuBarPanel` 默认读取 `NSApplication.shared.applicationIconImage`，在标题/副标题块前显示32×32pt图标，间距10pt。图标没有底板、边框或阴影，并作为装饰图从辅助功能树隐藏；刷新按钮仍在原 `Spacer` 右侧。
- Windows 复用 Tauri 默认窗口图标，在原生托盘菜单首项构建 `IconMenuItem`，固定文本为 `AI Token Meter`、状态为禁用，后接原生分隔线。四项服务摘要、刷新、设置、浮动条、关于和退出事件没有改动。
- 两端均复用仓库已有应用图标，没有新增图片资产、网络请求、偏好迁移或用户数据字段。

## 失败先行证据

- macOS 品牌渲染测试先向真实 `MenuBarPanel` 注入纯品红合成图标；实现前因初始化器不接受 `brandIcon` 而编译失败，错误为 `extra argument 'brandIcon' in call`。
- Windows 托盘测试先通过 Tauri `MockRuntime` 调用 `build_brand_header`；实现前因该函数不存在而编译失败。测试随后用合法1×1 RGBA检查菜单项身份、文字和禁用状态，并用错误字节长度确认图标参数确实进入原生构建器。
- 初次独立 Swift scratch 构建因隔离目录无法联网取得已锁定的 Sparkle 依赖而停止；改用仓库已有依赖缓存后取得上述真实编译红灯，没有把环境错误记作产品失败。

## PR原生复验修正

PR #19首轮Windows CI `34330043004`通过前端、真实浏览器、production build、格式和严格Clippy后，`tray_summary`测试程序在进入测试前以`0xc0000139 / STATUS_ENTRYPOINT_NOT_FOUND`退出。失败只发生在本轮新增的`tauri::test::mock_app()`调用；Tauri官方开放缺陷`tauri-apps/tauri#13419`记录了相同调用、平台和退出码，另一个相同报告`#13954`已归并到该缺陷。

修正先把测试改为要求尚不存在的`brand_header_definition`并取得未解析导入RED，再让生产构建函数通过这一纯定义传递固定ID、文案、禁用状态和图标。两项回归直接验证合法RGBA原样进入定义、错误尺寸被拒绝，不启动上游有缺陷的MockRuntime；正式路径仍把同一定义交给真实Tauri `IconMenuItemBuilder`，Windows原生编译、runtime、NSIS与GUI subsystem门禁不跳过。

修正提交`1efc8ed`的Windows CI `34332463420`用时12分17秒全绿，完整runtime、NSIS、GUI subsystem和安装器上传均通过；同一提交的macOS CI `34332463456`用时2分47秒通过。该缺陷由`REQ-20260909-005`结项。

## 渲染与视觉证据

macOS 测试以2×比例渲染真实面板，扫描纯品红像素边界，要求图标为62至64像素见方且位于前导120像素内。生成物只在设置 `AI_METER_DOC_SCREENSHOT_DIR` 时写入 `/private/tmp/req002-menu-brand/menu-panel-brand-header.png`；截图使用合成图标和 demo 模型，不包含真实账号、额度或截图元数据。人工查看确认图标位于标题左侧，标题/副标题整体垂直对齐，右侧刷新按钮和面板背景未改变。

Windows 宿主环境可真实构建 Tauri 原生菜单项，但不能替代交互式 Windows 11 对原生菜单的最终观感与多 DPI 检查。现有应用图标文件已人工查看，自动化验证了同一默认图标进入品牌首行和托盘图标构建路径；Windows 真机视觉继续遵守项目既有现场边界。

## 验证

- macOS：464项普通 Swift 测试、3项刷新调度、18项 PTY runner 与6项 Gemini PTY，共491项通过；其中新增真实面板品牌渲染测试1项，相关菜单/品牌回归43项通过。
- Windows 前端：18个测试文件共124项，加5项终端输入协议测试通过；TypeScript/Vite production build通过。
- 真实浏览器密度：25项进程生命周期、8个Gemini详情场景、16组四服务布局和608个文字角色通过。首次仅因受限环境禁止绑定回环预览端口而停止，获准后同一命令全绿。
- Windows Rust：修正后完整261项通过，含5项托盘摘要/品牌测试；`cargo fmt --check` 与全目标 `-D warnings` Clippy通过，依赖图确认不再启用Tauri `test`特性。首次完整运行的6项本地HTTP夹具只因受限环境禁止绑定localhost而失败，获准后原命令全部通过。
- macOS无Widget Release App构建通过，便携资源、Sparkle framework/helper、`@rpath`、嵌套组件与严格签名结构均通过。
- 6份跨平台合同、合同可移植性、Windows发布资产归一化、更新源工具、216份Markdown、公开发布安全和Git差异检查全部通过。

## 审查结论

逐项对照规格审查 `78b1ad3..da50eb3`：macOS真实面板接入、32pt/10pt、装饰性可访问性和刷新位置符合；Windows品牌首行、禁用状态、分隔线、默认图标复用和事件保持符合；未发现新增资产、敏感数据、版本或更新源变化。Critical 0、Important 0、Minor 0。

## Git与交付边界

- 规格与计划：`2f52437`
- 测试驱动实现：`da50eb3`
- 验证与审查记录：`d4de77b`；该检查点已快进整合到本地 `main`，整合前协调入口的未提交需求状态已单独保存。
- 用户随后通过 `REQ-20260909-004` 明确要求直接发布；本成果已纳入0.6.2/build17候选，发布证据见[0.6.2记录](2026-09-09-v0.6.2-release.md)。
