# Code0 站内额度监控设计

关联需求：REQ-20261026-004。2026-10-26；003 已发布后接续本项。本文为开发规格，尚不表示实现、测试或真实官网登录验收完成。

## 产品范围

Code0 是可选第五项服务，显示官网当前连接的余额、累计已用站内额度、最后成功同步时间及过期/暂停状态。用户在 Services 点击“连接 Code0”打开独立的官方登录窗口；连接说明告知成功后每五分钟尝试后台同步。用户登录后点击应用中的“登录完成，开始同步”，应用先关闭登录窗口，再用新文档进行第一次受控同步。只有完整数据保存成功并且窗口完成关闭，才进入本次运行的自动同步状态。

默认安装、升级、打开设置/详情、显示 Code0 圆环都不联网。原四服务的显示选择、顺序、隐藏、密度与贴边设置保留；新增 Code0 默认隐藏。恢复默认也不启用 Code0。小组件保持原有三项默认布局，本次不增加 Code0 小组件。

明确操作包括连接、登录完成并同步、立即同步/恢复、断开。手动与定时动作进入同一个串行会话管理器；任何失败都暂停，不自动弹出登录、不自动重试。程序重新启动后仅加载旧数值并标为缓存/需恢复；**首版不跨进程恢复自动同步授权**。用户点击恢复才进行新验证，无需重新输入仍有效的官网登录信息。

## 官方来源与显示含义

