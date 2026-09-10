# Compact浮动条横向收窄

关联 `REQ-20260910-001`，规格见[设计说明](../design/specifications/2026-09-10-compact-strip-width-design.md)，步骤见[实施计划](../design/implementation-plans/2026-09-10-compact-strip-width.md)。

## 背景与范围

用户要求Compact样式中圆环左右两边再收紧，圆环大小和上下高度保持不变。本项采用70pt/px宽度；48pt/px圆环的单侧空间由约15降至11。Compact的Logo、环间距、线宽、1至4个Provider高度，以及Comfortable和折叠把手均保持。

## 失败证据

生产代码仍为78时，Swift尺寸测试明确收到`78.0`而期望`70.0`；Windows前端两份测试共5项明确收到`78px`而期望`70px`；Rust原生偏好与窗口尺寸测试3项明确收到`78.0`而期望`70.0`。失败均指向旧宽度，圆环和高度断言未改变。

## 实现与验证

macOS的Compact逻辑宽度改为70pt，独立S形路径只按78→70的比例收紧横向控制点，纵向坐标保持。Windows前端、SVG裁剪、CSS后备值、真实浏览器样本和Rust原生逻辑窗口统一为70px；48px圆环和全部高度保持。

定向转绿结果：macOS 53项FloatingStrip尺寸、轮廓、左右镜像、1至4个Provider、拖动/点击、位置和真实渲染通过；Windows前端17项、Rust原生偏好与位置9项通过。旧78宽和新70宽的2×截图均为286pt同高，像素宽分别为156和140，圆环像素尺寸保持不变。

完整验证：

- macOS 461项主测试、3项独立刷新调度、18项PTY runner通过，6份跨平台合同和发布辅助测试通过；
- macOS无Widget arm64 Release App完成资源、Sparkle嵌套组件和严格签名验证；Widget因本机没有Apple Development身份按既有边界跳过；
- Windows 124项前端和production build通过；
- 真实Chrome通过25项浏览器进程生命周期、16组四Provider裁剪/点击布局和608个文字角色；
- Windows宿主241项Rust、格式和全目标全功能严格Clippy通过；不同DPI的位置测试覆盖1×、1.25×、1.5×、2×；
- 240份Markdown、公开安全检查与`git diff --check`通过。

最终差异复核确认生产改动只触及Compact宽度、Compact路径及Windows CSS后备值；Comfortable、圆环、Logo、高度、折叠把手、详情和数据采集均未改变。物理Windows交互边界继续保留。

## 安全、发布与环境边界

截图使用演示数据，不读取或保存真实账号、凭据或额度。本项保持公开版本0.6.3，不自动创建tag、Release或更新源。物理Windows 11的指针、多屏和DPI视觉仍保留现场观察边界。
