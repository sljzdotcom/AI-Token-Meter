# 测试指南

## Google Antigravity 验证（当前开发）

当前第四服务使用官方 `agy -p /usage` 的非交互模式。普通测试通过四行无账号合成契约验证 Gemini 与 Claude/GPT 各自的 Five Hour 和 Weekly 窗口、剩余转已用、重置时间、顺序变化、缺行/重复/未知列、非法百分比和未来主版本失败关闭。macOS 另验证普通 Pipe 进程而非 PTY、超时、输出上限、认证分类和私有临时目录；Windows 验证有界隐藏进程、Job Object 清理、官方 Native 路径与旧 `gemini` 不回退。测试不读取真实 Google 账户或个人额度。

## 旧 Gemini CLI 验证（0.6.0 历史）

0.6.0 最终源码的 Windows 前端109项、macOS宿主Rust256项、严格Clippy和前端构建通过；PR、main与正式发布流水线也完成了原生Windows SDK/ConPTY编译和运行验收。当时的固定官方CLI合成账号与PTY回归见[采集日志](2026-09-08-gemini-collector.md)和历史契约；当前生产与测试树已迁移到Antigravity，不再提供旧CLI的opt-in执行入口。

## 普通测试

```bash
bash scripts/test.sh
```

当前候选的基线为 **499 个测试全部通过**：478项普通测试、3项独立刷新调度和18项PTY runner。当前完整验证先运行普通测试，再用独立测试进程运行主线程刷新调度和PTY runner测试，避免CI runner的并行主线程负载干扰有界调度断言。Antigravity使用普通有界进程、严格四窗口解析与无损缓存迁移回归，并覆盖fresh/cached/unavailable详情底板与青绿色强调。品牌回归用2×位图渲染真实菜单面板并核对40pt前导图标。浮动条原生渲染回归使用明确的2×Retina位图，核对Mini 65pt、Compact 78pt、Comfortable 108pt、48/60pt圆环、14pt内凹收起把手、上下镜像肩部、暴露侧连续圆角、30pt/px反向肩弧、底部无Settings入口、左右边缘、1至4项圆周包含、自动收起关闭、已展开密度即时调整和内容安全过渡；调度合同要求25ms轮询上限可兑现50ms设置步进。Windows前端测试文件串行调度，继续使用Vitest默认5秒单项上限，避免发布runner上多个jsdom环境并行冷启动造成资源竞争。并发PTY fixture只使用Shell内建读取，不在32路命令之上额外派生管道进程。传入`--filter`等参数时仍只运行调用者指定的单次测试命令。Keychain隔离读写、已安装Claude Code auth状态、已安装Claude Code CLI额度快照和已安装OpenAI Codex CLI额度快照是环境门控检查；当前环境未启用或不具备相应条件时按设计跳过。

以上数字不与Windows相加计算通过率。当前前端为137项组件测试；宿主Rust为254项；密度门禁覆盖25项进程生命周期、8个Antigravity详情状态、9种Provider详情表面、24个浮动条布局、2个收起态和608个文字角色，并核对五页签窄窗可达、三档固定顺序、已展开密度即时调整、自动收起关闭后延迟控件禁用且值不变、底部无Settings入口和键盘焦点。旧 Gemini ConPTY、分片和终端输入夹具已随生产交互路径移除；Claude/Codex仍使用的PTY与Windows原生进程门禁保持。

普通测试覆盖：

