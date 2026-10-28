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
