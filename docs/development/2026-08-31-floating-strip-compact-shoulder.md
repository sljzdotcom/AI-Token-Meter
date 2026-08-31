# 浮岛紧凑短肩开发记录

日期：2026-08-31  
分支：`codex/compact-shoulder`  
规格：`docs/design/specifications/2026-08-31-floating-strip-compact-shoulder-design.md`  
计划：`docs/superpowers/plans/2026-08-31-floating-strip-compact-shoulder.md`

## 问题与根因

上一版浮岛使用三段连续 S 曲线，顶部从 `(108, 0)` 展开到 `(0, 104)`，底部严格镜像。路径连接和阴影问题已经解决，但单侧肩部仍占 104 点高度，在 108 点宽的窄浮岛上形成过大的外鼓。

用户在真实安装版截图中用红线标注目标边界，并在三种视觉原型中选择方案 A“紧凑短肩”。其核心不是继续调长 S 曲线，而是把轮廓改为贴边短平台加紧凑圆弧。

## 实现

窗口保持 `108 × 356`，右贴边路径改为：

- `(108, 16)` 到 `(66, 28)`，控制点 `(98, 23)`、`(88, 27)`；
- `(66, 28)` 到 `(0, 88)`，控制点 `(29, 29)`、`(0, 54)`；
- 中央直边从 `(0, 88)` 到 `(0, 268)`；
- 底部两段曲线严格关于 `y = 178` 镜像，最终到 `(108, 340)`；
- 闭合路径沿贴边侧连接上下终点。

左贴边继续使用既有 `point(_:_:)` 解析器水平镜像。没有修改窗口尺寸、圆环位置、Logo、表面填充、服务配色、详情布局或数据逻辑。

## TDD 证据

### 基线

隔离工作树建立后运行完整测试：152 项、33 个测试组、0 失败，4 项环境门控跳过。

### 红灯

先添加方案 A 的真实路径与拖动契约，再运行旧实现：

- `VisualSystemTests`：13 个预期问题；旧路径边界仍为 `(0, 0, 108, 356)`，`(40, 50)`、`(1, 94)` 等新内点不在路径中，`(107, 8)` 等新透明区仍有填充；
- `FloatingStripDragShapeTests`：1 个预期问题，短肩可见点 `(40, 50)` 不能拖动；
- `FloatingStripPointerDragStateTests`：2 个预期问题，真实指针入口不能从 `(40, 50)` 开始拖动。

共 16 个问题均来自旧几何，不是编译错误。

### 绿灯

替换为规格坐标后：

- `VisualSystemTests`：11 项通过；
- `FloatingStripDragShapeTests`：4 项通过；
- `FloatingStripPointerDragStateTests`：4 项通过；
- 完整测试：153 项、33 个测试组、0 失败；4 项真实钥匙串或已安装 CLI 环境门控按设计跳过。

测试辅助进程仍会报告受限环境无法创建 WebKit 用户缓存目录，但对应测试通过，测试命令退出码为 0。

## Release 与安装

`bash scripts/build-app.sh` 完成生产构建、App Bundle 组装、资源生成和 ad-hoc 签名。随后验证：

- `codesign --verify --deep --strict`：通过；
- `plutil -lint`：`OK`；
- 候选版 SHA-256：`d7b941ddaf4be3108c85567c4bc8d5ddd9d2cea26fc8d17d1adfc8c7fd9249d1`；
- 安装版 SHA-256：`d7b941ddaf4be3108c85567c4bc8d5ddd9d2cea26fc8d17d1adfc8c7fd9249d1`；
- 旧安装版备份：`/private/tmp/AI Meter.app.pre-compact-shoulder-20260831-1600`。

## 实机验收

- 右贴边显示紧凑短平台和短圆弧，没有旧版大范围外鼓；
- 左贴边正确镜像；验收后恢复 Right；
- 在玻璃空白区域进行真实拖动，垂直位置随拖动变化，随后恢复用户原有 97%；
- Codex Logo 点击后无障碍状态变为 `Detail open`，证明 Logo 与详情交互正常；
- AI Meter 菜单中的 Settings 可打开完整设置窗口；
- Claude、Codex、DeepSeek 的 Logo、圆环和已确认的独立配色保持不变；
- 浅色背景上没有黑色外投影或边缘接缝。

## Git 检查点

- `c60634c`：确认紧凑短肩规格；
- `5be0989`：实现计划；
- `df7afa6`：方案 A 路径与回归测试（任务 1/2）。
