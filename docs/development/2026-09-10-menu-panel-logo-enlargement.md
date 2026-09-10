# 菜单面板顶部 Logo 放大

关联 `REQ-20260909-011`，规格见[设计说明](../design/specifications/2026-09-10-menu-panel-logo-enlargement-design.md)，步骤见[实施计划](../design/implementation-plans/2026-09-10-menu-panel-logo-enlargement.md)。

## 实现

macOS `MenuBarPanel` 顶部应用Logo由32×32pt改为40×40pt，精确放大25%。面板继续固定为380pt宽，外边距16pt，Logo与产品名/副标题文字块的间距继续为10pt。标题字号、刷新按钮、系统菜单栏图标、Provider图标、Settings、About和Windows原生托盘菜单没有修改。

Logo继续直接使用应用自身图标并保持高质量缩放、透明融合和辅助功能隐藏，没有增加图片资产、底板、描边、圆角或阴影。

## 失败先行与视觉证据

真实`MenuBarPanel`的2× Retina渲染测试先把期望从32pt改为40pt。未改生产代码时，测试实测宽高均为64px，因要求78–80px而准确失败。生产尺寸改为40pt后，系统、Antonio和DIN Condensed三种字体选择均通过，合成图标实测约80×80px，仍位于标题区左侧，产品名、副标题和刷新按钮保持可见且无裁切。

前后截图只使用纯色合成图标和演示数据，不含真实账号、额度或用户截图。

## 验证与边界

- 定向真实渲染：3种字体选择全部通过；
- 完整Swift门禁：461项普通测试、3项独立刷新调度、18项PTY runner全部通过；
- 6份跨平台合同、发布辅助脚本、236份Markdown和公开安全检查通过；
- 无Widget的macOS arm64 Release App完成资源、Sparkle嵌套组件及严格签名验证；
- 差异复核确认生产代码只改变菜单面板顶部Logo的一组宽高常量。

当前版本保持0.6.3。本项不创建tag、Release或更新源；PR与合并后main双平台门禁在完成后回填。