- 用量模型和百分比边界；
- ANSI/终端输出净化；
- Claude Code 与 OpenAI Codex 解析 fixture；
- 进程超时、取消和终止；
- DeepSeek API 映射与敏感错误清理；
- 刷新协调、缓存降级和通知阈值；
- 菜单栏与圆环展示计算；
- 自动隐藏、外部点击和交互暂停；
- OpenAI Codex 重置额度映射；
- OpenAI Codex 重置券到期自然日状态、稳定排序、不完整明细提示和自适应详情高度；
- OpenAI Codex 本机 SQLite 数值行解析、30 天窗口、连续天数、最长会话与紧凑文案；
- OpenAI Codex 的 nvm/Node 管理器目录发现、桌面 App 内置二进制后备、搜索优先级，以及 Node shebang 在 Finder 环境中的运行 PATH；
- DeepSeek 30 天补零、缓存、当前官网 amount/cost 分片解析与完整性合并。
- 浮岛位置偏好默认值、NaN/损坏回退、持久化和垂直夹紧；
- 左右贴边、自动吸附、固定侧拖动、稳定显示器身份、旧编号迁移、主副屏目标优先、Settings 主动重绑定、显示器断开无损回退和详情展开方向；
- 浮岛轮廓渲染边缘、品牌 Logo 光学校正、视觉层级和 App Icon Bundle 声明；
- 玻璃拖动命中区、AppKit 指针状态、无障碍移动、详情交互状态所有权、键盘/VoiceOver 自动隐藏暂停和非颜色状态标记；
- App 启动不被 Keychain 阻塞，以及 DeepSeek 密钥读取的隔离超时、独立 GCD 单调时钟截止时间与单次在途保护；即使 Swift 协作线程池被阻塞，迟到凭据也不能进入网络层。
- Settings 四 Tab 顺序、服务/登录项反馈路由、品牌文案与真实 `Info.plist` 兼容身份。
- Widget 脱敏快照、固定 Provider 顺序、DeepSeek 基准消耗、最近重置与充值券摘要；
- Widget 原子 App Group 存储、损坏/未知版本降级和个人标识/凭证回归；
- Small/Medium/Large 布局合同、Small 无文字源码合同、时间线过期状态和扩展禁止网络/CLI/Keychain 合同；
- Widget 条件打包、签名顺序、App Group 与沙箱验证脚本合同。
- 发布版资源优先从主 App Bundle 解析且不得回退到构建机绝对路径；旧 SwiftPM 嵌套资源布局会被验证器拒绝。
- 软件更新状态转换、按钮能力、动作去重、固定安全错误映射和 Sparkle 代理事件桥接；
- Sparkle 更新交互前只隐藏 Settings、保留普通窗口并激活应用的窗口置前策略；
- Sparkle 版本/校验和锁定、Info.plist 手动检查策略、framework/helper 嵌入、`@rpath`、嵌套签名与发布脚本安全合同；
- appcast enclosure 的版本、build、长度与 EdDSA 签名验证，以及篡改归档必须被拒绝。

## CLI 安装与登录引导回归（0.5.0 起）

关联 [CLI 引导规格](../design/specifications/2026-09-07-cli-onboarding-design.md)。测试只使用注入的账户状态、临时 fixture 目录和本地假下载器；不得实际安装、重装 CLI，或调用真实登录/退出流程。

- 状态：未安装→安装中→重新发现→待登录→已连接；无法检查保持未知，不显示成未安装；账号仍可见。
- 操作：同一服务重复点击、迟到的检查结果、超时后重试、打开终端失败，均不留下永久禁用或重复终端；安装发现已有 CLI 后不重装。
- 脚本：下载失败不能执行残缺文件；成功执行使用匹配的解释器；失败退出可见、临时文件清理、特殊字符路径安全；前端不能提供任意安装 URL 或命令。
- Windows：官方独立安装目录发现；显式 WSL/自定义路径缺失不会改装 Native；PowerShell 实际 fixture 执行由 Windows 原生 CI 覆盖，不用 macOS 字符串断言冒充。
- Windows 页签：英/中四项均图标+名称，装饰图标不重复朗读，点击与键盘入口保留；16px currentColor 和系统字体不改变原密度。

真实安装流程只在独立测试设备、用户明确选择后人工验收，确认终端可见、可取消、可重查且完成后更新账号卡。自动化 fixture/CI 不等于真实网络安装或浏览器 MFA 全流程已验收。当前结果和版本状态见[开发记录](2026-09-07-cli-onboarding.md)。

