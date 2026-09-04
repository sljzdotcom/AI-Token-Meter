# Windows 浮动条视觉与贴边稳定性修复记录

## 范围

对应 `REQ-20260904-004`。用户在 Windows `0.3.0-preview.0` 真机发现白色矩形外框、上下白条、曲线锯齿、反向肩部缺失和拖动贴边不稳定。用户确认 Windows 采用 macOS 已验收轮廓，并授权 macOS 保持视觉不变、两平台同步发布 `0.3.0-preview.1`（build 8）。

## 根因证据

- meter 是 `decorations:false + transparent:true`，但没有关闭 Windows 系统阴影；Tauri/Windows 对无边框窗口的阴影会产生可见 1px 外框。
- `styles.css` 使用十一点 `polygon()`，Win32 又把窗口裁成 16 段整数 GDI Region；两套路径互相切割，并且都不等于 macOS `FloatingStripShape`。
- strip-only CSS 隐藏上下肩部伪元素，因此深海背景只覆盖中间主体。
- 前端在 `startDragging()` Promise 后立刻调用吸附保存；Windows WebView 的拖动完成时序不能作为可靠鼠标释放合同。
- 显示器拓扑线程启动后第一轮无基线，会无条件恢复一次位置；拖动期间也没有互斥状态。

## 红绿测试

第一轮先增加真实 DOM 和 Rust 几何测试：

- 前端要求存在左右两条 `objectBoundingBox` SVG path，坐标由 macOS 108×356 路径手工归一化；旧实现因组件不存在失败。
- Rust 要求路径包含 `(108,16)/(66,28)/(0,88)/(0,268)/(66,328)/(108,340)` 并严格水平镜像；旧曲线因从 `(108,0)` 开始而失败。
- 实现后前端定向 13/13、production build 和 Rust 两项几何测试通过。

第二轮先增加拖动状态和拓扑策略测试：

- 一个会话只能占用一次，旧会话不能结束新会话，取消路径必须解锁；实现前因 `MeterDragGate` 不存在而编译失败。
- 首次拓扑观察只建基线、拖动期间不恢复、只有基线后的真实变化才恢复；实现前因策略函数不存在而编译失败。
- 实现后 Windows 平台相关 20 项测试及 macOS 主机严格 Clippy 通过；Windows-only API 继续由 GitHub `windows-latest` 编译运行。

## 实现要点

- `FloatingStrip` 输出左右两条归一化 SVG clip path；`.floating-strip` 自身同时承载背景和内容。
- meter 配置和运行时都关闭 shadow；Windows 11 额外尝试 `DWMWA_BORDER_COLOR = DWMWA_COLOR_NONE`。
- 删除 `SetWindowRgn` 可见裁剪，避免整数 GDI Region 截断 WebView2 抗锯齿边缘。
- `begin_meter_drag` 在系统拖动前登记原生会话；Windows 线程观察 `VK_LBUTTON`，释放后才吸附、保存稳定显示器 ID/边/高度并发出边变化。
- 显示器拓扑线程首次只建立快照；拖动期间若拓扑变化只更新基线，不争抢窗口位置。

## 安全与兼容边界

本次不读取或修改任何 Provider 凭证、账号、缓存内容或网络端点。DWM 属性在不支持的系统上按兼容方式忽略；窗口透明和启动不依赖该调用成功。macOS 只同步版本号和 build，不修改 `FloatingStripShape` 或 AppKit 窗口行为。

## 发布验证

本机发布候选通过前端 15 项测试与 production build、Rust 130 项测试、rustfmt、严格 Clippy、macOS 386 项测试、139 份 Markdown 文档检查、4 份跨平台 fixture 合同、Release feed/Windows 资产归一化脚本和公开源码/历史秘密检查。完整 Rust 首轮还暴露一条仍锁定旧 `(width, 0)` 粗轮廓的遗留窗口策略测试；将其改为校验新 macOS 同源肩部坐标和严格镜像后，130 项全绿。

标签 `v0.3.0-preview.1` 指向发布候选提交 `9213fd6`。GitHub Actions [33826484923](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/33826484923) 的 Windows 原生构建、macOS 标签源码核验和同步发布全部通过，公开 prerelease 已发布至 [GitHub Release](https://github.com/sljzdotcom/AI-Token-Meter/releases/tag/v0.3.0-preview.1)。

公开匿名重下后的校验结果：

- macOS ZIP SHA-256：`89aec717f253875051f748b760191628d227bdf399c7602142fca89acf8d9ab9`；Sparkle 签名通过，篡改副本被拒绝；
- Windows NSIS SHA-256：`1e44dda9315d48fe7fc62a31d510c0339e2d24ccd1824a97e8ae5bce09474cdd`；Tauri minisign 使用应用内置公钥验证通过；
- `latest.json` 与固定入口 `latest-preview.json` 字节一致，SHA-256 为 `73c88307217d110fd9e8bda92064efa399cff50d4f4cef07dbeffc0e0de3803c`，版本和下载地址均指向本次 Windows 安装器；
- 稳定 `appcast.xml` 仍指向 `0.2.2`，Preview 发布没有污染 macOS 稳定更新通道。

发布后仍需用户在真实 Windows 11 检查 DPI、白边、左右肩部、拖动、多屏位置和 `preview.0 → preview.1` 原位升级。未取得这些证据前，需求只能标记为 `待用户确认`。
