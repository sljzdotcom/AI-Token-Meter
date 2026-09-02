# 浮动条「深海波纹」背景开发日志

日期：2026-08-31

规格：`docs/design/specifications/2026-08-31-floating-strip-deep-sea-background-design.md`

计划：`docs/design/implementation-plans/2026-08-31-floating-strip-deep-sea-background.md`

## 目标与范围

本次只调整屏幕边缘浮动条内部视觉：加入方案 C 的静态黑蓝「深海波纹」背景。浮动条轮廓、窗口尺寸、三枚服务 Logo、圆环颜色、详情页、点击和拖动规则均不改变。左右贴边时背景随轮廓镜像，前景控件不镜像。

## 资源制作

最终资源位于：

`Sources/AIMeterApp/Resources/Backgrounds/floating-strip-deep-sea.png`

资源由内置图像生成能力制作，以近黑、墨蓝和受控亮蓝形成两至三条宽阔波纹，明确排除了 Logo、圆环、文字、粒子、气泡、橙色和外投影。原始生成结果保留在生成工具目录，项目内版本机械裁切并重采样为 `324 × 1068` PNG，恰好对应 `108 × 356` 浮动条的 3× 高密度尺寸。

## 实现结构

- `Package.swift` 将 `Resources/Backgrounds` 作为独立资源目录复制进 App Bundle；
- `FloatingStripBackgroundAsset` 负责定位和解码资源；
- `FloatingStripSurface` 先绘制原有玻璃渐变，再按比例填充背景图，覆盖 `0.38` 的固定黑色遮罩，最后以现有 `FloatingStripShape` 裁切；
- `FloatingStripBackgroundPresentation.horizontalScale` 只在左贴边时把图片水平镜像；
- `FloatingStripView` 继续在背景上方绘制 Provider 圆环，因此 Logo 和进度方向不会随图片翻转；
- 图片加载失败时 `FloatingStripSurface` 仍显示原玻璃底色，不影响交互；
- 背景图片标记为辅助功能隐藏，浮动条本身以单一可操作元素暴露拖动和边缘切换动作。

## 测试驱动过程

资源测试先因 `FloatingStripBackgroundAsset` 不存在而正确失败，随后加入资源定位、SwiftPM 声明和最终图片后转绿。镜像与回退测试先因 `FloatingStripSurface` 不支持注入背景而正确失败，之后完成最小合成实现。

新增或强化的检查覆盖：

- App Bundle 能找到并解码 3× 背景资源；
- 使用真实红蓝分区测试图确认左右渲染互为镜像；
- 资源缺失时保留玻璃回退和透明肩部；
- 外部像素透明、无多余阴影或接缝；
- Claude、Codex、DeepSeek 品牌配色和 DeepSeek 余额进度语义不变；
- 玻璃空白拖动、Provider 点击排除和辅助功能移动仍有效。

完整回归结果：156 项测试、33 个测试组、0 失败。4 项依赖真实 Claude/Codex 登录或钥匙串隔离条件的集成检查按设计跳过。

## 构建、安装与指纹

- Release 构建：通过；
- ad-hoc 签名验证：通过；
- `Info.plist`：通过 `plutil` 校验；
- 背景资源：PNG，`324 × 1068`；
- 最终候选包与 `/Applications/AI Meter.app` 可执行文件 SHA-256：`fbeada3e35dfe5d61449aac929ae6748d61a8a084882fcc639616a24195ddd25`，完全一致；
- 安装前版本可恢复备份：`/private/tmp/AI Meter.app.pre-deep-sea-background-20260831-2202`；
- 首轮候选版本可恢复备份：`/private/tmp/AI Meter.app.pre-accessibility-fix-20260831-2205`。

## 真实界面验收

- 右贴边：黑蓝波纹、短肩、透明区域和三枚 Logo 清晰；
- 左贴边：背景与轮廓正确镜像，Logo 和圆环方向不变；
- Claude、Codex、DeepSeek 均可打开详情，Codex 自动隐藏实测通过；其他详情沿用同一会话控制器并由回归测试覆盖；
- Settings 从菜单栏正常打开，Left/Right 设置均生效；
- 浮动条拖动命中逻辑与 Provider 点击排除测试通过；
- 外部空白点击关闭由本地/全局事件策略回归测试覆盖；
- 新背景最初使拖动面板在系统辅助功能树中退化为容器，已通过把表面声明为单一辅助功能元素修复；修复后系统树重新显示 `Move floating meter` 及增减、左右切换动作；
- 验收结束后恢复用户原有右侧偏好与垂直位置，系统树确认 `Right edge, vertical position 97 percent`。

## Git 检查点

- `004e74c`：确认深海波纹视觉规格；
- `3f2e7d8`：记录完整实施计划；
- `aece1a5`：加入背景资源、资源声明与首组测试；
- `5688acf`：完成镜像、裁切、遮罩、回退与渲染测试；
- `9110f72`：修复背景合成后的浮动条辅助功能回归。

最终文档提交和主分支合并节点在完成对应 Git 操作后由提交历史继续记录。
