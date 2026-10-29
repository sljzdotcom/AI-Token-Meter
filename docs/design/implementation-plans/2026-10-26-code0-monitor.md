# Code0 站内额度监控实施计划

> 面向执行 Agent：使用 subagent-driven-development 逐任务实现和独审；root 负责最终判断、隔离 CI、Git 与发布。沿用已经绑定并明确确认的执行模型，不重启已有执行者。普通技术决定及测试通过后的发布已有授权。

目标：完成[Code0 设计](../specifications/2026-10-26-code0-monitor-design.md)的可选第五服务与有条件后台同步。架构：共享被动页面桥、两端纯策略/数值模型、独立原生网页会话、现有 UI 的最小集成。技术栈：Swift/WebKit、Rust/Tauri/WebView2、TypeScript；不新增远程服务。

## 全局约束

原四服务不改账户/采集语义；Code0 默认隐藏、展示不联网、widget仍默认三项。仅明确连接/恢复可以联网；成功后本次运行内至少300秒间隔、每次45秒、失败暂停，重启不自动恢复。Connect登录最长600秒，关闭确认最长10秒。只捕获官方页面的 GET status/self；无凭据、账户字段、主动API请求、付费调用。站内额度无汇率、比例和预算告警。原子保存先于发布；新 attempt 必须等旧 owned 窗口销毁确认；官网 auth refresh 以在途集合/revision隔离旧self，后续有效self才证明可采数值。

实现先写可执行拒绝型契约和测试，由 root 在现有批准的临时 CI 看行为 RED；编译失败不算 RED。每个任务完成后独立审查规格与质量，最终全分支审查一次。执行者不自行提交/推送、运行宿主产品测试、操作 VM/真实账号；root 接管检查点和隔离验证。每项报告明确哪些验证未运行。

### 并行门禁修复：REQ-20261028-003 状态发布与保留契约（2026-10-28）

Mac40714996064 已独立接受受控空读RED与完成发布对照（10616/790f312f）；历史9ac内部先后仍不倒推。允许按下面的窄方案修复公共CLI状态发布和必要停止交接，不更改其他启动协议、实际watchdog分类/信号或原一秒测试期限。

