# BoundedCommandRunner超时测试可靠性

关联需求：`REQ-20260913-004`。日期：2026-09-13。触发入口：[PR #44](https://github.com/sljzdotcom/AI-Token-Meter/pull/44)。

## 失败与证据边界

发布证据PR首轮macOS workflow `34739695959`只有`BoundedCommandRunnerTests.terminatesAfterTheConfiguredDeadline`失败。被测调用已经得到预期的`UsageCollectionError.timedOut`，随后完整墙钟断言记录1.507秒并要求少于一秒。其余542项主测试通过；相同用例在本机低负载连续20次通过。

旧测试在调用runner之前启动墙钟，把50毫秒超时任务获得调度前的等待、进程启动、超时、停止、退出通知和EOF排空全部计入一秒清理门限。原CI日志无法区分延迟位于超时触发之前还是停止后的清理期间，因此不能据此认定产品清理超限，也不能确认具体调度阶段。该测量缺陷与已经修正的取消测试相同，只是旧超时测试尚未从真实停止请求事件开始计时。

## 失败先行与最小修正

测试先改为调用内部`timeoutDidFire`观察入口，旧实现准确编译失败为`argument passed to call that takes no arguments`。随后`BoundedCommandRunner`增加默认关闭的内部观察闭包，在配置的睡眠完成后、提交`timedOut`停止请求前触发。

修正后的测试使用`ContinuousClock`和锁保护的时间点捕获器，继续要求：超时事件不早于50毫秒、调用返回`timedOut`，并且真实超时停止请求到完成进程清理及返回少于一秒。夹具由2秒改为30秒，避免延迟触发时终止失效仍靠自然退出通过；独立审查的变异验证在延迟触发且禁用停止操作时证明原2秒夹具会约2.006秒后误通过。公开初始化器仍不带观察者；生产命令、超时、信号、0.5秒强制终止、输出和错误语义均未改变。

## 本地验证

- 修正后的定向用例连续20次通过。
- 完整脚本通过543项主Swift、3项独立刷新调度和18项PTY runner，共564项Swift。
- 6份跨平台合同、Windows发布资产归一化、release feed探测、306份Markdown与公开安全门禁通过。

独立审查、PR最终双平台CI、合并节点与main CI将在同一记录中继续补充。
