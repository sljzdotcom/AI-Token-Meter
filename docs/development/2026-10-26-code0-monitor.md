# Code0 站内额度监控开发记录

关联 REQ-20261026-004；[规格](../design/specifications/2026-10-26-code0-monitor-design.md)、[计划](../design/implementation-plans/2026-10-26-code0-monitor.md)。本项接续已发布的0.10.11，当前处于开发中，没有 Code0 发布或真实官网登录验收结论。

## 来源与实现边界

核对官方公开文档、首页引用的脚本与匿名公开status后，选择明确网页登录、被动观察官网请求的方案。没有调用账户查询、Key生成/管理、登录/刷新接口或付费模型。两个指标仅为余额及累计已用站内额度，除数来自本次文档；不猜美元/人民币或余额比例。

Code0默认隐藏，保留旧四服务设置和小组件三项默认。仅用户明确连接或恢复后才联网；首次完整同步、保存和关闭确认成功后，本次运行每五分钟尝试同步。失败暂停，重启不从缓存推断联网授权。公开网页的实际账户、Cookie和2FA兼容性仍未验收。

## 失败先行检查点

[草稿 PR #67](https://github.com/sljzdotcom/AI-Token-Meter/pull/67) 从文档合并main `a97c2a5` 建立。规格提交 `9618828`；拒绝型数值接口与新回归提交 `22f24fd`。这时两个平台的捕获接口一律拒绝，未添加成功转换或网页会话。

- 首轮 [macOS 40301782804](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/40301782804) 在测试宏展开阶段失败：新测试嵌套使用 `#require`。语法解析未发现该问题；它不是有效行为RED。`99d574a` 仅将该语句拆成先读取provider ID、再解析provider，保留两个必要断言。
- 首轮 [Windows 40301782799](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/40301782799) 完成前端、格式及Rust编译/lint；Code0捕获5项测试中1项非法输入通过，4项正向行为失败：合法数值仍被拒绝、新provider/unit未识别。后续兼容套件和安装器未执行。
- 修正后的 [macOS 40303601344](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/40303601344) 编译成功，主组670项、106套件出现13个issue，均来自新增Code0捕获与可选第五项测试。合法数值拒绝、类型缺失、默认隐藏/迁移及第五行高度缺失成为实际RED；其余原主组通过。后四组、工具和hosted步骤未执行。
- 同提交 [Windows 40303601327](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/40303601327) 再次观察相同4项Code0正向失败、1项负例通过；仍不把未运行的后续套件计为通过。

两端实际RED已确认后开始数值、目录和偏好迁移实现。旧六份共享fixture保留，新Code0样本从辅助目录提升为正式第七份时保持单一来源。不得将此段RED记录改写成已实现/已GREEN。

## 共享网页桥与原生预研

`4739a58` 新增共享网页桥的拒绝型入口、15项合成Node契约及独立CI任务。测试仅使用受控fetch/XHR和合成响应，不加载官网脚本或真实账户。覆盖完整status+self、乱序请求、重叠auth refresh、精确来源、原调用语义、大小上限、异常和子框架/销毁失效。随后 `d49cc94` 补充原始数值词法舍入、合法零值/小数除数、UTF-8字节边界三项，合计18项。上述检查点只完成语法检查，尚未观察其实际行为RED，也没有原生资源包装或桥实现。

原生接口预研确定Windows须先创建空白隐藏实例，确认COM拒绝策略安装成功后才导航；macOS采用专用网站存储、公开委托和实际对象释放确认。隐藏实例无显示/聚焦/用户手势，不能将可见登录的任意系统权限行为混作隐藏实例承诺。静态接口与源码依据不能代替实际原生夹具。

## 旧终端测试失败的独立记录

此前仅文档main合并触发的三个终端测试失败保留在[0.10.11发布记录](2026-10-26-v0.10.11-release.md)。它们不是Code0预期RED，也未被当作已证产品缺陷或简单环境波动。

`487a40d` 仅为合成测试补充有界失败证据：查询前后存在性结果、固定状态/标记、回执存在性及退出状态；缺回执时保留夹具目录。原状态分类、断言、超时和生产代码不变。另一个执行者独立静态审查无严重/重要发现；实际编译运行仍待下一检查点，不能声称001观察竞态已修复。

## 待完成门禁

数值/兼容、共享桥26项与原生消息协议已通过；macOS纯生命周期及来源探针也已通过。具体原子存储、OS独占访问、实际网页驱动与策略夹具、两端界面和Windows会话仍在开发，之后完成整分支独立审查及签名发布。产品测试只在已批准的隔离CI执行；没有在开发宿主运行产品测试、真实CLI或用户账户操作。

## XHR可观测边界澄清

独立接口审查确认XHR只公开最终地址，无法区分直接请求与中间跳转后回到同一地址。规格据此明确：原请求与最终响应必须为同一允许端点，fetch另拒绝已跳转响应；XHR仅承诺拒绝可观察到的不匹配，不宣称完整网络链无跳转。这是被动采集的能力边界，不改变官网原请求，也没有新增主动账户查询。共享桥行为RED已在 `60691d9` 的Node任务观察：18项中6通过、12预期失败；有效完整配对输出0而期望1，完整桥仍未实现。

## 首个实现候选检查

`60691d9` 提交数值解析、可选第五项和偏好兼容候选。静态合同7份、文档373份及公开扫描通过；完整Task1源码独审无关键/重要发现，交接清单范围需补说明。实际隔离检查尚未GREEN：macOS [40308014742](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/40308014742) 被之前诊断辅助函数的同作用域变量重名阻断，行为测试未运行；Windows [40308014721](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/40308014721) 前端138项中137通过，旧设置测试仍按四服务目录计算总控件数，后续Rust及安装器未运行。两处进入窄修复，既有行为断言和诊断边界保留。

## Windows原生入口选择

只读追踪锁定Tauri2.11.5源码后确认：无remote capability会拒绝远程自定义命令，但内部channel fetch有ACL例外，因此不能用该配置宣称远程页完全没有内部IPC。Task4改为owned bare Window加直接WebView2 COM controller，仅安装过滤额度桥，不构造Tauri/Wry WebView。公开gated WindowBuilder、HWND及环境/controller完成回调已有源码依据，实际编译、双平台策略与销毁夹具仍待实现；不以接口研究冒充运行结果。

第二轮 `3cf9bb8` 原生结果：mac主组675项仅新增单位缺动态模板样例导致2个issue；Windows前端138项通过，Rust首组43通过/1旧托盘恢复测试失败。`a838d60` 仅补三语言样例及明确单服务恢复/全五项选择测试，定向独审通过，未放宽旧断言；新隔离检查仍待结果。

## Task1完整验证通过

`a838d60` 的[macOS原生任务](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/40311436159/job/121533318008)完成675+3+18+50+16=762项常规测试、8项窗口测试及合同/文档/公开扫描；此前诊断代码也实际编译运行。独立网页桥任务仍为拒绝型预期RED，所以该workflow总状态不能误写为全绿。[Windows 40311436139](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/40311436139)完成前端138、Rust287、密度生命周期/浏览器检查、debug NSIS与GUI子系统检查。两次Cargo PDB输出名冲突告警与已发布基线相同，保留为既有非阻断噪声，不冒称无告警。完整Task1及两轮修复均独立审查，无未关闭关键/重要发现；Task1完成，004整体继续进行。

`c97ddf7` 新增共享桥行为候选、24项合成测试、两端原生拒绝型协议及字节一致性门禁。阶段独审无阻断发现，记录一项额外数值边界测试建议；当前实现仍不能接收原生消息。下一隔离检查验证JS候选并取得原生trace实际RED，之后实现native验证；官网窗口、专用存储、生命周期/保存事务和最终发布门禁尚未完成。

## Task2共享桥通过与原生行为RED

`c97ddf7` 的[独立网页桥任务](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/40313485799/job/121540175377)实际24/24通过，资源逐字节一致性通过。原生Swift主组684项只在3个新增协议正向测试出现14个issue，均因拒绝型入口未接收合法控制或capture；[Windows 40313485613](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/40313485613)成功编译和lint，协议10项中7项负例通过、3项正向失败，具体返回均为ValidationRequired。后续套件与安装器未执行，不计入通过。

两端实际行为RED经逐条核对后，开始原生来源/epoch/attempt/nonce/document、序号、refresh集合和一次消费校验；Task2尚未完成。共享桥24项通过不代表原生窗口、登录或真实账户已验收。

## Task2完整验证通过

`3695590` 的[macOS 40316850370](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/40316850370)全部通过：687+3+18+50+16=774项常规测试、8项窗口测试、26项网页桥测试，以及合同/文档/公开扫描。[Windows 40316850218](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/40316850218)前端138、Rust300（含原生协议13项）、密度浏览器、debug NSIS与GUI子系统检查全部通过。原生合法完整trace、重叠refresh后新self及终止控制从预期失败转为通过；原负例保留，新增数值上限、重复解码键、撤销后迟到事件测试也通过。

Task2独立规格与质量审查无关键/重要发现，第一阶段数值覆盖建议已补并实际运行通过。macOS现有5处AX弃用告警与先前基线一致，Windows仍为既有PDB输出名冲突及上游Action运行时提示，未新增本任务告警。Task2完成；实际官网窗口、独立profile/lease、保存事务、登录后同步、定时调度及Services/detail仍由Task3/4继续，未宣称完整Code0或真实账户验收完成。

## Task3生命周期行为RED

`665cffa` 引入可注入时钟、窗口驱动和独立store协议的拒绝型会话，以及16项生命周期测试。[macOS 40319952295](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/40319952295)实际编译成功，主组703项/109套件仅新增会话测试15项失败、24个issue：Connect固定拒绝、没有owned实例、缓存未加载、profile忙原因未提供；未连接零创建的负例通过。其他主组没有失败，后续分组/hosted未运行。

这确认了可执行的行为缺口，现继续纯会话状态机实现，随后再接具体磁盘存储、WebKit驱动、实际窗口夹具和UI。当前三个新增文件没有创建浏览器、使用用户账号或启用自动网络。

## Task3账户缓存回归与原生来源观察

纯会话候选增加严格时限、保存后发布、释放确认、撤销与重入保护。独立审查发现：启动读取缓存失败或缓存非法时，内存为空并不能证明旧账户持久值不存在；新的明确Connect仍必须在取得profile lease后失效旧值。`6e2262c` 的[macOS 40324956433](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/40324956433)实际编译成功，主组717项只有该新增两方法/三分支出现13个issue，原28个会话方法通过。`64ac910` 增加独立持久失效标记，删除失败保持cleanupPending并禁止新登录，30个方法及原断言均保留；修复的完整隔离结果仍待确认。

同一失败检查点的独立WebKit探针任务实际通过1项测试：合成内联页面的原生消息来源为预期official-root，主框架/对象归属、secure、前后隐藏且未聚焦、最终WebView和Window双weak释放均观察成立。探针只使用受控内容，不访问官网或账户；该结果证明当前CI运行时的fixture来源接缝可用，不代表生产driver、各权限拒绝分支、实际TLS或真实账号验收。`64ac910` 同时为四个观察回调补撤销及对象身份守卫，定向审查无新增问题，修后探针需随必要CI验证。

`64ac910` 的[macOS 40327278963](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/40327278963)完整通过：717+3+18+50+16=804项常规测试、8项窗口回归、26项网页桥与1项原生来源探针。新增账户缓存回归全部转为通过，原30个会话方法断言未改；I1和探针迟到事件守卫定向复审均无关键/重要发现。纯生命周期这一层完成，具体原子store、OS lease、实际driver与界面仍待开发。

本次原生编译另外暴露探针三个optional delegate签名的nearly-matches告警：alert、confirm与deprecatedTLS回调的completion类型和当前SDK不一致。这不否定已观察到的来源、隐藏和对象释放，也不能被解释为这三个策略已验证；下一必要原生检查点修正签名并补选择器/实际事件证据，不靠压制告警或手调delegate冒称真实覆盖。

同提交的[Windows 40327279102](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/40327279102)也已完整通过：前端138项、Rust300项、密度浏览器检查、debug NSIS和GUI子系统检查；本次Windows产品源码未改，验证未出现新失败。

## Task3实际存储与窗口契约检查点

`2b0eb9e` 加入拒绝型原子store、OS profile lease和真实网页驱动入口；新增11个Core测试方法、17个原生参数case，独立任务要求全部case记录齐全并验证隐藏和释放。隔离执行边界独审无阻断项，但不代表具体实现完成。

[macOS 40332596811](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/40332596811)在编译阶段失败：store测试两处throwing load放进require宏后缺少内部try，另一个测试构造缺少必填limit参数。普通和原生两任务均未执行行为测试，此次不是有效RED。原probe的alert回调仍有optional签名不匹配告警，当前编译器给出的completion隔离要求与上一诊断不同；保留实际selector断言，继续校正接口，不靠移动extension或关闭告警门禁处理。

`ef0f32e` 的[macOS 40333978112](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/40333978112)已编译并实际运行：主组728项/111套件只有新增store与lease出现29个issue（24+5），其余主组通过；原生驱动17个case记录齐全，cold负例通过，16个正向case均因拒绝创建/无策略安装确认产生32个issue。这些成为底层实现的实际RED，后续常规分组与窗口回归未运行。

原生来源探针另有4个issue，原因是alert selector没有注册；confirm及deprecatedTLS selector为true。alert completion注解的诊断再次反转，不能继续盲目往返或把它归入预期driver失败。[Apple公开接口](https://developer.apple.com/documentation/webkit/wkuidelegate/webview(_:runjavascriptalertpanelwithmessage:initiatedbyframe:completionhandler:))同时提供async形式，下一实现候选采用该明确接口并保留真实selector与编译告警门禁。当前尚未证明修复成功。底层GREEN实现已获授权，真实存储/窗口和界面验收仍未完成。

同一 `ef0f32e` 的[Windows 40333978105](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/40333978105)完整通过前端138、Rust300、密度浏览器及debug NSIS/GUI检查。前一检查点Windows任务被更新提交替代而取消，未计为通过；本轮仍不代表Windows Code0会话已经实现。

`49041dd` 为实际store/lease/WK驱动候选，尚未运行GREEN。完整源码独审发现5项需修正的问题：持有期间锁或目录替代后权威失效、rename后目录同步失败的候选恢复、受控401服务精确路径、创建资源失败的零对象释放，以及消息序列化前的键界限。另记录测试连接随服务停止统一清理的建议。先在同一候选增加针对性行为回归，再用隔离CI确认缺口，随后集中修正；不以源码审查或语法解析替代实际通过。
