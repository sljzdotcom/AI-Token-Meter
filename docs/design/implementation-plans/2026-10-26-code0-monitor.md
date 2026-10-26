# Code0 站内额度监控实施计划

> 面向执行 Agent：使用 subagent-driven-development 逐任务实现和独审；root 负责最终判断、隔离 CI、Git 与发布。沿用已经绑定并明确确认的执行模型，不重启已有执行者。普通技术决定及测试通过后的发布已有授权。

目标：完成[Code0 设计](../specifications/2026-10-26-code0-monitor-design.md)的可选第五服务与有条件后台同步。架构：共享被动页面桥、两端纯策略/数值模型、独立原生网页会话、现有 UI 的最小集成。技术栈：Swift/WebKit、Rust/Tauri/WebView2、TypeScript；不新增远程服务。

## 全局约束

原四服务不改账户/采集语义；Code0 默认隐藏、展示不联网、widget仍默认三项。仅明确连接/恢复可以联网；成功后本次运行内至少300秒间隔、每次45秒、失败暂停，重启不自动恢复。Connect登录最长600秒，关闭确认最长10秒。只捕获官方页面的 GET status/self；无凭据、账户字段、主动API请求、付费调用。站内额度无汇率、比例和预算告警。原子保存先于发布；新 attempt 必须等旧 owned 窗口销毁确认；官网 auth refresh 以在途集合/revision隔离旧self，后续有效self才证明可采数值。

实现先写可执行拒绝型契约和测试，由 root 在现有批准的临时 CI 看行为 RED；编译失败不算 RED。每个任务完成后独立审查规格与质量，最终全分支审查一次。执行者不自行提交/推送、运行宿主产品测试、操作 VM/真实账号；root 接管检查点和隔离验证。每项报告明确哪些验证未运行。

### Task 1: 两端数值契约与可选服务兼容

文件：Swift `Domain/UsageModels.swift`、新增 `Code0/Code0WebsiteCapture.swift`、现有 presentation/preferences/widget/CLI exhaustive switch；Rust `domain/usage.rs`、`domain/presentation.rs`、新增 `collectors/code0_capture.rs`；TS `state/usage.ts`；共享 schema/providers/fixture/checker；两端现有偏好及布局测试。

- [ ] 先加两端可编译严格拒绝 capture 入口和测试：`quota=1250000, usedQuota=750000, divisor=500000` 应为余额2.5、累计1.5；0合法，缺除数/字符串/Bool/负数/超安全整数/非有限/未知字段不接受；现有实现返回拒绝使正例行为 RED。
- [ ] 用现有 raw ID/JSON 解码入口验证 `code0/siteCredits/cumulativeUsage`，不把缺枚举导致的编译错误算 RED。先保留原六共享fixture；新增第七额度fixture及断言。
- [ ] RED后实现最小类型、无比例转换和格式化；Swift raw deepSeek及旧JSON兼容保留。类型新增涉及 exhaustive switch 的修改只增加明确 Code0 分支，不接入 CLI 或网络。
- [ ] 默认 provider 集与服务目录分开；旧记录缺 Code0 时追加隐藏，schema Swift6/Windows5；新版本显式选择保留。覆盖旧四服务排列/子集、三可见迁移和五行高度，不把原测试简单常量4改5。
- [ ] `Code0WebsiteCapture` 只接收过滤后的三个数并生成两指标；不接收原始账户JSON。命名在Swift/Rust各自规范下对应同一fixture。任务报告列实际 API 与所有改动文件供后续集成。

具体断言示例：

```swift
#expect(UsageProvider(rawValue: "code0")?.displayName == "Code0")
#expect(capture.balance == 2.5)
#expect(capture.cumulativeUsage == 1.5)
#expect(snapshot.primaryMetric?.usedFraction == nil)
```

```rust
assert_eq!(capture.balance, 2.5);
assert_eq!(capture.cumulative_usage, 1.5);
assert!(capture_with_invalid_divisor.is_err());
```

### Task 2: 共享被动桥及严格消息协议

文件：新增 `web/code0-bridge.js`、`web/code0-bridge.test.mjs`，生成/一致性检查脚本和两端资源接入；各平台纯消息验证器。源脚本以显式 native 配置 nonce/document ID/transport 初始化，不暴露生产测试origin例外。