## 多显示器回归（0.4.0 起）

普通测试覆盖模式解析、断开回退、重连、迁移、每屏位置、指针选屏和详情所有权。额外 AppKit 回退/固定边缘恢复测试需要 WindowServer 会话：

```bash
AI_METER_SCREEN_TESTS=1 bash scripts/test.sh --filter AppModelDisplaySettingsTests
```

测试使用独立 UserDefaults suite，不操作真实账户；未设置环境变量时不创建真实面板。单屏控制器测试不替代真实双屏拖动、拔插、主屏切换、睡眠/唤醒或 Windows 多 DPI 验收。

本轮现场验收按以下顺序记录，不要求再次提供账户密钥：

1. 两屏左右、上下或错位排列：单屏模式从空白处拖到另一屏；macOS 分别验证 Automatic、Left、Right，松手后目标正确并能在重启后恢复。
2. Settings 切 Primary、Selected、All；All 每屏只有一个浮动条，不增加刷新请求，点击另一屏只保留一个详情。
3. 给各屏设置不同边缘和高度；拔掉指定屏后主屏临时接管，重连恢复原记录。回退时用户主动改边或拖动才改目标；仅切固定边缘再回 Automatic 不擦除记忆。
4. 拖动中断屏/切模式、连续快速切模式和语言：无卡死、无旧拖动迟到覆盖、语言不跳回；全屏隐藏按所属显示器生效。
5. Windows 在 100%/125%/200% DPI 下切 English/简体中文及已安装的微软雅黑/黑体/楷体/Antonio；详情标题、正文、按钮、图表和错误态不截字，Settings 和浮动条尺寸不变。未安装字体明确标注并安全回退。

原生 CI 的安装器是未签名更新的 debug 验证资产，不进入正式更新源；物理操作结果另写本轮开发日志，不能用纯策略或浏览器测试代填。

## Keychain 集成测试

Keychain 测试会触及当前 macOS 用户的 Keychain，必须显式开启：

```bash
AI_METER_RUN_KEYCHAIN_TESTS=1 bash scripts/test.sh --filter KeychainStoreTests
```

只在确认测试环境允许创建和删除测试项目时运行。

## 真实 CLI 冒烟测试

真实测试会调用已安装并登录的 Claude Code/OpenAI Codex CLI：

```bash
AI_METER_RUN_CLI_SMOKE=1 bash scripts/test.sh --filter CLIIntegrationSmokeTests
```

当前包含：

1. Claude Code 认证状态；
2. Claude Code 隔离工作区 `/usage`；
3. OpenAI Codex `app-server` 速率限制与本机聚合活动。

这些测试读取真实账户的当前用量，因此不应在未授权的 CI、共享机器或日志会被公开保存的环境中运行。测试结果只能记录成功/失败与耗时，不能提交原始账户输出。

## Release 构建

```bash
bash scripts/build-app.sh
```

构建模式：

```bash
AI_METER_INCLUDE_WIDGET=0 bash scripts/build-app.sh  # 普通 ad-hoc 主应用
AI_METER_INCLUDE_WIDGET=1 bash scripts/build-app.sh  # 必须有 Apple Development 签名
```

独立验证 Widget target 可编译：

```bash
CLANG_MODULE_CACHE_PATH=/private/tmp/ai-meter-widget-clang-cache \
swift build --disable-sandbox \
  --scratch-path /private/tmp/ai-meter-widget-build \
  --product AIMeterWidgetExtension
```

## App Bundle 验证

```bash
plutil -lint "dist/AI Token Meter.app/Contents/Info.plist"
scripts/verify-app-resources.sh "dist/AI Token Meter.app"
codesign --verify --deep --strict --verbose=2 "dist/AI Token Meter.app"
scripts/verify-update-bundle.sh "dist/AI Token Meter.app"
file "dist/AI Token Meter.app/Contents/MacOS/AIMeterApp"
test -s "dist/AI Token Meter.app/Contents/Resources/AppIcon.icns"
```

