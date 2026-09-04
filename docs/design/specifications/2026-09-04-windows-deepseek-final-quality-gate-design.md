# Windows DeepSeek 最终质量门禁修复设计

**需求：** `REQ-20260904-006`

**日期：** 2026-09-04

**状态：** 已确认；用户授权采用推荐技术方案持续修复直到完成

## 目标

关闭上一轮最终定向复审留下的四项 Important：分片传输停滞、初始查询覆盖新会话、命令失败覆盖已确认会话，以及已有历史时清理失败无恢复入口。修复必须保持 macOS 源码和视觉不变，不扩大 DeepSeek 官网访问范围，不缓存登录信息或网页原文。

## 根因

1. `DeepSeekHistoryAssembler` 会从首个有效分片开始计算 20 秒有效期，但 WebView 运行时只安装了 30 秒页面就绪计时器和 15 分钟交互会话计时器。首片进入 `Waiting` 后没有异步任务主动触发失败，缺片时只能等到 15 分钟总时限。
2. 详情页注册状态监听后立即执行初始查询，却没有保存发起查询时的 attempt。用户可以在查询返回前点击同步；旧查询随后以 authoritative 模式覆盖新会话的 opening 状态。
3. `open_deepseek_history` 失败后的恢复查询异常或返回无效值时，前端无条件写入匿名 failed。若监听已经确认同一 attempt 的 generation 正处于 opening/active，这会错误停止轮询并恢复详情自动隐藏。
4. 历史聚合会先写入快照，再销毁官网窗口。销毁失败后状态为 failed 且保留 `cleanup_pending`，但当前组件只在历史为空时渲染同步/重试按钮；已有图表会把唯一恢复入口隐藏。

## 推荐方案

### 分片传输停滞计时

- 在 `DeepSeekHistoryWindowRuntime` 增加唯一的 `transfer_timeout` 句柄，和 generation 一起保存。
- 只有 coordinator 接受首个有效分片并返回未完成时才安装 20 秒计时；单分片直接完成时不安装。
- 计时到期只能对相同 generation 取得 failed 终态所有权；旧 generation 的回调不得改变新会话。
- completed、cancelled、failed 以及任何清理路径统一取消 ready、transfer、session 三类计时器。
- coordinator 提供可测试的“分片已经开始”和“取得传输超时终态”边界，避免运行时凭 UI 状态猜测。

### 前端 attempt 与 generation 所有权

- `deepseekHistoryAttempt` 是详情实例内单调 epoch。初始状态查询保存发起时的 epoch，只有返回时 epoch 未变化才能应用。
- 点击同步先递增 epoch，再清空旧 generation，并保存旧 generation floor；旧事件和旧查询均不能越过 floor。
- 所有合法状态查询结果都进入同一个 generation 感知接收函数，不为 completed/cancelled/failed 编写旁路。
- 命令或恢复查询失败时，仅在当前 attempt 尚未确认任何 generation 为 opening/active 时显示本地 failed；已经确认的活跃会话保持原状并继续轮询。

### 清理失败恢复

- `DeepSeekHistory` 在存在图表时仍渲染同步状态区域。failed 时显示固定错误和 **Try again**，opening/active 时显示只读进度；普通 completed/idle 时不增加视觉噪声。
- **Try again** 继续调用现有 `open_deepseek_history`；后端先执行残窗核对和 `cleanup_pending` 释放，再创建新 generation。
- 收到真实 `Destroyed` 事件时，如果当前 generation 正在等待清理，直接核销逻辑所有权；若仍为正常活动会话，则走既有 cancelled 终态。

## 状态与错误规则

- 对外仍只暴露 `idle/opening/active/completed/cancelled/failed`，不增加携带内部错误或网页内容的新状态。
- 任何旧 attempt、旧 generation 或降级状态均被忽略；同 generation 只允许状态单调前进。
- status channel 不可用时仍禁用同步按钮；已有图表不因此消失。
- cleanup failed 只显示固定本地文本，不暴露路径、Cookie、API Key、账号或官网响应。

## 测试合同

1. Rust：首个有效分片后未收到后续分片，传输时限取得 failed 终态；完成或其他终态后旧计时不能再次取得所有权；失败清理后 `Destroyed` 或重试可以释放并创建新 generation。
2. React：旧初始查询在点击同步后返回，不能覆盖 opening；事件确认 active 后命令与恢复查询都失败，仍保持 active；completed/cancelled/failed 查询均走统一接收路径。
3. React：已有 30 天历史且状态 failed 时仍显示错误与 **Try again**，点击后调用同步；opening/active 时图表保留且详情不自动隐藏。
4. 回归：完整前端、Rust、production 密度浏览器门禁、macOS 测试、文档与公开安全检查全部通过；Windows-only 构建由 `windows-latest` 验证。

## 验收边界

代码审查不得遗留 Critical 或 Important。自动化与 Windows runner 全绿后才能合并；合并后才生成下一版 Preview。真实 Windows 11 上的登录、关闭、复用聚焦、字体下拉和图表恢复仍按原需求清单逐项验收，不能由 macOS 本机构建冒充。
