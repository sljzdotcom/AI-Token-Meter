# AI Token Meter 下一阶段需求登记

**登记日期：** 2026-09-01  
**状态：** R1/R4/R5/R6 已完成实现；R2 已实现并完成自动化与部分实机验收，完整桌面层验收待补；R3 是下一独立子项目
**登记基线：** `main` 提交 `8c42ab2`
**设置与品牌实施：** 功能提交 `e5f8e94`、`1f7f6f4`，通过 `337ff72` 合入 `main`

本文只记录用户确认提出的下一阶段需求和当前事实，不代表设计方案已经批准。开始开发前，仍需按每个子项目完成设计、规格确认、实现计划和验收。

## 需求清单

### R1. Settings 使用分类 Tab

**进度：** 已完成。设置固定分为 Appearance、Monitoring、Services、About 四个顶部 Tab，服务反馈会自动路由到 Services，登录项反馈路由到 Monitoring；Settings 根视图始终使用系统字体。

- Settings 内容增多后，改为按类别划分的 Tab 界面，避免所有设置堆叠在同一页面。
- 后续新增设置应根据功能归属自动放入合适的 Tab，而不是继续追加到单一长页面。
- Tab 顺序和职责已由自动化测试锁定，后续设置按职责放入对应页面。

### R2. 浮动条只在桌面显示

**进度：** 实现与自动化合同已完成，见[桌面层与肩部背景连续性规格](superpowers/specs/2026-09-01-floating-strip-desktop-layer-and-background-crop-design.md)和[开发验收日志](development/2026-09-01-floating-strip-desktop-layer-and-background-crop.md)。系统整屏截图已证明普通/全屏 Edge 不被浮动条或详情覆盖，Space 切换关闭详情和位置保持也已完成本机验收；Mission Control、左右两个普通 Space 和多显示器仍需人工环境补验，因此暂不标记完整完成。

- 浮动条只应出现在 macOS 桌面场景。
- 当 Edge 或其他应用进入全屏空间时，浮动条不应覆盖在全屏应用之上。
- 需要明确并测试普通窗口、全屏空间、多显示器、切换 Space、Mission Control 和显示器插拔时的显示规则。

### R3. macOS 桌面 Widget

- 新增可添加到 macOS 桌面的 WidgetKit Widget。
- 支持多种系统允许的尺寸。
- 最小尺寸只显示三个带 Provider Logo 的状态框，不显示冗余文字。
- 中型和更大尺寸展示哪些额度、余额、重置时间和刷新状态，留待独立 Widget 规格确认。
- Widget 与主应用之间的数据共享、刷新频率、隐私边界和无主应用运行时的降级行为需要单独设计。

### R4. 产品名称与副标题

**进度：** 已完成。显示名称为 **AI Token Meter**，副标题为 **Private AI usage monitor**，构建产物为 `AI Token Meter.app`。

- 产品显示名称改为 **AI Token Meter**。
- Bundle Identifier `com.millerpan.AIMeter`、可执行文件 `AIMeterApp`、Keychain 身份和 `Application Support/AI Meter` 兼容目录保持不变，升级不会丢失偏好、密钥访问、缓存或 Claude 工作区批准。

### R5. 应用 Icon 状态确认与后续处理

**进度：** 已确认沿用当前无文字仪表指针图标；Bundle 元数据测试继续验证 `CFBundleIconFile = AppIcon`。

当前事实：

- 项目已经有自定义 App Icon，不是最初的空白图标，也不是写有 `AI` 的文字图标。
- 当前 Icon 是深蓝底色、青绿到紫色进度环和白色仪表指针的图形。
- 图标由 `scripts/generate-app-icon.swift` 生成，`Info.plist` 已通过 `CFBundleIconFile = AppIcon` 引用，构建产物中存在完整 `AppIcon.icns`。
- 该图标最初由提交 `1a165d3` 引入。

如果 Finder、Dock 或应用列表仍显示旧图标，应先排除 macOS 图标缓存或仍在运行旧安装包，不重新设计图标。

### R6. 背景图覆盖顶部反向半圆

**进度：** 已完成。`1.22×` 等比裁切、左右仅水平镜像、真实肩部像素和玻璃 fallback 均有自动化回归；左右贴边实图均显示上下肩部连续蓝色波纹，原始 PNG SHA-256 与基线一致且无 Git diff。见[开发验收日志](development/2026-09-01-floating-strip-desktop-layer-and-background-crop.md)。

- 当前深海背景图只在浮动条中间主体区域完整显示，顶部反向半圆/肩部存在黑色或未覆盖区域。
- 背景图应继续延伸并裁切到顶部反向半圆中，使半圆与中间长方形成为一张连续、无接缝的背景。
- 不应出现单独边框、黑色补丁、透明断层或方向错误。
- 左右贴边时，背景与形状必须正确镜像，但 Provider Logo、圆环和内容不能镜像。

## 初步拆分

这些需求拆分为三个相对独立的子项目：

1. **浮动条显示与视觉修复：** R2、R6。
2. **设置与品牌整理：** R1、R4、R5。
3. **WidgetKit 扩展：** R3。

前两个子项目已经实现；下一步单独建立 WidgetKit 扩展。Widget 依赖的共享展示模型与品牌命名现在已经稳定。

## 剩余验收与设计问题

- R2 已采用桌面层语义：普通/全屏 Edge 的系统整屏证据通过，Space 切换关闭详情；仍需补验 Mission Control、左右两个普通 Space 和多显示器结果。
- Widget 支持的尺寸、各尺寸字段和是否允许交互。

## 实施门槛

任何代码改动开始前必须完成：

1. 为对应子项目探索现有实现并提出方案；
2. 用户确认设计；
3. 编写并提交设计规格；
4. 编写实现计划；
5. 按测试驱动方式实施、审查、安装和验收。