当前预期：

- `Info.plist` 合法；
- ad-hoc 签名通过严格验证；
- 可执行文件为 arm64 Mach-O；
- 最低系统版本为 macOS 14。
- `AppIcon.icns` 存在且 `CFBundleIconFile` 指向 `AppIcon`。
- Logo 与深海背景直接位于 `Contents/Resources` 的标准子目录，可在脱离源码与 SwiftPM 构建缓存后加载。
- `Sparkle.framework`、Updater、Autoupdate 与两个 XPC helper 完整，主程序通过正确 `@rpath` 加载，并且 feed、公钥和禁用后台检查的键存在。

## Windows 测试与 NSIS

在 Windows 11 x64 开发机执行：

```powershell
npm --prefix windows ci
npm --prefix windows test
npm --prefix windows run test:density
npm --prefix windows run build
cargo fmt --check --manifest-path windows/src-tauri/Cargo.toml
cargo clippy --locked --all-targets --manifest-path windows/src-tauri/Cargo.toml -- -D warnings
cargo test --locked --manifest-path windows/src-tauri/Cargo.toml
npm --prefix windows run tauri build
```

`npm --prefix windows test` 运行 React 组件与状态回归。Antigravity 详情测试核对四个额度窗口、剩余百分比、重置时间、来源版本和认证缓存恢复；旧 Gemini 终端输入夹具已移除。

`test:density` 先用独立配置构建 production fixture，再运行 21 项跨平台进程回收测试，最后由 Vite preview 与 Chrome/Edge headless 加载构建产物并核对详情、Settings、系统字体隔离和原生 select/option 的计算样式。Unix/macOS 先让整个进程组享有 TERM 宽限，再探测全组；leader 已退出但后代仍在时，只有宽限期结束后才 KILL。Windows 保留 `taskkill /T /F`，并使用一次性独立 Chrome profile，防止已有浏览器进程接管 `--dump-dom`；fixture 在 React 同步提交后立即读取计算样式，不依赖后台 `requestAnimationFrame`。门禁需要本机回环端口和可用浏览器。真实 `windows-latest` 覆盖 Credential Manager 隔离 target、ConPTY 输入输出/终端握手、Job Object 回收、Native/WSL 候选策略、Claude/Codex app-server fixture，并编译 DWM 无边框合成、鼠标释放监视、Win32 物理显示器接口、拓扑监听与 WebView2 托管历史窗口，运行其纯策略测试，再验证更新状态与完整 NSIS 生成。可见轮廓由 WebView2 SVG 抗锯齿路径负责，不再使用 GDI `HRGN`。CI runner 不冒充真实显示器拔插、官网真实登录、窗口前台焦点或原生下拉弹层；CI 上传的 debug NSIS 只用于构建回验，正式签名 NSIS 更新资产必须由 Release workflow 注入 Tauri signing secret。

交互式 Windows 真机还必须手工覆盖：左右贴边、125%/200% DPI、多显示器拔插、全屏 Edge 隐藏/恢复、普通窗口上方详情、外部点击关闭、真实指针拖动、Native/WSL 账号显示、DeepSeek WebView2 登录与 30 日图表。CI runner 没有可替代这些视觉/账户证据的桌面会话。

包含 Widget 时再运行：

```bash
scripts/verify-widget-bundle.sh "dist/AI Token Meter.app"
```

它验证 `.appex` 可执行文件、Info.plist、嵌套签名、双方 App Group 和 Widget App Sandbox。当前没有有效 Apple Development 证书的机器只能验证无 Widget 构建与“强制构建清晰失败”保护，不能把 Gallery/桌面验收标为通过。

## 更新归档验证

正式更新发布由单一入口生成；它会先执行完整测试和公开安全扫描，再构建、签名、压缩并更新 `appcast.xml`：

```bash
SPARKLE_TOOLS_DIR="/path/to/Sparkle/bin" \
scripts/package-update-release.sh 0.2.2 6
```