- [官方文档](https://console.code0.ai/docs/home)及[用量日志说明](https://console.code0.ai/docs/8533385m0)以登录官网查看用量为当前明确路径。
- [Key 管理](https://console.code0.ai/docs/8533061m0)使用 Cookie/Session 管理凭据，不能推导出普通模型 API Key 可读账户余额。Key 配额不是账户余额。不尝试任何 Key 管理/生成接口。
- [计费说明](https://console.code0.ai/docs/8507360m0)将站内记账额度与美元区分。当前公开前端和公开 status 的货币标记也不一致；本产品统一使用“站内额度”，不用 USD/CNY/汇率或充值额推算余额。
- 当前公开页面代码在新文档根组件发起 GET `/api/status`，正常官网认证后发起 GET `/api/user/self`。页面自身可先执行 POST `/api/user/auth/refresh`；这是官网行为，不是应用新增调用。公开代码提供可行性依据，不是稳定官方接口承诺。

本次仅观察 self 的 `quota`、`used_quota` 与当前文档 status 的 `quota_per_unit`。余额为 `quota / quota_per_unit`，累计已用为 `used_quota / quota_per_unit`。除数样本 500000 不是默认值。两项数值来自同一完整 self 响应；不混合不同响应，不从 localStorage 补字段。未取得合法除数即不发布。

不增加历史图、请求数、token 数、充值管理、API Key 配置、付费模型探测或第三方 OAuth 支持。累计已用并非周期预算，没有 limit/reset/使用比例；圆环以无比例数值状态展示，不能用余额加已用伪造额度上限或触发预算告警。

## 双平台数据契约

新增 provider `code0`、unit `siteCredits`、kind `cumulativeUsage`，展示名 `Code0`。主指标 label `Balance`、kind `balance`；次指标 label `Cumulative usage`、kind `cumulativeUsage`；limit/reset 均为空。两项原始值必须是 0 至 9007199254740991 的安全整数，除数是正有限且不大于同上界的数；除法结果必须有限。拒绝负数、字符串、Bool、缺项、非有限或溢出，不静默截断或补零。

最后同步时间由 native 在接受完整数据时生成，原子持久化成功后才发布；不信任网页时间。失败不更新旧成功时间，已观察到明确登录/退出/连接切换则清除旧账号数值。只缓存两个归一化指标和版本/成功时间，不保存原始响应、账户 ID、身份哈希、cookie、token、headers 或请求 body。

Swift 与 Windows 的原四项 HTTP/CLI collector 保持原范围。Code0 快照由独立网页登录会话发布；旧四项刷新结束时以当前 Code0 状态合成，不能用旧捕获覆盖较新成功或断开。全部服务账户刷新也不得覆盖 Code0 会话状态。保存失败保持暂停，不能先更新 UI 再忽略写盘错误。

## 被动页面桥

只在 owned WebView 主框架 document-start 注入。共享桥逻辑由一份可测试源生成两端资源，检查资源一致性；不复制 DeepSeek 的宽泛整段响应桥。

1. 观察 fetch/XHR，保留原返回对象、异常和事件语义；自身不发网络请求。请求与最终响应均须是 HTTPS、无 userinfo、官方精确同源 `https://console.code0.ai`、默认 443 端口、无 query/hash、精确 GET `/api/status` 或 `/api/user/self`。最终 URL 必须匹配原请求的同一端点，不能仅要求两者分别位于允许集合。fetch 另拒绝 `redirected` 为 true、opaque/opaqueredirect 或缺失可验证URL的响应；XHR 拒绝空或可观察不匹配的 responseURL，但其公开接口不暴露中间跳转链，无法排除跳转后回到同一允许URL。这些检查只约束数值采集，不阻断官网原请求。拒绝非成功 HTTP、非 JSON 和业务失败。
2. 响应处理设 64 KiB 上限；fetch clone 以有界读取终止副本流，XHR 已收到的 body 先检查长度，再解析。只选择规定的数值，不将整个 data/响应字符串交给 native。
3. 每次新同步使用 native 生成的 nonce、connection epoch、attempt ID；每个文档/请求有单调序号。控制消息是固定枚举及序号；数值消息只含协议版本、nonce、document/auth/self 序号与三个数。native 拒绝未知字段、超过 2 KiB、错误类型/来源、旧序号、重放或失效实例。
4. 桥不能读取 cookies、localStorage、sessionStorage、凭据/账户字段，不能导出 headers/body。官方脚本在网页中照常使用自己的登录数据不等于应用读取这些数据。
5. status 可以先于或晚于 self，但属于同一文档。新 self 开始立即废弃旧 pending self，异步解析前后都检查序号。成功接受一次即消费 attempt；新页面或迟到解析不能再次发布。

### 官网认证刷新与失效

POST `/api/user/auth/refresh` 开始时增加 auth revision，废弃 pending self，将 request ID 加入在途集合；结束只删除它自己的 request ID。任意 refresh 在途时不发布。全部 refresh 完成后，必须观察**此后新开始**的成功完整 self；早已发出的 self 即使晚到也不合格。同文档已验证的 status 除数可保留。

HTTP2xx refresh 仅表示传输完成，不证明认证成功；应用不解析 auth bundle，通过后续有效 self 才接受数值。refresh 的网络错误/abort/非2xx暂停，包括保守处理409，应用不发重试。200业务失败但没有有效新 self 则到期暂停。

POST `/api/user/login`、`/api/user/login/2fa`、`/api/user/auth/logout`，旧 GET `/api/user/logout` 在开始即使旧数值失效。连接登录阶段可继续由用户操作，但不捕获/发布余额；同步阶段出现这些动作则暂停并清除旧账号数值。主框架替换、断开、停止也撤销旧 attempt。

nonce 是消息关联，不是账户身份认证。本产品准确承诺“当前专用官方连接最后一次完整 self 响应的数值”，不承诺能识别未来未知的全部认证机制或服务端静默账户变化。

## 原生会话与窗口

状态为 disconnected、connecting、syncing、closing、active、paused、cleanupPending。每次尝试一个 owned 窗口，无共享 DeepSeek 网页存储。macOS 使用专用 WKWebsiteDataStore；Windows 使用专用 WebView2 profile，无 Tauri 应用 IPC capabilities 授予远程网页。

连接前取得固定 Code0 profile 的 OS 排他 lease（macOS flock，Windows 独占 share handle 或已证实等价机制），保持到断开清理/正常退出。目录与锁路径由 native 派生并拒绝符号链接替代。其他进程占用则提示忙，不删除锁、不聚焦未知窗口、不发送进程信号。

关闭先撤销采集权、detach bridge、取消 load，再销毁 owned 窗口/网页对象。必须确认该实例销毁/释放后才创建下一轮；调用 close 或隐藏不是确认。关闭失败/10秒内无法确认进入 cleanupPending，后续同步禁止。Disconnect 在 lease 内失效、关闭、删除专用数值缓存、清除仅 Code0 网站存储，完成前不允许重新连接；清理失败不假报已断开完成。

原生 UI 策略在首次 load 前安装：拒绝新窗口、下载、非官方同源导航、外部协议、文件选择、权限/媒体/通知；JS alert/confirm/prompt 明确取消；HTTP身份/客户端证书挑战取消；TLS 使用系统正常验证，不接受证书例外。隐藏尝试始终不 show/focus；拒绝请求即暂停。window.open JS 包装不能替代 native 拦截。缺少可靠 native hook 的平台不得声称隐藏模式已安全完成，须实现最小引擎接缝及受控夹具。

平台落实：Windows 使用不带Tauri/Wry WebView的owned bare Window提供HWND，再通过公开COM接口创建专用WebView2 environment/controller；不注入Tauri内部脚本或注册应用IPC协议。启用同版本Tauri的公开gated WindowBuilder及与锁文件一致的WebView2直接依赖，不修改上游依赖源码。controller先隐藏并停在初始空白状态，UI/STA线程确认全部事件、设置及document-created脚本的异步注册完成后才导航官网；创建请求返回或派发回调本身不是安装成功。macOS14 使用公开媒体/对话框/导航委托；未支持的定位和通知采用 WebKit 的默认拒绝路径并用实际夹具验证，不调用私有接口。隐藏屏幕采集依赖全新、从未显示/聚焦且没有用户手势的文档条件；camera/mic拒绝不能冒称覆盖屏幕选择器。可见登录属于用户主动交互，不承诺能拦截任何未来网页通过真实手势发出的系统屏幕选择请求；应用不自动授予权限，首版没有屏幕采集功能。

可见登录窗口也只接受官方同源流程；未知第三方认证提示不支持，不擅自放行。允许的网站自身仍会发其他网络请求；“只捕获两接口”不能写成“网站只发两请求”。窗口销毁确认不等于全部共享浏览器后台进程终止，不杀浏览器进程。

## 有限调度与恢复

登录窗口最长10分钟；隐藏同步最长45秒；关闭确认最长10秒。自动间隔最少300秒，从上次开始计，使用注入单调时钟。新连接首次尝试可立即开始；已经有一次尝试后的手动/定时同步都遵循300秒下限，并显示剩余等待。休眠恢复只合并一个到期动作，不补跑所有 tick。

active 且无 owned 窗口/closing/cleanupPending 才能开始自动尝试。成功先持久化两个数值，再关闭确认后 active；失败暂停本次运行自动授权。缓存/profile 存在不构成授权；重启始终暂停，消除脏在途状态自动恢复的风险。用户明确恢复后先执行一次新验证，再恢复本次运行自动同步。

已连接用户隐藏 Code0 仅改变显示，不隐式注销；Services 中状态仍显示后台连接与断开入口。全局刷新不能绕过频率、独占或暂停。

## 兼容、验证与发布

旧偏好迁移明确隐藏新 provider，但保留旧四服务排列/子集和旧三可见迁移；新用户显式选择五项后，native窗口、轮廓、拖动区域与DPI布局按五行计算。语言沿用各平台现有设置：macOS英语、简体中文、繁体中文；Windows英语与简体中文。本次Code0标签覆盖这些既有语言，不扩展Windows整机语言选项。官网品牌标识、数值无币种符号、widget默认三项全部验证。

按测试先行提交可编译拒绝型契约与有意义的行为 RED，在已授权的临时 GitHub macOS/Windows CI 运行。不得在宿主运行产品测试、真实 CLI、真实账户登录或收费请求。受控网页夹具覆盖实际 bridge、native弹窗拒绝与销毁；纯状态测试覆盖并发/迟到回调/299与300秒/保存失败/清理失败。独立审查后运行正常完整双平台及文档/公开发布检查，按既有授权合并并发布。

真实官网 cookie、2FA、未来网页变更和真实余额准确性仍需现场证据；合成测试不冒充真实账户验收。协议不满足时明确暂停，不补猜接口或伪造 fresh。首版不要求先获取用户真实账户才可开发；真实登录不能由开发测试擅自执行。
