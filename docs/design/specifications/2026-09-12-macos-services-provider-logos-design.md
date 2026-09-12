# macOS Services 产品 Logo 设计

关联需求：REQ-20260912-003。日期：2026-09-12。

## 目标

macOS Settings → Services 的四个服务分组在产品名称前显示对应 Logo，让用户扫视页面时能快速区分 Claude Code、OpenAI Codex、DeepSeek 和 Google Antigravity。名称、账户状态、按钮、安装登录流程和数据逻辑保持不变。

## 已确认方案

- 复用应用已经打包的四份 Provider Logo 和现有光学校正，不新增第二套图标。
- 每个分组标题使用水平排列：18pt Logo、6pt 间距、现有产品名称。
- Settings Logo 使用系统 `.primary` 前景色，自动适配浅色与深色外观；浮动条和详情页继续使用既有白色样式。
- Logo 作为装饰元素隐藏于辅助功能树，分组标题只朗读一次产品名称。
- 四个分组使用同一标题组件，避免不同产品出现尺寸、间距或对齐差异。

## 备选与取舍

1. **统一单色 Logo（采用）**：最符合 macOS Settings 的信息层级，浅色和深色模式都稳定，品牌轮廓仍足以区分。
2. Provider 强调色 Logo：辨识更强，但会和按钮警告色、状态色竞争。
3. 保留纯文字：改动最小，但不能满足用户快速区分产品的目标。

## 验收

- 四个 Services 分组标题均显示正确产品 Logo 和原名称。
- Logo 视觉框固定18pt、名称间距6pt，并在浅色/深色外观下可见且未裁切。
- VoiceOver/辅助功能只得到一个产品名称，不重复朗读装饰 Logo。
- ProviderLogo 现有浮动条与详情用法的默认白色外观不变。
- Release App继续只使用现有 Logo 资源，不新增网络、账户或数据行为。

