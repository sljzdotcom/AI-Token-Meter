# Mini 浮动条与自动收起修正

关联需求：REQ-20260911-001、REQ-20260911-002、REQ-20260911-003、REQ-20260911-004。日期：2026-09-11。

## 结果

双平台的浮动条尺寸固定为 Comfortable 108pt/px、Compact 78pt/px、Mini 65pt/px，Settings 只按该顺序显示三项。旧设置中的 `compact` 保持 Compact，因此升级后使用78pt/px；用户主动选择Mini后才切换到已确认的65pt/px连续深海轮廓。Mini与Compact使用48pt/px圆环和10pt/px间距，Comfortable保持60pt/px圆环和12pt/px间距。

Appearance新增自动收起开关，默认开启并持久化。关闭时两端状态机都会取消待执行计时并立即展开；显示与收起延迟保留原值，控件暂时禁用，重新开启后继续使用。Settings底部提示弧改为深色；主体悬停只展开浮动条，只有底部弧区悬停或键盘焦点才显示齿轮。

## 验证

失败先行测试准确捕获旧实现缺少Mini、Compact仍为65、偏好schema缺少自动收起字段、Settings只有两档且整条悬停显示齿轮。修复后的本机证据：

- macOS完整门禁：472项普通测试、3项独立刷新调度、18项PTY runner，共493项；跨平台合同、发布脚本回归、269份文档与公开安全检查通过。
- Windows前端133项、production build、Rust 246项、格式和严格Clippy通过。
- 真实Chrome完成三档、左右贴边和1至4服务共24组布局；每个圆环36点周界采样、点击/拖动、Settings静止/焦点/底弧状态、三档固定顺序，以及自动收起关闭后的延迟禁用与值保留均通过。
- 无Widget macOS Release App完成资源便携性、Sparkle嵌套组件、严格代码签名、更新Bundle和arm64产物验证。

物理Windows 11的真实指针、多显示器与125%/200% DPI仍属于现场验收边界；自动化不把这些环境操作冒充为已执行。首次独立审查的Critical/Important/Minor为`0/2/2`，底弧命中、用户指南、旧schema对称性与Compact测试尺寸全部修复；最终复审为`0/0/1`，唯一测试命名Minor也在候选提交前关闭。最终功能与0.8.0版本候选提交为`26e6b61`；PR/main精确CI与发布证据在后续节点补入本记录。