- 仅prod CLI builder、原CLI测试、Mac ci.yml三个文件。覆盖supervisor两处不确定状态和zsh report_status：同目录独占唯一临时文件、0600、完整写入/检查close/真实rename；失败仅清理本次所有的临时资源，保留旧正文并返回失败，不原地截断回退、不修改exec_status启动协议。
- 发布失败与已知进程不确定性单调锁存；不确定性必须在尝试保留状态发布之前置位。新增短命shell发布者必须实际结束并回收后才处理延期取消/最终ACK，不能留下迟到写入者；若无法确认结束则保持未确认，不把wait被信号打断当作已退出。 极端未确认时允许helper保持保留态等待owned join，原登录期限不延长或重置；query未知/EPERM/可能复用PID均不能当ESRCH，不按未核对数值PID强杀。负例本身最终也必须有真实自有清理证据，不能靠外层watchdog超时冒充通过。
- 2026-10-29窄化提交位点：既有launcher看到helper_cancelled就可立即清理，因此其可见点也必须晚于发布子进程真实回收。shell子进程只对父已知的独占同目录staging执行创建/0600/完整写入/检查close并退出；父确认join后以zsh/files的builtin zf_mv直接rename提交，不另生提交子进程、不调用外部mv或复制回退，模块缺失保守失败。父提交前只接受私有owned regular/absent目标，拒绝目录、符号链及其他类型；原最终.stopped ACK也以同一父builtin提交，精确正文及应用解析不变，不留下外部mv提交子进程。supervisor两处保持进程内Perl原子提交。v2的真实query42→guardian发布因果链仍在该进程内的真实rename前/完成后受控；另以shell定向IO/interruption回归证明parent join→builtin提交顺序，不伪换guardian路径。实际模块可用性、故障及信号交错仍须隔离CI验证。静态依据为[zsh官方源码](https://github.com/zsh-users/zsh/blob/master/Src/Modules/files.c)及[Apple发布源码](https://github.com/apple-oss-distributions/zsh/blob/main/zsh/Src/Modules/files.c)：BIN_MV置MV_ATOMIC并直接rename，目标unlink仅用于非原子分支；尚不以源码阅读冒充目标主机实测。
- 允许最小新增停止许可，复用已有.status.stopped路径，严格格式guardian_stop_accepted_v1:PID:nonce。仅真实匹配app取消/guardian-stop请求被观察、此前无不确定或发布失败、所需直接子回收条件已检查后，才能在组清理前发布；后续新不确定性不能被旧许可覆盖。许可只证明接受请求，绝非清理完成。
- 父shell主等待及stop_owned_processes两路对请求式停止均核对同实例PID/nonce许可，仍需原reap、真实组不存在、watchdog和前台恢复检查。ready下无有效许可的异常supervisor终结保守保留guardian、禁止receipt/完成ACK；不得由timeout路径绕过。正常CLI退出及既有普通非信号错误合同保留；无法区分的信号式退出可保守保留并增加定向回归。已在真实确认清理后移除的状态不因后续文件失败伪造重建。
- guardian许可与原helper_stopped_v1精确格式互斥，应用侧原完成ACK解析不扩大接受；最终helper ACK仍只在全部所有者结束后产生。stopped及其现有临时路径加入碰撞校验。不能承诺不可通信/外部强杀等所有故障均能持久记录原因，但新增原子发布失败不得造成新的ready成功、假取消确认或未确认guardian移除。
- 活动受控测试升级v2，旧v1提交/冻结/原始RED永存，原未插桩目标及五断言不变。prepublication臂停在真实staging完整写入/close后、真实rename前；实际watchdog读旧ready并完成原分支/期限证明后才放行rename，再确认真实preserve读及最终决策。published臂真实rename成功后放行read并确认原条件信号语句执行完毕；两臂shell均等待最终post-decision ACK再清理。
- v2两臂都要求真实query exit42、发布成功、读/决策在原期限内、正常cleanup、guardian保留、准确process_state_unavailable、无callback/USR2 trap/取消确认。缺失/前置失败/超时一律拒绝；增加temp-create/write/close/rename失败、许可缺失/错误身份/发布者未回收的窄回归，核对不新增确认成功。严格validator只接受schema v2、actual exit0及两臂完整正向事实；保留原字节上限、身份/hash、exact keys/types/重复与非有限拒绝及原3job/全部门禁，不兼容放行v1。

2026-10-29首份候选源码互补审查：SPEC 12153/4794575f为C0/I4/M0、QUALITY 12605/3df1fe7c为C0/I5/M0；Root均全文接受发现，候选SOURCE HOLD、新实际行为全部未运行。下一fix1仅集中修复以下五组，不推已知有缺陷的旧候选或覆盖其19成员冻结/报告：

- 状态提交后的延期取消必须一次性、不可重入，不无条件借finish_launch_claim重新消费仍存在的.cancel/未消费标志。新增使用真实最终取消函数、保留取消请求及真实owned清理的回归，证明正常130退出、确认后不再生发布者；不能以替换真实handler的marker stub或应用及时删除文件为证明。
- exec探测waitpid返回非EINTR/失去所有权后锁存不确定，不能根据child_done=false重新登记数值PID或向其发信号；拥有/完成/失去所有权分开，不把unknown假作已回收。增加无未核对PID信号的受控所有权丢失回归。
- fork到父登记及子handler切换必须防止可发许可的清理在登记前介入；只移动赋值位置不足。保持原exec授权协议/期限，父子信号状态都正确恢复，受控关键窗口证明必要直接子确实回收后才有停止许可，不把最终group-ESRCH等同此先决条件。
- 窄修既有group_peers真实夹具的提取边界，避免把新停止closure自由变量带进use strict程序；保留原实际query/live-peer断言，不删用例、伪造控制文件或把编译失败当行为RED。
- 只在两份新IO/join子夹具内明确私有绝对root工作目录或使用全绝对路径，所有证据与清理留在该root。不要全局修改shared native spawner/test进程cwd；证明真实负例与正常join退出，不能靠ambient旧标记或外层超时。

fix1另存完整三文件HEAD前/后及相对首候选的差异、保护清单与新冻结；原五断言/watchdog整行、一秒期限、strict v2及全部旧门禁保持。修后仍须互补源码复审和隔离实际完整验证，不能把静态关闭发现称实际修复通过。

fix1互补定向复审（SPEC 11976/40bcff79、QUALITY 13016/98259e79）均C0/I2/M0，原五个具体路径已有源级修正，Root全文接受剩余两项后仅授权fix2：

- 初始STOP循环的真实waitpid终结/失去所有权与随后清除pre_exec_child/child_owned/STOP状态，必须对可执行清理的TERM/INT/HUP处理成为同一不可中断转换；不能在kernel已回收后仍以旧owned标志发数值信号。只保护必要短非阻塞wait及状态更新，准确保存/恢复原mask，失败先保守锁存；不长期屏蔽信号、不延后取消或扩大期限。新增关键回收窗口的安全回归，在任何未核对PID信号的系统调用前拦拒，不制造真实PID复用受害者或运行旧危险路径。
- 新fork窗口PTY回归必须正向证明在外层fallback信号前自然完成。允许共享观察器仅新增被动natural/forced事实，不改原超时、信号或失败清理行为；新case要求自然完成、未用外层救场并保留所有现有状态/清理断言。事实必须在真正进入fallback/发送HUP、TERM、KILL前锁存，不能由最终正常130或cleanupSucceeded反推；若外层介入应正常保留清理但该回归失败，不能只加宽松耗时断言。

fix2保持原首候选19、fix1的28成员与所有作者/双审报告不可变，另存HEAD完整差异和fix1→fix2窄差异。完成后同两peer复审，所有实际编译/行为/GREEN仍待隔离验证。

### Task 1: 两端数值契约与可选服务兼容

文件：Swift `Domain/UsageModels.swift`、新增 `Code0/Code0WebsiteCapture.swift`、现有 presentation/preferences/widget/CLI exhaustive switch；Rust `domain/usage.rs`、`domain/presentation.rs`、新增 `collectors/code0_capture.rs`；TS `state/usage.ts`；共享 schema/providers/fixture/checker；两端现有偏好及布局测试。

- [x] 先加两端可编译严格拒绝 capture 入口和测试：`quota=1250000, usedQuota=750000, divisor=500000` 应为余额2.5、累计1.5；0合法，缺除数/字符串/Bool/负数/超安全整数/非有限/未知字段不接受；现有实现返回拒绝使正例行为 RED。
- [x] 用现有 raw ID/JSON 解码入口验证 `code0/siteCredits/cumulativeUsage`，不把缺枚举导致的编译错误算 RED。先保留原六共享fixture；新增第七额度fixture及断言。
- [x] RED后实现最小类型、无比例转换和格式化；Swift raw deepSeek及旧JSON兼容保留。类型新增涉及 exhaustive switch 的修改只增加明确 Code0 分支，不接入 CLI 或网络。
- [x] 默认 provider 集与服务目录分开；旧记录缺 Code0 时追加隐藏，schema Swift6/Windows5；新版本显式选择保留。覆盖旧四服务排列/子集、三可见迁移和五行高度，不把原测试简单常量4改5。
- [x] `Code0WebsiteCapture` 只接收过滤后的三个数并生成两指标；不接收原始账户JSON。命名在Swift/Rust各自规范下对应同一fixture。任务报告列实际 API 与所有改动文件供后续集成。

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

- [x] 先写行为 fixture，装入拒绝型桥，确认有效配对没有输出导致 RED；随后实现 fetch/XHR包装与严格URL、GET、响应最终URL、content-type/success、64KiB处理。
- [x] 测试页面自己的请求才触发捕获；安装桥后请求计数仍0。原fetch返回/异常和XHR事件一致；非目标请求不 clone/read。只发固定control和三个数，token/email/id/header/body不出桥。
- [x] 覆盖status/self先后，self2先发/self1晚到，解析时revision变化，重叠refresh各request ID完成，refresh结束前已发self不得被接受；明确login/logout清旧值，网络失败/409暂停。200refresh不直接发布，必须后续新self。
- [x] 生成资源同源检查纳入两端CI；native拒绝大于2KiB、额外字段、非法数、旧epoch/nonce/document/revision/sequence与第二次成功。捕获拒绝原因使用固定枚举，不记录原body。

具体序列：`self1.start → refresh1.start → refresh2.start → refresh1.end → self2.start → refresh2.end → self2.end` 输出0；再 `self3.start → self3.end` 与同文档合法status输出恰1。数值相同的两次请求也按序号隔离。

### Task 3: macOS 原生会话及 Services/detail

文件：新增 `System/Code0WebSession.swift`、`Core/Persistence/Code0SnapshotStore.swift`、`Views/Code0DetailView.swift`；`AppModel.swift`、`ServicesSettingsView.swift`、`FloatingStripView.swift`、provider样式/三语言资源；新增纯生命周期/App合成/实际WKWebView夹具测试。

- [x] 以注入时钟、窗口驱动、store先写测试：未连接零创建；busy不重复；299秒拒绝/300允许；保存失败不发布；关闭确认晚到不建新窗口；断开后旧callback不能写；重启缓存不恢复授权。
- [x] 实现 dedicated WKWebsiteDataStore 与 OS profile lease，串行状态机/nonce/attempt，主框架来源检查，成功先save后发布再关闭。定义独立WebSession协议，App测试可注入，不把实际网页登录作为单元测试前置。
- [x] 首次load前配置 WKUI/导航/下载/挑战的明确拒绝；隐藏模式不show/focus。用CI受控页面验证实际window.open/下载/协议/对话框/权限/认证取消和owned对象释放，不能以源码字符串测试替代行为。
- [x] 登录完成按钮先关闭登录实例再创建新隐藏验证。普通成功300秒后调度；任何错误暂停，重新启动需明确恢复。Disconnect仅清Code0，失败保留cleanupPending。
- [x] App四项refresh与Code0回调合成使用当前状态；Services账户读取不覆盖它。三语言Connect/登录完成/同步/恢复/断开、时间、旧缓存、忙/暂停清楚；无API Key/CLI入口；详情纯展示无onAppear加载。

Task3底层受控夹具已在 `9bd45db` 通过；媒体设备前置分支有明确引擎证据，真实媒体权限委托/TCC与证书/TLS握手的未观察边界仍保留，不视为真实账户验收。App接线与界面已在`4838bbc`完成实际定向验收，12项窗口回归通过；原有终端测试观察器的失败由REQ-20261027-001修正，整体完整测试和发布仍待Task5，不以组件通过覆盖该门禁。

### Task 4: Windows 原生会话及 Services/detail

文件：新增 `platform/windows/code0_web_session.rs`、`code0_webview.rs`；`lib.rs`state/commands、独立数值store；TS Services/detail/useUsageSnapshots；providerlogo/tray/prefs/nativewindow五行。独立测试 `code0_web_session.rs`、`SettingsCode0.test.tsx`、`Code0Detail.test.tsx` 与实际WebView2夹具。

- [ ] 先按同一状态/失败时序写RED，数值缓存先保存后发布；不要复用现有忽略保存错误的普通complete_success路径。
- [ ] 原生exclusive profile lease、唯一owned bare Window与destroy回调；直接COM创建专用WebView2，不安装Tauri/Wry WebView及其内部IPC。使用已锁定版本的直接依赖与公开gated WindowBuilder，UI/STA异步确认environment/controller、拒绝handler及桥脚本注册完成后再导航；覆盖navigation/new-window/download/permission/dialog/auth，正常TLS不绕过。
- [ ] 受控fixture验证真实native拒绝/隐藏/销毁，组件测试验证仅点击明确动作才invoke；独立scheduler遵循300秒、sleep合并、busy和paused门禁。
- [ ] 普通collector继续四项，由独立会话合成Code0；旧普通刷新/Disconnect迟到结果不能复活。Windows既有英语/简体中文、无币种/比例、最后成功时间和可选第五项与macOS相应行为一致；macOS继续既有三语言，不新增Windows语言系统。

#### S2 分片：原生拒绝处理器安装与实际事件分开验收（2026-10-28）

Windows40709469737 已独立确认真实同实例十项缺失、拒绝加载且正常清理/exit1，作为安装分片的有效RED。先完成以下S2-1，不把安装GREEN视为事件拒绝或生产接入完成；后续S2-2再逐项补真实刺激与拒绝/清理证据。

2026-10-29进展：S2-1源码独审15964/417b805a及实际独审14364/d967cdb3均C0/I0/M0，Root分别全文接受。fc2096f/真实7644c7f的Windows40726949365正常完整成功，十项实际token库存1023、只读同实例receipt1、未导航、exit0和正常释放/TempClose通过；原45/10/75及旧门禁保持。这只结项安装分片，下面S2-2及S3–S5仍未完成。

- S2-1仅driver及选定S2 fixture接线：增加显式真实安装动作，随后沿用只读同实例receipt；不让读取库存的方法暗中执行安装。测试仍不导航、不加载页面，原schema6正向分支/actual exit0/真实normal cleanup必须成立，workflow判定与旧case不变。
- 十项状态只能来自同一owned原生emitter的真实add成功与token库存：top/child导航、外部协议、new-window、download、script-dialog、permission、HTTP basic-auth、client-certificate、TLS-error；Some(0) token有效。接口不可用或部分失败保持真实部分库存并拒绝加载，不能标全Installed、伪造全缺失或吞掉错误。
- 现有官方HTTPS同源策略用于top/child导航；其余同规格明确拒绝，不接受TLS例外。下载与客户端证书的两个必需setter均各尝试一次；一项失败不能阻止尝试另一项。不增加外部协议/系统对话刺激，不改信任、权限、CA、账号或真实网站环境。
- 每个callback使用弱owner、匹配request/generation/emitter及UI/STA约束、现有Active/HandlerLifetime库存；Owner借用外不调用COM/原生API、不保存事件args、不增加强owner环。正常拒绝与setter/身份/移除不确定分开处理，后者维持sticky保护；stale/unknown不得默许原生默认行为或改写新attempt。
- add/remove必须同一emitter和确切token，remove单次尝试、不能重试/零token当缺失/预先清状态；失败保留未确认状态。任何路径的局部native引用先纳入现有库存或Active，完成所有解绑/回调/原生对象释放后沿用真实BrowserProcessExited/Destroyed/Released/run_return/一次TempDir.close证据。保持45/10/75、所有旧断言、计数/上限及既有P0–P9行为。
- 独立源码SPEC+QUALITY及实际S2安装正向验收后，才开始S2-2事件覆盖；未到目标事件的前置拒绝不能算该事件已测。文件选择/CDP、被动bridge、production registry/lease/clock/clear另属S3–S5，不混入此安装分片。

#### S2-2 N1：真实顶层导航拒绝的首片（2026-10-29，候选口径待独立窄审）

S2-1源码与实际独审均已接受。下一片只增加现有处理器的真实受控覆盖，不预判其已有实现错误、不制造RED；正式计划唯一在本文件，私有准备材料仅作只读依据。新源码GO须在本节独立窄审后发给唯一作者。

- 同一位新派Sol6.1/high作者拥有driver、fixture及Windows workflow的独立追加调用/strict validator，Root保留正式计划、整体审核及Git所有权。Mac三候选、四parked、Cargo、UI与所有旧case不碰。固定选择器`refusal-top-owned-file`、独立schema7，保留原schema1–6及S2安装不导航合同。
- 同一新鲜进程、STA、独占profile/Window及既有TempDir内以create_new建立固定叶名零字节文件，只从该自有路径生成file URI并绑定request/generation/case。源路径必须canonical后仍是自有canonical TempDir的准确固定叶名，Windows prefix只接受Disk/VerbatimDisk；生成URI必须无host/userinfo/query/fragment。UNC/VerbatimUNC/device/非本地authority、转换或长度异常均是setup failure且不得Navigate，不做字符串前缀比较或强行改写URI。十项真实安装与只读同实例1023库存确认后，保留现有Active/弱owner/原生identity/重入保护，借用外仅提交一次原生Navigate；不读取文件正文，不接收任意URL/path，不叠加P1、HTTP/服务传输、白名单或信任例外。这里只排除新增浏览器网络刺激，不承诺盘符底层存储绝无共享IO。
- 证据只能来自实际top NavigationStarting：准确自有URI在原生端内部匹配、NavigationId getter成功且不把0当缺失、IsRedirected=false、同sender/request/generation/live匹配。真实SetCancel一次成功后读取原生Cancel=true；只发布固定enum/bool/有界计数与顺序事实，不输出URI/path/ID/token/nonce或对象Debug。重入事件可早于Navigate返回，不能要求错误的先后关系。
- 原生事件及动作之后必须有匹配PolicyRejected一次；复用真实Context::invoke置failed/closing、poll在借用外经events(request,event)单次发出Failed(PolicyRejected)及fixture真实Session观察的通道，不从Cancel/receipt推算或人为填1，不叠加P1。照常立即关闭，不等观察消息、不重置期限。要求真实BrowserProcessExited/Destroyed/Released/ref0/run_return/一次TempDir.close、无新增owned窗口、actual exit0及原45/10/75。缺目标事件/读回/身份/安装或清理失败均为失败或未覆盖，不skip/PASS、不把前置失败称行为RED。
- 新schema7与exact fresh run/case只接受selected=completed=1、事件与Cancel/读回/PolicyRejected准确单次及所有setup/lifecycle正向事实；拒绝多余/重复键、Bool充整数、非有限、错误身份、缺/多case、超限和陈旧拼接。Root追加新鲜产物及真实退出记录、严格正向校验和保留pass字段的关键事实数据突变负例，独审整个三文件组合后再隔离CI。若既有处理器直接GREEN如实记录；若实际出现缺陷，先保留证据再窄修。
- N1源码和实际接受后再次冻结停止。child/redirect、popup/download/dialog/permission/external及真实401/TLS/client-cert均另行锁定：synthetic302不保证实际redirect，当前策略阻断loopback/P1不能直接叠加，TLS与client-cert回调先后未保证。不得通过允许系统动作、导入CA/忽略TLS/证书例外/凭证/真实账号来强造可达性。

N1首份源码互补审查（SPEC 24188/69dd6397、QUALITY 20434/585c0803）均C0/I1/M0，Root全文接受同一错线程observer查找发现：invoke拒绝off-UI后receipt=None，但新wrapper仍无条件访问UI Weak/Rc/RefCell。仅允许driver窄fix1，使observer查找/发布只发生在真实Some(receipt)的已过UI路径；None/affinity拒绝不得触UI-local owner。保持原atomic unknown/affinity、args/sender及临时原生对象的Active/Drop、无target旧委托和所有语义/数据/时限；fixture及Windows workflow首候选字节保持。准备阶段的local-URI I1已关闭，与本次source I1分开保留。另存原20成员、双审报告及修正delta，修后两peer定向复审和实际隔离验证；不执行错线程旧路径制造RED、不把静态修正当原生通过。

### Task 5: 跨平台验收与已授权发布

- [ ] 整分支独审读取前四任务报告、实际CI和当前差异，处理阻塞发现；不要重复运行已通过且未变化的测试。
- [ ] 正常完整Swift测试/Windows Rust+前端+nativeGUI、两端实际网页夹具、合同、文档和公开扫描全部通过。真实网站账户未测单列，不以合成fixture冒充。
- [ ] 版本按当前main递增；正常签名打包/PR合并/Release/三个更新源，匿名下载验证资产摘要及签名。保留现有macOS签名与Windows签名能力边界，不声称额外公证。
- [ ] 更新唯一台账、开发记录与索引并向协调入口回传提交/测试/发布/现场边界。003现场与005监听不因004发布自动完成。


### 2026-10-29 验证排程更新（不改变任务或验收口径）

Mac fix2定向SPEC9918/b770d125与QUALITY9917/28c4f2ef源码C0/I0/M0，Root全文接受；N1 fix1双源码复审同样已接受、本地提交4b11fd8。两组均实际UNRUN；在任何新推送前改为同一个获审源码检查点进入隔离CI，执行新Mac v2与完整旧门禁、Windows N1 strictschema7/真实效果及全部旧门禁、独立dispatcher。四parked继续保留，当前源码接受不授予实际通过或Task4/5完成。后续实际证据须绑定新提交实际checkout，另经独立审查，不使用工作区未提交字节冒充已测源码。


#### S2-2 N2 条件准备：固定空白页上的真实 child 导航拒绝

本节是未来单片的准备/审查基线，尚非源码GO；N1实际独审接受后才可按本节发唯一作者。Root已全文读取只读准备18427/4a55e897，允许在隔离debug+fixture环境评估下述固定DOM刺激作为普通实现选择，不扩展生产脚本/API或原S2授权。不得把未达目标事件当覆盖。

- 独立固定selector `refusal-child-initial-data`、schema8、新鲜进程/profile/TempDir/唯一bare隐藏Window；仅driver、fixture、Windows workflow三文件。schemas1–7/N1字节与旧case保持。十项真实hook安装及单独只读同实例1023 receipt先完成，不能借P1/inline_loaded/任意seed导航实现。
- 唯一刺激入口只接受真实request，无script/URL/path参数。在借用外读同一core.Source，要求实际初始值严格about:blank并重验live/身份；缺失/不同/未就绪失败，不导航初始化。原生状态预留一次后只调用一次core.ExecuteScript，固定源码检查top===self、location.href严格about:blank、有现成document根且无iframe；只创建一个iframe、先设src为字面量data:text/html,再挂入现有DOM，最后返回固定字符串code0-child-submitted-v1。无子脚本/资源/正文、网络API、存储、手势、模态、focus、权限、下载或popup；无循环/计时器/重试/任意插值。目标即使未取消也仅为空惰性data文档。
- ExecuteScript必须有真实非null完成handler。调用前预留N2专属有界完成义务及handler库存，保持weak/UI/Active与借用外COM/Drop；该义务不得进入旧create Pending的callbacks.pending早退计数，旧创建guard语义保持。真实child拒绝立即推进现有Failed/Stop/Close/Destroyed，不等脚本完成；最终正常Released/ref0必须同时具有真实完成disposition和handler真实释放/库存排空，不能提前减票、以销毁推断完成、丢票称取消或延长10秒。缺失/被保留完成在原期限内不能清理则失败/不确定。
- 完成回调在UI门后、任何字符串分配/通用wrapper转换前，将借用PCWSTR按最多64 UTF16单元有界检查/复制，拒绝null/无效UTF16/超长；不CoTaskMemFree、不留指针。真实HRESULT须成功，结果严格等于JSON字符串 `"code0-child-submitted-v1"`，只记录固定事实。完成只是刺激证据，不创建child/Cancel/Session计数或新的授权。初始提交要求live；合法同一已预留request/generation/installation的完成允许在child导致closing后、Released及原期限前到达，仅作结果/清理disposition，不使用禁止closing的S2 authorized作为回收门、不恢复导航或采集权限。陈旧/错代/重复/已Released/逾期完成失败，不能刷新首次关闭时刻。child事件、提交返回和完成之间无固定先后；错误线程只能用原线程安全边界，不触UI Weak/Rc/RefCell。原caps4/4096/16、45/10/75及全部旧门禁保持。
- 目标只来自已注册Hook::ChildNavigation的真实FrameNavigationStarting，URI准确data:text/html,、成功NavigationId getter（0有效不输出）、IsRedirected getter/false、实际core sender/request/generation/live及同安装opaque receipt；SetCancel一次成功和真实Cancel读回true，在元数据错误时仍尝试取消并保留不确定，getters后再验授权。拒绝型旧child/top委托保持；若先到about:blank或其他事件导致关闭、或者目标未到，就是前置阻断/未覆盖，不改白名单、重标目标或延迟失败。
- unchanged Context/poll真实Failed(PolicyRejected)经借用外events进入选定child分支，准确单次计数并与opaque child票据关联；脚本完成/DOM/Cancel不推算该计数。单独child状态不借top_selected绕过schema6原安装即关闭合同，N1第一关闭时间Cell方式保持且child自己的首次真实closing只能采一次。真实BrowserExit/Destroyed/ref0/Released/run_return/一次TempDir.close、无多窗口、actualexit0均必须成立；不声明per-frame ID/DOM ancestry或额外top导航证据。
- 严格新鲜schema8仅接受准确selected/completed=1、安装先于刺激、一次真实原生提交/完成及子事件/动作读回/Session失败和所有清理正向事实。额外/重复键、Bool充整数、旧identity、缺/多case、超限、保留pass下关键事实突变均须拒绝。先独立窄审该准备边界、再源码双审与隔离actual；缺目标/前置/超时不skip/PASS或称行为RED。不自动第二目标/传输回退，不扩大旧P1/网络/信任/CA/账号权限。

准备fix1：独立16199/e643b964的C0/I1已由Root全文接受；上述N2专属完成义务与closing-phase disposition条款修正原准备Existing Pending歧义，原18427准备和4032 brief及首审保留不改。当前仍无源码GO，先定向复审与N1实际接受。

N1当前进展：Windows40747139906/真实757c/1d38090的有限N1实际独审20165/0a0148da C0/I0/M0已Root全文接受；安装、真实top取消读回、真实Session拒绝及正常清理通过。此为N2准备依赖的窄接受，不结项Task4/5或覆盖Mac实际HOLD；N2仍须准备fix1复审后才能派源码。


### Mac publication fix3：实际失败后的集中窄修复（2026-10-29）

1d38090/Mac40747140380实际独审12694/adeb53be HOLD C0/I5（五失败定义/14issues）及作者只读分析18433/ffd21dc2四组均由Root全文核读。两v2目标事实正向、固定diagnostic acceptancefalse正确，完整suiteexit1不能通过；STOP具体失败谓词仍未知，不将继承mask假设冒称历史实测原因。以下是原技术实现选择的修正，不改变用户需求或旧验收合同。fix3唯一作者仅拥有builder和CLI tests两文件、必要新ignored报告/冻结；Mac workflow、App launcher/C2 test、四parked、native及旧报告/冻结全部保持。

- 停止许可迁至既有.status.ready，仍严格同实际guardian PID/nonce/单LF、完整有界owned regular单link0600/inode/EOF检查和原子发布；父统一从watchdog_ready_file读许可。旧.status.stopped仅保留最终helper_stopped_v1 ACK语义，App parser与653原absence断言不改。ready阶段复用依赖真实startup已完成父确认后才启动guardian、watchdog只缺失时创建且不重写既存ready、两消费者均在late removal前；保留原watchdog整行及所有ready移除时序，源码审查逐路验证。发布/身份/所有权/后续父query/foreground未知仍单调保留、不能将许可当清理完成。
- 实际matched请求的全部pre-fork退出补同一one-way停止许可/不确定保护。必须结构性证明尚无fork子、原所有权与身份准确、真实完整matching请求且无先前发布/cleanup失败；parent loss/status!=ready/泛stop_requested不授予许可。保留pre_exec_cancelled wire与真实130/原parent reap/group/watchdog/foreground gate，不旁路父strictgate、不信号未证PID或重试。许可发布失败不得假成功。两旧launch-gate最终helper_cancelled断言保持，实际输出补固定enum以区分未知状态，不输出任意文本。
- 发布器中断成功臂只观察真实生产builtin wait成功分支0的有界builtin-only事实，在真实commit前要求该事实；移除fixture新增第三次消费wait的假设，不增child/query/缓存PID重试或替代真实join。保留held child、USR2/deferred/ready、真正final handler、stage79负臂、取消请求/出版序列/130/guardian及no-latepublisher等原断言；不把actual98叫产品join失败或编造其wait rc。
- STOP三臂在独立测试actor内显式建立TERM可投递且其余继承mask位保留的已知前提，可用未发送的blocked sentinel证明exact saved-mask恢复。只改该fixture的mask准备与有界固定enum阶段/guard诊断，不改production正确的原mask保存/恢复、公共nativeobserver或任何全局线程mask。实际stopped/terminal/ECHILD wait、masked pending/selfTERM/真实handler、退休/uncertainty、准确仍owned child才KILL/reap、无未证numeric syscall、normal143/EOF/reap/teardown与3秒原限均须保留；失败原因仍须以新事实判定，不删handler或以外部救援当通过。
- 同步所有受影响精确fixture/seam：strict permission输入、ownedchild许可writer/ready、fork-permit needle、原/观测F_SETLK gate块，原语义/逆还原保持。新增或修改回归保持真实产品路径，不用模板计数代替行为。原初19/fix1 28/fix2 28及全部actual失败永久保留，另冻HEAD→fix3全差异和fix2→fix3增量、源码保护清单/DATA自检。修后独立SPEC+QUALITY定向复审，再按原完整隔离门禁实际验证；不改schema/期限/上限/断言/省略门禁，不执行宿主产品测试或旧不安全源。

Macfix3当前进展：作者13175/b0ae95f5、完整48250/058438d8及SPEC11602/5e4cb82a、QUALITY11417/4b804e68均Root全文核读，定向SOURCE C0/I0/M0接受；70 suite/C2/全部新实际UNRUN。先本地2源码/Root文档检查点及备份，再发已完成只读接入的N2新作者，源码获审后同轮实际验证。旧Mac实际HOLD不改写为通过。

N2当前进展：作者13749/e3b10376、全99487/cc626818及SPEC22908/2785341a、QUALITY18340/66648b11均Root全文核读，SOURCE C0/I0/M0接受。旧N1提交只有新增child互斥guard需作明确字节保留限定。当前一切新编译/实际N2及Macfix3的70/C2均UNRUN；精确源码/文档检查点、完整备份后同一隔离CI实际验证，未到目标或缺完成/清理仍失败/未覆盖，不扩大刺激或重试。


联合实际检查点：获审Macfix3与N2已随3118813推送，隔离Mac40761904949、Windows40761904948与dispatcher40761904917正在运行，真实merge5b660a6全树与head一致。以上历史UNRUN描述保留为发起前状态；当前仍无本轮实际验收结论，不据子任务成功扩张GO或直接发布。


#### S2-2 N3准备：一次受控新文档的alert拒绝（尚无源码GO）

Root全文读取原初始blank准备6459/6d1a、A/B比较11688/7535及候选合同12102/015aa2d2。选择A的单次硬编码无网络空HTML新文档，B的P1/S2组合留后续S4，禁止直接叠加或去掉旧互斥guard；选择严格S：真实alert事件后必须有实际成功HRESULT和准确固定token，失败HRESULT的event-only分支不采用。预审需先接受该有限合同，且N2实际独审接受后才派唯一新源码作者。

新独立selector refusal-dialog-new-document-alert/schema9、真实十hook1023/默认dialog false先于唯一NavigateToString；一次性about:blank grant只属于该同实例/请求/安装与准确NavigationId，额外真实NavigationCompleted成功及Source/readback/实际提交返回共同证明新doc。额外observer是真实独立库存/同emitter单次移除，不冒称第十一refusal hook；缺失/额外/歧义/redirect或getter失败都不给脚本授权。production、official谓词、P1/inline互斥、N1/N2与schemas1–8保持。仅driver/fixture/Windows workflow，硬编码空文档与单次固定alert/返回token，无参数化URL/HTML/JS、网络/存储/手势或任意导航。

实际目标须Kind=ALERT、准确固定message/Uri、同native sender/request/generation/install/live、真实未Accept/ResultText/deferral返回，不虚构Cancel读回。完成使用N2独立obligation/mailbox/handler库存和UI先行有界PCWSTR，不能进入阻止Stop/Close的旧creation Pending；实际refusal立即关闭，不等token/延迟revocation/加时，合法closing完成只作原操作disposition，不恢复权限。strict S缺成功token/目标/真实completion或清理即失败，不以event-only、expectedAbort、handlerDrop或丢票通过。最终仍需原45/10/75、实际NativeFailed通道、BrowserExit/Destroyed/ref0/Released/runreturn/TempClose/exit0及新鲜exact schema9，全旧门禁保持。alert不能证明confirm/prompt/beforeunload或所有OS UI不存在，未到目标不叫有效RED/GREEN；预审/双源码审查/实际验收分开。


Macfix3实际进展：40761904949完整四job成功，11751/3462e9fc独审C0/I0/M0经Root全文接受；两完整70与原C2/四失败组、严格双臂v2通过。主736实际pass/27skip（12另hosted覆盖/15仍未覆盖）及全部serial/后门禁保留准确范围。只接受本修复与配置门禁，旧内部根因不倒推、不代替Windows/N3及Task4/5或发布验收。


N2实际进展：40761904948/5b660a6完整Windows成功，23610/9b2605d7独审C0/I0/M0经Root全文接受，真实child/完成/库存/正常清理有限slice通过，非生产Code0WebSession。N3预审发现初始blank与新NavigateToString的NavID归属辨别不足，当前SOURCE HOLD，先最小补强准备再复审；保留现有候选准备，不自动选另一传输或放宽原生关联。


N3准备I1修正方向（尚无源码GO）：Root全文读取18271/21f58064独立HOLD，固定DOM标记不能证明旧NavID属于新load。选择新的唯一受控运输：同一TempDir的独立固定leaf code0-s2-dialog-empty.html，经create_new与handle元数据证明确实0byte regularfile并关闭，按N1既有本地盘/canonical/精确父目录/nohost/query/fragment校验派生内部file URI，唯一core.Navigate；不读取个人文件、不使用NavigateToString、不自动回退。新selector refusal-dialog-owned-empty-html/schema9替代从未实现的旧N3候选。grant只接受准确该内部file URI的单次真实Starting及匹配ID Completed/IsSuccess/Source完整匹配，初始blank不能匹配；固定alert脚本不接收路径/URL或拼接代码，可查file协议/固定leaf后缀，原生Source/event负责全路径关联，严格成功token与立即关闭/独立完成义务保持。该方案须新fix1准备brief与定向独审C0后才可SOURCE GO；旧全部候选/审查不改，不将file下载/前置阻断当目标覆盖。


N3fix1准备13591/c606ac63由Root全文读，唯一controlled-CI sole-writer freshTempDir前提明确，非抗恶意同权限替换证明。后续唯一新作者仅driver/fixture两源码及新报告冻结，Root负责Windows workflow固定追加run/strictchecker/artifact；新exact schema9 producer/keys/types须在完整三文件双SOURCE审查/CI GO前冻结，而非禁止作者设计实现。实施GO不等于源码接受或native执行许可，旧WF移除新块须逐字逆还原；全部旧SOURCE/actual/四parked保持。先定向准备复审及完整检查点，尚无源码GO。


第21轮独立dispatcher40761904917现完整SUCCESS，追加14584/7cb9ad50（保留194192/c0ec前缀、全文208776/df56d200）独审C0/I0/M0经Root全文接受；本轮25基线/四严格编译后的准确语义kill及全部清理通过，实际job23m52/步骤22m25，原期限不变。当前3118813三CI及三实际独审均已结束接受各自有限范围，仍不代表Task4/5或发布完成，N3fix1准备独审待完成、尚无产品源码GO。


N3fix1准备14036/d23bfcd9定向独审关闭原I1、C0/I0/M0经Root全文接受，固定owned0byte.html与准确内部file URI/strictS及sole-writer限定获准作为下一实施合同；新鲜作者已只读接入、零写入。先提交并完整备份当前三实际C0和准备检查点，再明确唯一作者实施GO；N3 SOURCE/actual仍未执行或接受，不能据准备C0称目标通过。


完整59文件检查点73309d84读回后，Root已明确向新鲜Sol6.1/high作者授予N3唯一实施GO；仅driver/fixture两源码及新报告冻结，Root负责后续WF/checker。基线本地3859fc1/远端311，N3仍在开发、未冻结/独审/编译/actual，不据实施GO扩大为源码接受或发布。


### S3后续准备：原生预绑定input的有限chooser取消oracle（尚无源码GO）

Root全文读取15708/a672与两阶段16415/cf79119e准备，选择待独立预审的请求特定protocol-cancel equivalent：S3自身新fresh owned0byteHTML/十hook/完整NativeURI-NavID完成证明、真实CDP配置ACK、native FrameTree/loader与verified uniqueContextId、先固定非激活create input，再从native DOM根查询唯一节点并describe准确backendNodeId，最后同world固定userGesture click一次。真实Page.fileChooserOpened必须frame/mode/backendNode吻合；真实trusted cancel后files0/valueEmpty成功回调invocation必须早于真实Stop/Close，不能延迟立即revocation/关闭或用Close造成的cancel通过。初始/前置/oldloader/假节点/缺实际target/后关闭回执均失败；所有旧2048-byte响应/8KiB artifact及45/10/75保持。

仅有限固定Page.enable/intercept、Runtime.enable、FrameTree两快照、createIsolatedWorld、DOM getDocument/querySelectorAll/describeNode与两个固定evaluate，参数准确allowUnsafeEvalBlockedByCSP:false、grantUniveralAccess:false；无任意API/代码/路径设置或global UI采样证明。此片不覆盖全局OSUI absence、nested/multiple/directory/capture/FileSystemAccess，生产其他chooser面仍未完成。准备C0及N3 SOURCE C0前不派S3实施；可安排后续同一CI独立运行N3/S3，但S3须自身实际前提全成立后才click，不拼接其他case证据。新source具体字段/表达式/Node可选字段/有限receiver与生命周期须在SOURCE双审/CI GO前冻结，未满足则HOLD，不扩大caps。当前N3唯一作者不因该准备扩范围。


N3候选已冻结：作者26260/07a22825及两源码115656/78aab096均Root全文读、25冻结成员与保护核对；Root仅在作者STOP后追加177行schema9严格WF，完整三源码129064/8a913d05已Root全文核读，规格及质量独审正在进行，未SOURCE接受/提交/编译/actual。旧WF可逐字逆还原，11root/61dialog数据契约及270负变体/8坏JSON仅数据检查，不冒称原生测试。S3后续准备19101/16e43b3c独审HOLD I1（receiver白名单/数量/epoch阶段未封闭）已Root全文读，先修准备、不扩当前N3任务；FSA13716/db61b18e只读报告表明上游CDP有pre-picker路径但Runtime155未证，权限DENY不冒称通用picker前拦截，其他chooser面仍未完成。


N3完整SOURCE双审已Root全文读：SPEC27042/5c1fe419 C0/I0/M0、QUALITY21257/7014bb1c C0/I1/M0，合并裁定HOLD；缺陷为原fixture started+45秒未绑定，较晚DialogState时钟及closing adoption可放行截止后的alert与完成。先保存两报告/原候选，再原作者定向fix1，仅N3绑定唯一原绝对deadline及getter/Drop后重验，原10/75及严格S/61字段保持，不提交或运行已知有缺口候选。


2026-10-29 N3 SOURCE fix1执行检查点：初始SOURCE C0/I1/M0 HOLD保持，持久14文件增量`req004-3859fc1-n3-SOURCE-HOLD-before-fix1-20261029`清单SHA256 `9b44e8a95ef1077edb74254090b9c906e24c13fe13711c548c85c54961908d33`已逐文件回读，承接原候选60/d04dcc。原作者已获两原生源码的fix1窄修派工，原started+45秒绝对期限、重入调用后的复核及有意义共享路径回归是本轮范围；Root工作流及四份parked不变。S3 receiver/epoch新预备brief22185/f79ae33f经Root全文读取后交原独立审查者定向复审，尚未接受、没有S3源码GO。当前本地3859/远端311，新实际测试仍未运行，不提交推送已知缺口候选。继续用户最新Sol6.1/high、保留改动且全测试与独审后直接发布授权，不重复征求模型/发布确认。


#### S3准备fix1：精确receiver/观察epoch与独立预算（2026-10-29，仅准备接受）

22185/f79ae33f准备与独立17485/7528623c已由Root全文核读，原准备I1 ADDRESSED、C0/I0/M0。固定10个同core原生receiver：Page.fileChooserOpened、Runtime.executionContextCreated/Destroyed、Runtime.executionContextsCleared、DOM.documentUpdated、Page.frameAttached/Detached/Navigated、Page.documentOpened、Page.navigatedWithinDocument。每次真实投递均须Args2+空default SessionId；全注册先于enable/config/唯一fresh文件导航。Bootstrap仅有界已知初始化且无节点权限；真实fresh-doc/config完成后call4前唯一arm，随后所有文档/拓扑/context失效为sticky，无ignore-first、重绑或第二getDocument。唯一createWorld candidate与ACK必须原生ID一致后使用uniqueContextId。所有授权边界仍使用原attempt绝对45秒；真实target立即撤权关闭，真实成功effect回调/有界捕获/身份复核均须早于第一次实际Stop/Close entry。

新S3独立预算为command response/named-context candidate/chooser总3槽、所有retained协议文字合计4096 UTF-8字节、单copy2048 UTF-8/scan2048 UTF-16、Bootstrap最多16通知、10receiver与最多11completion handler的独立真释放库存。旧P1的4是dispositions/script库存、4096是script-ID UTF-16单位、16是delivered resource数，不挪用或扩大。准确typed profile/固定脚本/producer/artifact仍须在组合SOURCE双审前冻结。

接受范围仅本次请求的真实回调观察序、匹配target与preStop取消effect。postStop已知销毁形状仅有界清理，不证明事件由Stop导致，不补缺正向事实、不清先前invalid；公共payload未给引擎全局因果时间戳，不能宣称全程无未投递变化/全局OS窗口不存在。受控fresh空文档、唯一固定输入、无其他writer与真实native单次导航gate是前提；FSA/nested/multiple/directory/capture/production连续性仍未完成。此准备接受没有S3源码或CI GO，仍待N3原截止fix1 SOURCE C0、Root持久检查点与明确唯一作者派工。actual全部未运行。


#### S2-2 N4条件准备：初始blank真实popup拒绝（2026-10-29，仅准备接受）

Root全文核读准备21307/d26c17f7、范围澄清3857/9201e00d及独立15686/5e895017，SPEC/QUALITY均C0/I0/M0。候选只在新鲜隔离fixture真实top about:blank、同core十hook及只读1023先决下，一次硬编码无手势window.open about:blank；无新导航、参数/网络/真实账号/deferral/SetNewWindow/额外owned窗口、重试或故障注入。目标必须真实NewWindowRequested，native URI/IsUserInitiated=false、SetHandled成功与真实Handled=true读回、同请求安装/live/原45秒调用与Drop后复核、实际Session PolicyRejected单次；固定严格成功HRESULT/token和独立完成义务/真handler drain、原10秒关闭/75秒watchdog及全部正常清理/实际exit0仍必需。

仅接受此实际请求在文档化成功API语义下的拒绝，不证明全局OS UI不存在或任意native失败下绝不默认popup。Handled失败/错线程/缺回调/读回false可能产生默认空白popup，必须失败/未覆盖并保留不确定；不为它造救援WebView、不从app-owned数量推断系统窗口。立即Close下严格token是否能真实到达未知，不能延迟关闭或改event-only/expectedAbort。源码需锁精确script/schema/producer/strict checker并双审；作者提议schema10未分配，Root将与S3无冲突后才派源码。当前N3fix1仍唯一源码作者，N4没有源码/CI GO、actual全部未运行。


S3固定表达式/profile准备追加接受（2026-10-29）：Root全文核读26380/cab2b90d（两连续完整字节段）及独立20482/f9a4c5aa，SPEC/QUALITY C0/I0/M0，仅准备。两个惰性表达式743/94910f66与1364/8c3536bf固定单input、原对象及真实trusted cancel时的files0/valueEmpty快照；完整owned URI仍由native独立证明，cancel/change计数不声称一直到Close无后续事件。11调用/10receiver精确closed typed profile及3槽/4096 retained文字预算维持，新增三项显式浅Node可选profile：Document仅空xmlVersion、HTML frameId只与native root一致性、非authority UA shadow summary允许nodeId0；root/query/INPUT仍正ID，INPUT.frameId拒绝、不增加pierce/重查/方法。没有增加CDP timeout窗口，原45/10/75与调用/Drop后复核保持。Runtime155具体shape/激活/cancel/preStop完成及实际内存/释放全部未验，未知/不合即失败/未覆盖。没有S3源码/CI GO，仍待N3原截止fix1 SOURCE C0、检查点与明确唯一作者任务。

2026-10-29 final-gate queue: readonly legacyclock auditor reports same originalattempt45 gap in immutable3859 N1/N2 (fixturestarted1960;adopt2067/2082 beforetimeout2785;native live2201/authorized2278 no clock;N2completion2035 close10only). Report stillfreezing/notRootFULLyet; queue exactwithinTask4 for fullread and narrowfix BEFORErelease, notsilentlydefer. Existing early actualC0 retained; no actuallate execution claimed. CurrentN3fix1 staysONLYN3, no scope expansion toP1/production/othercases.


2026-10-29 N3fix1源码候选已完整冻结并进入双复审：报告21346/a30757d1、34成员manifest6183/c47b0ab0，driver356390/84afe839、fixture154069/a753bcc7。Root已全文核读报告及初始→fix1完整64753/77fd8510差异（连续4段），重新生成累计139693/f61a740b逐字一致，全部34+原25+Root11及WF/四parked核对通过；Root新增7成员组合包完整153101/6bb3657b（覆盖初始已读+完整修正，非新实际结果）。六共享路径回归仅定义、编译/实际未运行。持久65文件候选检查点manifest1c52fd6ffdff372d27681ef7433f1e9fdce1cef4cdbca3f8740b186f97381abe承接16/46c0及原历史。SPEC与QUALITY分别定向复审，SOURCE仍待接受，不提交推送已知缺口。

Root随后全文核读legacy N1/N2审计15808/820547e3，接受静态I1：原45秒未进入native授权/目标证据，先adopt closing遮蔽timeout；N2现有完成只检close10。已排在Task4发布前窄修，当前N3fix1范围保持。明确新正向口径：N1/N2刺激及目标、N2成功完成的真实调用与消费均受原45秒限制；close10只作真实清理，不扩成功窗口，晚/错误匹配完成可在close10真实disposal但不能成功。保留此前及时实际C0，未执行迟到原生复现，不扩P1/production或所有旧case。新窄修仍须N3当前SOURCE接受后单独检查点/派工及源码双审。

N5下载准备26063/efbbd0c6+Root3158/3bf83a82+独立20475/1a164f57已Root全文核读C0/I0/M0，仅有限正向准备：严格初始blank固定空data下载anchor，真实args.DownloadOperation.Uri、Cancel与Handled各自真实独立setter成功和读回、实际Session/严格token/正常清理。Handled不等于取消；默认progress、失败时保存/UI及目标可达/即刻Close竞态都未证，不声称全局文件系统/窗口缺席，无路径/手势/网络/策略/替代传输。没有源码/CI GO，schema提案未分配。S3固定脚本/profile准备C0记录已保存；全部新实际未运行，整体Task4/5未完成。


2026-10-29 N3原截止fix1组合SOURCE已接受：独立SPEC21630/5f805a60与QUALITY19053/375a436d均原I1 ADDRESSED、C0/I0/M0，Root两份全文核读并接受准确driver356390/84afe839、fixture154069/a753bcc7及不变WF98473/bbed。完整64753差异已读并累计逐字重建、全部冻结及其他改动保护通过。六回归/编译/actual全部未运行；这仅允许保存源码检查点，不是原生通过、整体完成、合并或发布。先保存新审查与Root结论后作选择性本地提交；另排N1/N2原截止窄修审查完成后再批次隔离CI，当前没有新CI运行。未来legacy brief8535/30902ee0已形成，仅条件草案、无源码GO，须本地基线检查点完成后明确派工。

2026-10-29 N3 SOURCE检查点已选择性本地提交`f2295b5765717b656bcc17eb2ed7fbc471d55494`（仅driver/fixture/Windows WF三文件，逐Git blob绑定获审hash），基线3859。Root3记录将另提交，四parked仍未staged；远端311未变/无新CI/6回归及编译原生实际未运行。SOURCE双审21630/5f805a60+19053/375a436d C0已Root全文接受，完整持久65候选及16源码接受增量`8e661a8bb772ec27b898eee2414df5f16ec076dd49920bc1e1f46f94aa8a4be6`已回读。之后仅在全Git/最新记录检查点完成后给N1/N2原45秒窄修明确GO；整体未完成未发布。

2026-10-29 legacy N1/N2原截止fix1已在持久检查点后明确派给原Sol6.1/high作者：新本地HEAD`ef311976e09f5b7a946912bb2239f95573096463`（N3 source f2295b5 + Root docs），28文件全Git19证据refs/HEAD归档/9规范blob pins检查点manifest`a737a4b3aa8d5e6a65616cd9b6130571b3e9d1710447e44fa43cac3fafc0a515`已回读。严格限同两native源码+新报告冻结，8535/30902ee0 brief；N3六回归/原语义、WF/旧schema/其他case/四parked及Root新记录保留。N1正向见证若移至args/COM Drop后，真实首次native closing时间不能随observer变晚；允许窄内部时间观察维持原close10。当前只有该作者可写产品源码，后续仍双SOURCE复审及隔离actual；远端311未变、无新CI/未合并发布。

2026-10-29 PermissionRequested可行性15270/0b692935及独立17785/f2c86a86已Root全文核读，事实/候选分析C0/I0/M0，实施仍缺内部前置：受审secure官方origin synthetic P1+S2 composite和一个有界denied异步效果通道/身份/预算。仅候选一次Notification.requestPermission、不创建/显示通知或读设备/数据；file/NavigateToString不能据此证明可达，初始blank也无保证。native Uri按请求origin而非完整doc路径，Kind/IsUserInitiated/DEFAULT→真实DENY→真实读回及调用/Drop后身份原45秒、preStop真实denied效果各自必要。原互斥guard、原10/75及正常清理保持，没有源码/CI GO、actual全部未运行；这是Task4内部依赖而非用户/环境阻塞，不改变进行中状态或增加产品通知功能。


2026-10-29 legacy N1/N2 original45 fix1已冻结但SOURCE HOLD：报告20211/78d3d341、32成员manifest5875/28ffed3d，D384779/fbc04f5a、F157090/1bd0c1ca，Root全文读取完整95612/5c6aa1d2差异并逐字重建。独立QUALITY19613/968d5f8b C0/I0/M0与SPEC18883/d232b9e0 C0/I2/M0均Root全文读取，综合按C0/I2/M0 HOLD：新8参数helper违反原strict Clippy默认7参数门槛（未实际运行lint）；N2尚未提交ChildState时真实closing缺首时间、fixture fallback可后移close10（setup本已失败，不是已证迟到GREEN）。原45成功权限/真实nativeOk与私有timely区分已实质修正，原及时actual C0/N3 SOURCE C0保留。先保存原候选、两报告与HOLD，再交原作者只修两点；5771/ae9cac30条件fix2 brief已形成，尚未源码GO。新8+原6回归/编译/native均未运行，不提交推送已知问题候选、无新CI/合并发布。

2026-10-29 legacy SOURCE修复round1/5（artifact fix2）已在52文件持久HOLD检查点`b7f6cfe40a31728879bb327ab35d368ddcf5f2dbf46aa8d61800da1c66c4a9bb`完成后明确派原Sol/high作者。限8参数私有输入分组+ChildNone首native closing生产者/实际adoption与共享回归；不提前建ticket、不改JSON/WF/45-10-75、不抹真实native返回，原32候选/双审/RootHOLD及4parked全部保留。5771/ae9cac30 brief已锁，HEAD ef31197/远端311不变；所有14+新增回归/编译/native未运行，无新CI/合并发布，不是等待用户决定。

2026-10-29 Root追加同轮静态I3：不可变legacy v1的s2仍import Duration但该模块已无使用（旧close10实现移走后遗留），strict-Dwarnings需清理；未运行compiler/lint。新增1633/70beaf35窄addendum与Root记录，原5771brief/双审/52候选保持。作者已确认当前s2只此一处并在同fix2中仅删未用导入，test/全限定Duration不变，无镜像测试；范围仍两源码/同round1，原SOURCE综合HOLD为C0/I3/M0，三项全部交双peer定向复审。已新增2共享firstproducer回归且8+6保留，16项及编译/native均UNRUN。保护快照一项元数据新增断言遇Root新addendum停止后按授权更新重建，是DATA采证调整非产品/测试结果。

2026-10-29 legacy fix2（同round1）候选已冻结并送双peer定向复审：report14235/7ce70070、37成员manifest6965/a1c3ca9b，D391027/ffc2dacd、F157090/1bd0c1ca整份v1不变；Root全文读17904/16e58e7a完成delta及report，37+report/WF4parked与旧25+34+11+32、HEAD-v1-after/perfile+concat重建均通过。累计102701/7a7508dd绑定HEADef31197。I1私有input5参、I2ChildNone独立首close生产者/同步已有mailbox、I3只删unusedDuration；新增2条shared producer/control，legacy10+N3six16项全UNRUN。当前只候选/未SOURCE接受、无新CI，不由作者自述关掉HOLD；原v1和双审52/b7f6链保持。

2026-10-29 legacy original45 fix2双SOURCE复审均C0/I0/M0：SPEC15907/e601a297、QUALITY13588/e416cf49已Root完整核读，原I1/I2及Root I3均ADDRESSED；37冻结成员和当前两源码/工作流/四parked逐字核对通过。D391027/ffc2dacd、F157090/1bd0c1ca，私有完成输入分组、ChildNone首native closing生产者及未用导入修正获源码接受；legacy10+N3六条共16回归、编译/Clippy/native尚未运行，下一步精确本地提交、完整备份后单批隔离CI，不提前合并发布。

 获审两源码已精确本地提交98a1fb48b31a739fd15c2faa8c71c757763abdf9，373文档、公开扫描与差异检查通过；实际16回归/编译/native未运行。

2026-10-29 本批N3+legacy已推候选HEAD8a19d853725cf395044b5dac2ff9c07bc05182da：Windows40792192995、Mac40792193050、独立dispatcher40792193064均已启动；20文件全Git检查点manifest823a1e143435769e2adf09dd00e2c36682b8cde3a0e15ff9907ae7b80eb1da5e完整读回，19本地证据refs未推。四parked保持、无合并发布。当前三CI结束前不推下一头/不取消。下一单片选择已预审N4固定空白popup，分配schema10、S3预留11、N5预留12，唯一作者仅两源码；Root负责WF及正式记录，新SOURCE双审前无N4执行许可。

2026-10-29 8a19批次实际Mac40792193050与Windows40792192995均completedSUCCESS，实际merge99dbeacbd764b7fafb6335b03752b1f9f3c6b42a/全tree同head8a19并由raw checkout绑定。Windows raw640999/68b58b1c、9组22JSON下载0；Root严格类型/字段比对N1/N2/N3全正向，16新增回归逐条唯一实际ok，15真实normal close+15 TempClose成功，N3原关闭预算至run_return759ms。Mac raw1499547/dcadd5c8、受控CLI v2产物3份已保存，独立实际复核进行中；Windows独立验收待报告，dispatcher40792193064仍运行。这里仅新鲜CI和Root数据核对，不冒充双平台独立验收或Task4/5完成；当前N4唯一作者候选静态工作，未SOURCE接受/执行。

2026-10-29 Mac40792193050独立实际审查16699/b3455b45已Root全文接受C0/I0/M0，仅本轮配置门禁：4job32step全成功、strict v2两臂与旧4失败组通过。main763含27skip、hosted补12仍15未观察；raw736start仅665逐条pass，按完整framework汇总/退出验收且保留71日志粒度缺口，不虚造逐条结果。Windows整轮成功正独立实际复核，dispatcher仍运行，整Task4/5未完。新challenge可行性22857/601999e9已Root全文读，仅研究：当前gate无三类路线，未来有限loopback401/TLS需新合同/审查，clientcert仍缺正常信任与回调可达前提；不改信任/CA、不冒充实际覆盖，也不因此停N4。

2026-10-29 Windows40792192995独立实际报告25753/00ee27a9已Root按两连续字节段完整核读，有限C0/I0/M0接受：N3真实alert/严格成功token/实际driver Session失败/正常清理，61字段、16共享边界回归各actual ok、396唯一Rust/前端302/strict Clippy/47step、15真实运行45正常诊断及TempClose均通过。原生时序为及时正例，未伪称真实COM跨45秒复现；N3关闭至run_return759ms，旧历史首因未知保留。Mac本轮actual C0已接受；dispatcher40792193064尚运行，后续N4候选和S3-S5/Task5未完成，未合并发布。

2026-10-29 Task5既存M1详情重开8秒期限回归另作隔离test-only RED采证：原Shell28819/13bd0429未改，已独审四parked中TS14757/5a1db4f7仅复制到专用支线，不挪动当前四parked/N4源码或推新PR67头。使用单独frontend-only CI先旧测试对照再真实新断言，未实际RED前不修Shell；主Windows/Mac实际C0保持、dispatcher当前轮继续。原生N4作者/当前优先级不改变。

2026-10-29 第22轮dispatcher40792193064完整独立实际C0/I0/M0已Root全文读16683/94ad6519追加并核225459/463927df总报告与原208776/df56前缀：25基线/四严格编译后精确语义kill/全部清理/最终verifier、21保留hash及22成员通过，job1729s/mutation1654s，原期限不改，当前private过滤67/68真实推导。8a19三组CI及各自有限实际独审均接受，20本地证据refs保持不推；整Task4/5尚未完成。N4两源码候选已STOP/封52成员8a6e938d和报告26531/bfc2bc7e，D435629/a29aa7f8、F169501/dcf35a8c，新1严格parser测试+原16逐字保持，新编译/17测试/native全部未运行；下一Root接WF并三文件双审，非SOURCE已接受或发布。

2026-10-29 Task5M1专用支线f98a7dc/CI40797644046已确认有效行为RED：实际Shell28819/13bd、原测试23全过；新增B/null两case均在逻辑t8的122行no-close断言观察到close_provider_detail提前一次，25项中旧23过/新增2失败，真实exit0/1、准备/编译/超时无失败。raw77407/6c6bfc5d与8文件完整artifact（实际Shell/old/new测试、日志/exit）已保存并Root核源/身份/摘要；仅失败后的Node缓存post-hook skipped，非测试跳过。其失败是独立回归的预期真实行为失败，不抵销已接受8a19三组CI；先持久保存RED，再唯一作者只修Shell展示计时，原8秒/测试/独立Code0数值/动作语义保持，后续GREEN与独审仍待验。

2026-10-29 N4完整三源码双审已Root全文接受SOURCE C0/I0/M0：SPEC20293/c1eed1cf、QUALITY25894/c3639a87均独立完整读122191/50b956差异及真实producer/时钟/借用/lifetime/WF；Root全文读报告和新WF、机械重建native109282，不冒称Root本人逐行读全native补丁。当前D435629/a29aa7f8、F169501/dcf3、WF110150/78db，schema10精确11root/44popup/原WF可整份逆还原。新17测试/编译/strictClippy/nativepopup/token/清理仍未运行，只选择性本地检查点，不称功能GREEN/整项完成或发布。

2026-10-29 N4获审三源码已本地精确提交df0622d2c50444e7608dfb44e0752307439fe96e（新编译/17测试/native仍UNRUN，远端main候选仍8a19）。Task5M1 Shell最小3增1删候选28971/72f0及Root定向WF5309/120f完整6222/0ba90双角色SOURCE独审16656/1c12e982已Root全文接受C0/I0/M0，原8秒/测试/独立会话数值与动作未改；有效RED40797644046保留，后续先支线target25+全前端/类型构建GREEN，未提前整合主分支或发布。