生产 EdDSA 私钥必须已存在于当前用户 Keychain 的 `com.millerpan.AIMeter` 账户中。脚本不会导出私钥。生成后再次独立验证：

```bash
SPARKLE_TOOLS_DIR="/path/to/Sparkle/bin" \
scripts/verify-update-archive.sh \
  appcast.xml \
  dist/releases/0.2.1/AI-Token-Meter-0.2.1-macOS-arm64.zip
```

验证器会读取 appcast enclosure，核对字节长度、版本、build、ZIP 内 App 与签名，并创建临时篡改副本确认验证失败；不会修改正式 ZIP。

双平台发布从维护者 Mac 执行：

```bash
SPARKLE_TOOLS_DIR="/path/to/Sparkle/bin" \
scripts/package-cross-platform-release.sh X.Y.Z-preview.N BUILD
```

脚本使用本机 Keychain 生成 macOS Sparkle 资产，创建草稿 Release，再触发 `.github/workflows/release.yml`。工作流重新验证 macOS ZIP/appcast，构建 Windows NSIS `.exe` 与配套 minisign `.exe.sig`，生成指向该安装器的 `latest.json`，只有两项 job 都通过才解除草稿。Windows 签名 Secret 未配置时必须失败并保留草稿。

## 文档和差异检查

```bash
scripts/check-docs.sh
git diff --check
```

`scripts/test.sh` 在 Swift 测试通过后自动调用同一个文档检查器。它验证 Markdown 相对链接、README 与 `Info.plist` 版本、README 与本页测试基线、必备维护文档，以及旧需求/设计目录不会复发。

## 手工界面验收

至少验证：

- 四个 Logo、圆环方向与真实百分比一致；
- 0% 不绘制虚假最小弧；
- 四个详情都能打开并按设置自动收起；
- Settings 显示 Appearance、Monitoring、Services、About 四个 Tab，并始终使用系统字体；
- About 显示 AI Token Meter、Private AI usage monitor 和真实版本号；
- About 启动后不自动请求 appcast；点击 Check for Updates 后能分别展示新版、最新版和离线安全状态；只有发现新版后 Update Now 才启用；
- 用签名的旧版隔离 fixture 验证 Sparkle 可下载、替换、重新启动到目标版本；绝不在未备份或未授权时覆盖 `/Applications` 正式安装；
- 点击空白处立即关闭，面板内点击不误关；
- DeepSeek 登录交互暂停自动隐藏；
- OpenAI Codex 重置券数量、完整日期、剩余天数无截断，多张券时面板高度受屏幕范围约束；
- 隐藏/恢复悬浮条与多显示器重定位正常；目标屏在线时不因主屏角色或枚举顺序跳屏；
- Automatic 可拖到左右任一侧；0.4.0 起 macOS Left/Right 允许跨屏拖动但松手后固定相应侧，Windows 拖动沿用最近边缘；重启后恢复目标物理屏、侧边和相对高度；
- 目标屏断开时临时回到当前主屏且配置不变；目标屏重新接入后自动恢复；
- 左右轮廓、阴影、背景拖动命中和详情展开方向正确镜像，贴边处无透明空白或可见接缝；
- 四个服务 Logo 在 60 点圆环中视觉重量接近，App Icon 在 Finder 与 Dock 小尺寸可辨认；
- VoiceOver 能读出服务、数值和详情状态；
- 浮岛玻璃表面可通过 VoiceOver 调整动作和普通键盘方向键移动；VoiceOver 阅读详情时不会被自动收起打断；
- 退出 App 后无遗留事件监听或刷新任务。
- Small Widget 只有三个 Logo 状态环且无可见文字；Medium 三卡与主应用口径一致；Large 最近重置与重置券数量/到期准确；
- 退出主应用后 Widget 使用最近脱敏缓存，过期后清晰显示陈旧；点击 Widget 只唤醒主应用；
- 浅色、深色、提高对比度与减少透明度环境下三种尺寸仍可读。
