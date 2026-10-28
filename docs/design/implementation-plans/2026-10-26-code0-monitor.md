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
