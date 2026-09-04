# Windows 浮动条视觉与贴边稳定性修复设计

**需求：** `REQ-20260904-004`  
**日期：** 2026-09-04  
**状态：** 已确认（用户选择方案 A，并授权 macOS 同版本发布）

## 问题与根因

Windows `0.3.0-preview.0` 真机截图暴露四个相互关联的问题：无边框透明窗口外侧出现白色矩形边框和上下白条；可见轮廓有阶梯与凹凸；macOS 已确认的上下反向肩部缺失；拖动后贴边偶发不稳定。

代码和平台合同核对得到以下根因：

1. Tauri 的无边框 Windows 窗口未关闭系统阴影。Windows 会给这类窗口绘制 1px 外框，透明窗口上尤其明显。
2. 前端使用十一点 CSS `polygon()`，原生端又使用 16 段整数 `CreatePolygonRgn`；两套曲线既不等价，也都不同于 macOS 的四段 Bezier。原生 GDI Region 会再次裁掉浏览器已经抗锯齿的边缘。
3. strip-only 模式明确隐藏了用于上下肩部的伪元素，所以背景无法覆盖 macOS 轮廓中的反向半圆区域。
4. 网页把 `startDragging()` Promise 完成当作可靠的拖动结束事件；同时显示器拓扑线程每 750ms 可执行恢复定位，拖动期间没有互斥状态，产生提前吸附或被重排的竞态。

## 目标

- Windows 可见轮廓精确复用 macOS `FloatingStripShape` 的 108×356 基准 Bezier，并按窗口实际尺寸等比归一化。
- 左右贴边互为镜像；背景在完整肩部内连续，不出现矩形白边、上下白条或额外阴影。
- 轮廓由 WebView2 的 SVG clip path 单一负责抗锯齿，Win32 不再以低精度 Region 二次切割可见像素。
- 拖动开始到鼠标释放期间暂停显示器拓扑自动恢复；释放后只吸附一次，保存显示器、侧边和相对高度。
- 保持既有多显示器稳定身份、临时主屏回退、桌面可见性、详情窗口和 Settings 行为。
- macOS 的视觉和窗口代码不改动；仅随跨平台版本号同步发布。

## 采用方案

### 可见轮廓

`MeterSurface` 提供两个文档级 SVG `clipPath`。右侧路径把 macOS 坐标除以 108 和 356，得到 `objectBoundingBox` 的归一化路径；左侧路径以 `x' = 1 - x` 镜像。`.floating-strip` 根据 `meter-edge--left/right` 引用对应路径。

基准路径严格为：

```text
M 108 16
C 98 23, 88 27, 66 28
C 29 29, 0 54, 0 88
L 0 268
C 0 302, 29 327, 66 328
C 88 329, 98 333, 108 340
Z
```

背景图片、暗色渐变、Logo 与进度环仍由同一个 `.floating-strip` 元素承载，因此完整路径（包括上下肩部）使用同一背景采样，不再拼接伪元素。

### Windows 窗口合成

- Tauri 配置对 meter 明确设置 `shadow: false`。
- 运行时再次调用 `set_shadow(false)`，防止配置或升级路径遗漏。
- Windows 11 上把 DWM border color 设置为 `DWMWA_COLOR_NONE` 作为防御层；不支持该属性时忽略兼容性错误，透明窗口仍可工作。
- 删除可见轮廓的 `SetWindowRgn`/`CreatePolygonRgn`。窗口保持矩形透明画布，视觉轮廓只由 WebView2 抗锯齿路径绘制，避免 GDI Region 切断半透明边缘。

矩形透明角落会占用极小的窗口命中范围，但窗口本身只有 116 logical px 宽，且只有非按钮背景用于拖动；本次优先保证用户明确要求的视觉完整与稳定拖动。若未来需要逐像素穿透，应独立实现 `WM_NCHITTEST`，不得重新引入可见 GDI 裁剪。

### 拖动完成与拓扑互斥

网页在开始系统拖动前调用 `begin_meter_drag`，原生状态立即标记 `drag_active=true`，然后调用 Tauri `startDragging()`；不再在 Promise 后调用 `meter_drag_ended`。

Windows 原生监视器读取鼠标左键状态：

1. 看到拖动开始后等待左键释放；
2. 释放时执行一次 `snap_meter_after_drag`；
3. 保存稳定显示器 ID、Left/Right 和归一化高度；
4. 发出 `meter-edge-changed`；
5. 无论成功失败都清除 `drag_active`。

显示器拓扑监控在 `drag_active` 时只更新观察到的拓扑，不移动窗口；拖动结束后下一轮按已保存位置继续。启动时先建立拓扑基线，不再把第一轮轮询无条件当作变化并重复恢复。

原生拖动监视设有限时与无按键回退，防止 WebView 事件中断后永久锁定。重复开始请求由原子 compare-and-swap 合并为一个会话。

## 错误处理

- 原生鼠标状态读取或吸附失败：清除拖动态并保留窗口当前位置，不覆盖已保存配置。
- 设置保存失败：界面仍完成吸附，但沿用旧配置；错误只写入脱敏日志。
- DWM 边框属性在旧 Windows 不可用：不阻止启动。
- SVG clip path 不可解析属于构建/浏览器合同错误，由前端渲染测试和 Windows 真机构建验收拦截。

## 测试与验收

- 前端测试以真实 DOM 验证左右路径存在、strip 引用正确的 SVG clip path、背景元素不再依赖肩部伪元素。
- Rust 单元测试用手工坐标锁定 108×356 的关键点、左右镜像和缩放行为；旧粗曲线必须使测试失败。
- Rust 状态测试验证重复拖动只启动一次、拖动时禁止拓扑重排、结束/超时必定解锁。
- Windows CI 运行前端测试与构建、Rust 测试、rustfmt、严格 Clippy、Tauri Release/NSIS 构建。
- 文档、跨平台合同、秘密扫描和发布门禁全部通过。
- 发布 `0.3.0-preview.1`：macOS 与 Windows 使用相同版本号和同一个 GitHub prerelease；稳定 appcast 不收录 Preview。
- Windows 真机验收项目：100%/125%/150%/200% DPI 无白边和明显锯齿；左/右镜像肩部完整；拖动到任意侧仅吸附一次；重启保持位置；多屏目标在线不漂移、离线临时回主屏。

## 非目标

- 不重新设计 macOS 浮动条。
- 不改变三服务 Logo、配色、数据口径或详情布局。
- 不把 Preview 写入稳定更新通道。
- 不在本次引入 Windows Widget 或逐像素透明区点击穿透。

