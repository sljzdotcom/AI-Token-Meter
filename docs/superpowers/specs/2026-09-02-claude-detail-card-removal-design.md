# AI Token Meter Claude 详情卡片精简设计规格

**日期：** 2026-09-02  
**状态：** 待用户确认  
**范围：** Claude 详情页本机活动区域  
**需求：** `REQ-20260901-008`

## 1. 背景

Claude 详情页当前在每日 Token 趋势下方继续显示 `Token composition` 和 `Top models` 两张卡片。用户确认这两组信息不需要在详情页展示，希望页面更短、更聚焦。

本次采用已确认的方案 A：只精简展示层，不修改本机活动的采集、缓存和领域模型，以保持既有快照兼容性并降低回归风险。

## 2. 目标

1. Claude 详情页不再显示 `Token composition` 卡片。
2. Claude 详情页不再显示 `Top models` 卡片。
3. 保留官方额度、Sessions、Active days、Tokens、每日 Token 趋势、隐私说明、采集状态和更新时间。
4. 保持详情窗口现有宽度、高度上限、自适应屏幕和局部滚动策略，避免点击后窗口尺寸跳变。
5. 不影响 Claude 官方额度、本机 30 日总 Token、缓存兼容、自动隐藏、置前和无障碍交互。

## 3. 界面结构

精简后的顺序为：

1. Claude 标题与当前主要额度；
2. Current session 与 Weekly 官方额度卡片；
3. `Last 30 days · This Mac` 标题；
4. Sessions、Active days、Tokens 三项统计；
5. 30 日每日 Token 趋势图，零活动时显示明确空状态；
6. 本机聚合隐私说明；
7. 状态与更新时间页脚。

`Token composition` 与 `Top models` 不提供折叠入口、占位符或设置开关，也不留下额外空白卡片。窗口尺寸策略保持不变，由现有本机区域滚动容器处理不同屏幕高度。

## 4. 数据与兼容性

- `ClaudeLocalActivityReader`、`ClaudeLocalActivitySummary`、快照缓存格式和敏感文本清理保持不变。
- Input、Output、Cache 仍用于计算每日与总 Token，不再单独可视化。
- 模型聚合字段继续保留在现有版本化快照中，本次不做缓存迁移或格式破坏。
- 删除只服务于这两张卡片的 SwiftUI 展示函数和未再使用的展示辅助类型，避免遗留不可达界面代码。
- 底部隐私说明继续如实说明应用读取的聚合字段。

## 5. 无障碍

- 移除两张卡片后，同时移除它们各自的 VoiceOver 节点。
- 官方额度重置时间、三项本机统计、每日趋势图、空状态、隐私说明和页脚的现有无障碍信息保持有效。
- 不用隐藏或透明视图保留已删除内容，避免 VoiceOver 仍朗读不可见信息。

## 6. 测试与验收

测试先行验证：

1. Claude 详情源码不再调用或定义 `tokenComposition` 与 `modelBreakdown`；
2. `Token composition` 与 `Top models` 两个可见标题不再存在于 Claude 详情视图；
3. Sessions、Active days、Tokens、每日趋势和隐私说明仍存在；
4. 已删除的 Top model 展示辅助代码与专属测试同步清理；
5. 完整测试、Release 构建、签名验证和 `git diff --check` 通过。

真实应用验收：安装候选版，点击 Claude，确认两张卡片完全消失，保留内容连续排列且 8 秒自动隐藏仍正常。

## 7. 非目标

- 不停止采集 Input、Output、Cache 或模型聚合元数据。
- 不迁移或删除旧快照中的现有聚合字段。
- 不新增“更多详情”、折叠按钮或显示开关。
- 不修改 Codex、DeepSeek 详情页。
- 不改变字体、配色、浮动条形状或其他窗口行为。
