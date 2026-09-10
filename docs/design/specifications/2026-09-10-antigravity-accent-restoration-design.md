# Google Antigravity 详情强调色恢复设计

**需求：** REQ-20260910-013  
**状态：** 已实现，待随0.7.3发布

## 目标

恢复Google Antigravity详情中已经确认的青绿色品牌强调：标题、额度数值和进度使用与Gemini来源一致的渐变/颜色；说明、重置时间、错误与操作按钮继续使用现有语义色。详情的深海不透明底板保持不变。

## 跨平台行为

- macOS沿用`AIMeterProgressBar`和Gemini渐变令牌，标题及四个额度的剩余百分比使用同一强调角色。
- Windows标题与额度值使用同源青绿色，进度条沿用Provider颜色变量。
- fresh、cached、unavailable三种状态都保留可识别的强调色；无数据时不伪造百分比。
- Claude Code、OpenAI Codex与DeepSeek的颜色映射不变。

## 验收

- 原生macOS三种Antigravity详情状态的位图采样均能识别青绿色强调和不透明底板。
- Windows真实浏览器覆盖八种Antigravity状态，标题、额度数值与进度保持同源强调色。
- 文本对比度、额度语义、缓存标识、Retry与安装入口不变。
