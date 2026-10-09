# Antigravity 安全恢复边界规格

**需求：** REQ-20261009-001
**基线：** v0.10.7 / build35，main `4158fea`
**适用范围：** macOS Antigravity；Windows不变。

## 现场状态

M4 Max当前基线由用户截图确认为 AI Token Meter 0.10.7/build35、Antigravity CLI 1.3.1。2026-10-08 23:26 的额度截图显示两个Gemini窗口剩余100%，但浮动条仍显示旧的 service/process error 暂停。额度可读取且额度充足，不等同于应用已验证恢复周期采集；也不说明历史失败的确切触发阶段。

## 可验证的行为

- Google [Headless mode](https://antigravity.google/docs/cli/headless/)说明 `agy -p` 是单次非交互调用，使用缓存凭据；无终端且未认证时返回 `authentication required`，不等待交互。它还要求 `/usage` 作为单独调用，因为CLI自身处理该命令。
- Google [Installation and auth](https://antigravity.google/docs/cli/install/)说明本机CLI使用操作系统密钥环中的会话；找不到已保存会话时会自动打开默认浏览器。
- Google [Model Quotas](https://antigravity.google/docs/cli/commands/usage/)说明 `/usage` 会向后端刷新额度。
- 本地 `ProcessGroupCommandRunner` 将stdin连接到`/dev/null`、stdout/stderr连接管道，不创建PTY，并以独立进程组实施限时和取消清理。这证明本应用不给CLI提供终端交互输入，不证明CLI进程不能调用LaunchServices打开浏览器。
- 普通 `GeminiCollector.collect()` 还运行 `agy -p /model` 与非headless的 `agy models`；测试用fake CLI只证明参数、无TTY、进程组/超时合同，不证明真实`agy`调用外部应用的行为。
- CLI版本校验接受严格三段数字的1.x版本且要求至少1.1.28。测试fixtures覆盖1.1.28和1.2.2；现场截图只确认CLI 1.3.1，未执行真实二进制。未知、格式不符、低于1.1.28或不同主版本会拒绝；目前也没有1.3.1真实CLI行为测试。

## 安全决定

一次用户明确发起的官方登录及成功额度读取，可以刷新本地额度缓存；它不能证明凭据将来过期时的行为，也不能证明其它CLI子命令不会弹出浏览器。因此单次成功不得清除持久 `suspended`，不得恢复启动、定时或唤醒后的周期采集。暂停状态及UI必须继续明确显示。

后台服务/网络/CLI失败当前直接转为`.suspended`，重试次数为0；取消也暂停。已持久化的暂停阻止普通自动和手动刷新再次启动Antigravity；协调器的`inFlight`合并并发请求。持久化写入失败的可靠跨重启封锁仍未被证明，见开发记录；不得将该边界描述成完整安全保证。

只有取得针对CLI版本的明确官方契约或可审计操作系统边界，证明后台命令在凭据缺失/过期时不会启动浏览器、终端登录或其它外部交互，并覆盖所有实际调用命令及持久暂停写入失败路径后，才可重新评估周期自动恢复。未取得前保持暂停；不把成功quota、模拟测试或一次用户登录当作替代证据。

## 验收范围

1. 单次官方登录回执匹配且quota读取成功时，只更新额度缓存。
2. 单次读取成功后，内存和持久`refresh-backoff.json`仍保持Gemini暂停；应用重启后自动/手动刷新都不再调用collector。
3. 登录失败、quota失败、无效响应、取消、并发开始和缓存写入失败均不解除暂停。
4. 提示准确说明额度只读取一次且自动刷新仍暂停。
5. 不运行真实`agy`、访问真实凭据/Keychain、登录、启动URL opener/Terminal/browser或测试设备账号；不发布。

上述验收只证明暂停保持和本地模拟流程，不证明未来可以安全恢复周期查询。
