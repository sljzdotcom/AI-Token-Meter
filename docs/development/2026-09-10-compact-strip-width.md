# Compact浮动条横向收窄

关联 `REQ-20260910-001`，规格见[设计说明](../design/specifications/2026-09-10-compact-strip-width-design.md)，步骤见[实施计划](../design/implementation-plans/2026-09-10-compact-strip-width.md)。

## 背景与范围

用户要求Compact样式中圆环左右两边再收紧，圆环大小和上下高度保持不变。首轮70pt/px候选通过完整本机验证；用户看过后要求继续改为65pt/px查看效果。65宽下，48pt/px圆环的单侧空间由原78宽时约15降至8.5。Compact的Logo、环间距、线宽、1至4个Provider高度，以及Comfortable和折叠把手均保持。

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

实现与完整本机证据候选为`2d2a260`。公开推送、PR、合并、tag、Release和更新源均未执行。

## 65pt视觉迭代

用户要求把宽度从78进一步改为65并查看效果。测试先从70改为65：Swift尺寸测试收到`70.0`、Windows前端5项收到`70px`、Rust原生3项收到`70.0`，均与65期望形成失败证据。随后macOS、Windows前端、CSS后备、共享合同、浏览器样本和Rust原生窗口统一为65；Compact内容的额外横向内边距降为0，使48pt圆环在65pt窗口中居中并保留每侧8.5pt空间。

65是奇数逻辑宽度，2×Retina渲染时现有Button与参照Logo的抗锯齿会出现1至2个物理像素采样差。实际界面截图确认Logo和圆环未裁切；一次`-0.5pt`偏移实验不能消除差异，已撤销。像素测试继续验证Logo方向与明亮像素，只把匹配范围放宽到2个Retina像素，没有删除镜像拒绝断言。

65预览的定向结果：Windows前端17项、Rust原生偏好与位置9项、macOS 54项Compact专项通过。78、70和65宽的2×三Provider截图均为572像素高，宽度分别为156、140和130像素，圆环尺寸保持。

65pt本地预览检查点为`cc4114d`，尚未公开推送。

用户确认65为最终宽度后，以同一候选完成完整本机门禁：

- macOS 462项主测试、3项独立刷新调度和18项PTY runner通过；6份跨平台合同、发布辅助测试、240份Markdown和公开安全检查通过；
- macOS无Widget arm64 Release App完成资源、Sparkle嵌套组件和严格签名验证；Widget继续按本机无Apple Development身份的既有边界跳过；
- Windows 124项前端、production build、25项浏览器进程生命周期、8组Antigravity详情、16组四Provider裁剪/点击布局和608个文字角色通过；
- Windows宿主241项Rust通过，格式及全目标全功能严格Clippy通过；首次受限运行的6项本地测试服务器用例仅因沙箱拒绝回环端口失败，允许本地回环后原命令完整通过，没有修改或跳过测试；
- `git diff --check`通过。最终复核确认生产改动仅收紧Compact窗口、路径及布局内边距；圆环、Logo、垂直尺寸、Comfortable、折叠把手、详情和采集逻辑保持。

65pt实现候选仍为`cc4114d`，完整门禁证据为`afb58d5`。公开推送、PR、合并、tag、Release和更新源均未执行。

## 安全、发布与环境边界

截图使用演示数据，不读取或保存真实账号、凭据或额度。本项保持公开版本0.6.3，不自动创建tag、Release或更新源。物理Windows 11的指针、多屏和DPI视觉仍保留现场观察边界。