- [ ] 先写行为 fixture，装入拒绝型桥，确认有效配对没有输出导致 RED；随后实现 fetch/XHR包装与严格URL、GET、响应最终URL、content-type/success、64KiB处理。
- [ ] 测试页面自己的请求才触发捕获；安装桥后请求计数仍0。原fetch返回/异常和XHR事件一致；非目标请求不 clone/read。只发固定control和三个数，token/email/id/header/body不出桥。
- [ ] 覆盖status/self先后，self2先发/self1晚到，解析时revision变化，重叠refresh各request ID完成，refresh结束前已发self不得被接受；明确login/logout清旧值，网络失败/409暂停。200refresh不直接发布，必须后续新self。
- [ ] 生成资源同源检查纳入两端CI；native拒绝大于2KiB、额外字段、非法数、旧epoch/nonce/document/revision/sequence与第二次成功。捕获拒绝原因使用固定枚举，不记录原body。

具体序列：`self1.start → refresh1.start → refresh2.start → refresh1.end → self2.start → refresh2.end → self2.end` 输出0；再 `self3.start → self3.end` 与同文档合法status输出恰1。数值相同的两次请求也按序号隔离。

### Task 3: macOS 原生会话及 Services/detail

文件：新增 `System/Code0WebSession.swift`、`Core/Persistence/Code0SnapshotStore.swift`、`Views/Code0DetailView.swift`；`AppModel.swift`、`ServicesSettingsView.swift`、`FloatingStripView.swift`、provider样式/三语言资源；新增纯生命周期/App合成/实际WKWebView夹具测试。

- [ ] 以注入时钟、窗口驱动、store先写测试：未连接零创建；busy不重复；299秒拒绝/300允许；保存失败不发布；关闭确认晚到不建新窗口；断开后旧callback不能写；重启缓存不恢复授权。
- [ ] 实现 dedicated WKWebsiteDataStore 与 OS profile lease，串行状态机/nonce/attempt，主框架来源检查，成功先save后发布再关闭。定义独立WebSession协议，App测试可注入，不把实际网页登录作为单元测试前置。
- [ ] 首次load前配置 WKUI/导航/下载/挑战的明确拒绝；隐藏模式不show/focus。用CI受控页面验证实际window.open/下载/协议/对话框/权限/认证取消和owned对象释放，不能以源码字符串测试替代行为。
- [ ] 登录完成按钮先关闭登录实例再创建新隐藏验证。普通成功300秒后调度；任何错误暂停，重新启动需明确恢复。Disconnect仅清Code0，失败保留cleanupPending。
- [ ] App四项refresh与Code0回调合成使用当前状态；Services账户读取不覆盖它。三语言Connect/登录完成/同步/恢复/断开、时间、旧缓存、忙/暂停清楚；无API Key/CLI入口；详情纯展示无onAppear加载。

### Task 4: Windows 原生会话及 Services/detail

文件：新增 `platform/windows/code0_web_session.rs`、`code0_webview.rs`；`lib.rs`state/commands、独立数值store；TS Services/detail/useUsageSnapshots；providerlogo/tray/prefs/nativewindow五行。独立测试 `code0_web_session.rs`、`SettingsCode0.test.tsx`、`Code0Detail.test.tsx` 与实际WebView2夹具。

- [ ] 先按同一状态/失败时序写RED，数值缓存先保存后发布；不要复用现有忽略保存错误的普通complete_success路径。
- [ ] 原生exclusive profile lease、唯一owned窗口与destroy回调，remote窗口无Tauri capabilities。初始化前注册navigation/new-window/download及WebView2 permission/dialog/auth等必要拒绝handler；正常TLS不绕过。
- [ ] 受控fixture验证真实native拒绝/隐藏/销毁，组件测试验证仅点击明确动作才invoke；独立scheduler遵循300秒、sleep合并、busy和paused门禁。
- [ ] 普通collector继续四项，由独立会话合成Code0；旧普通刷新/Disconnect迟到结果不能复活。三语言、无币种/比例、最后成功时间和可选第五项同macOS。

### Task 5: 跨平台验收与已授权发布

- [ ] 整分支独审读取前四任务报告、实际CI和当前差异，处理阻塞发现；不要重复运行已通过且未变化的测试。
- [ ] 正常完整Swift测试/Windows Rust+前端+nativeGUI、两端实际网页夹具、合同、文档和公开扫描全部通过。真实网站账户未测单列，不以合成fixture冒充。
- [ ] 版本按当前main递增；正常签名打包/PR合并/Release/三个更新源，匿名下载验证资产摘要及签名。保留现有macOS签名与Windows签名能力边界，不声称额外公证。
- [ ] 更新唯一台账、开发记录与索引并向协调入口回传提交/测试/发布/现场边界。003现场与005监听不因004发布自动完成。
