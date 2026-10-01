# Antigravity静默刷新与异常暂停

需求：`REQ-20261001-001`。用户在 M4 Max 的 macOS 0.10.3/build31 上报告，Antigravity刷新反复打开 Google 授权页，一夜出现数百页；另有 M4 Pro 设备运行正常。该设备及页面数量未能独立复现。本轮只用本地合成命令，不启动真实 `agy`、浏览器授权、Google登录或系统 Keychain。

## 实现

- Gemini 任一额度采集错误会保存终态 `.suspended`；启动、定时刷新、系统唤醒、普通深链、Settings 检查和 Retry 均遵守暂停。其他 Provider 的刷新/退避语义保持不变。
- macOS Services 暂停时提供专属 **Sign in to Antigravity**。登录脚本使用普通交互式 `agy`，退出后通过自定义 URL 返回一次性 UUID 回执。应用只接受当前待处理的回执，并消费后恢复一轮 headless 检查；无效、重放及并发第二次点击均无效。
- `GeminiCollector` 仍只用官方 `agy -p /usage` headless 路径，不注入未经官方文档确认的 `NO_BROWSER` 环境变量。
- `ProcessGroupCommandRunner` 将查询命令创建在独立 session/process group；超时、取消、输出超限或子进程遗留时对该组发送 TERM，再在宽限后发送 KILL。目标是本次本应用启动的进程，按进程名全局终止被禁止。
- 版本为双平台共享 `0.10.4`，macOS build32；Windows运行逻辑不变。

## 失败先行与验证

- 持久暂停专项证明 coordinator 重建后，手动与自动刷新均不调用失败的 Gemini collector；显式清除后下一次调用恢复。
- 命令 runner 使用临时编译的本地 C fixture 派生子进程；取消后确认子进程退出，同时无关的 `/bin/sleep` 控制进程仍存活。
- 登录生成脚本、私有权限文件写入、headless 命令、配置隔离、本地化及一次性回执由独立测试覆盖。
- 真实 M4 Max 现场观察仍须用户更新后确认，自动化结果不冒充现场复现或用户 OAuth 验收。

## 发布事务

PR、main合并、双平台CI、签名Release workflow、公网资产/三个更新源匿名验收、合并与Tag SHA、独立审查结果会在发布完成后补齐。
