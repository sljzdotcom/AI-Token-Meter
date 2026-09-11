# 悬浮条独立设置页签规格

关联需求：REQ-20260911-009。日期：2026-09-11。

## 目标与映射

双平台Settings新增第五个页签：macOS为`Floating Strip`，Windows英文为`Floating Strip`、简体中文为`悬浮条`。页签顺序固定为Appearance、Floating Strip、Monitoring、Services、About。

- Appearance只保留全局显示项：macOS为Display font；Windows为Language与Display font。
- Floating Strip按“内容与尺寸→屏幕与位置→行为”排列。内容与尺寸包含显示开关（macOS已有）、服务显隐/排序和Comfortable/Compact/Mini；屏幕与位置包含显示模式/显示器（各平台实际已有）与贴边方向；行为包含自动收起、显示/收起延迟和Detail auto-hide。
- Monitoring保留刷新间隔、额度基准、用量提醒、登录启动等监测与运行项；Services与About职责不变。

## 行为与兼容

本次只移动现有控件，不更改偏好字段、默认值、即时保存、服务排序、至少显示一项、显示器回退、键盘切换或滚动行为。由悬浮条右键菜单打开设置时直接进入Floating Strip页签；服务恢复入口继续进入Services。

Windows第五个页签使用16px装饰性线性图标，保留系统字体与中英文名称；左右方向键在五个页签间循环。macOS使用系统Symbol且不新增语言选择。

## 发布边界

本规格授权实现、测试、审查和安全整合；不创建新版本、tag或Release。发布仍需单独授权。
