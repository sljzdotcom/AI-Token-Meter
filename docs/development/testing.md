# 测试指南

## 普通测试

```bash
bash scripts/test.sh
```

当前基线为 **387 个测试、72 个测试组全部通过**。默认完整验证会先运行 375 项普通测试，再从独立测试进程运行 12 项 PTY 系统资源测试，避免 CI runner 的全套并发负载干扰伪终端时序；并发 PTY fixture 只使用 Shell 内建读取，不在 32 路命令之上额外派生管道进程。传入 `--filter` 等参数时仍只运行调用者指定的单次测试命令。Keychain 隔离读写、已安装 Claude Code auth 状态、已安装 Claude Code CLI 额度快照和已安装 OpenAI Codex CLI 额度快照是环境门控检查；当前环境未启用或不具备相应条件时按设计跳过。

以上数字是当前 macOS 基线；Windows 使用独立 Vitest/Rust 基线，不能把双方项目数相加后写成单一“通过率”。当前 Windows 本机跨平台基线为 37 项前端测试、7 项密度进程生命周期测试和 160 项 Rust 测试，严格 Clippy、production 前端构建及 production preview Chrome 计算样式门禁通过；Windows-only 分支、Tauri 壳与 NSIS 继续由 GitHub `windows-latest` 复验。

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

`test:density` 先用独立配置构建 production fixture，再运行 7 项跨平台进程回收测试，最后由 Vite preview 与 Chrome/Edge headless 加载构建产物并核对详情、Settings、系统字体隔离和原生 select/option 的计算样式。Unix/macOS 先让整个进程组享有 TERM 宽限，再探测全组；leader 已退出但后代仍在时，只有宽限期结束后才 KILL。Windows 保留 `taskkill /T /F`。门禁需要本机回环端口和可用浏览器。真实 `windows-latest` 覆盖 Credential Manager 隔离 target、ConPTY 输入输出/终端握手、Job Object 回收、Native/WSL 候选策略、Claude/Codex app-server fixture，并编译 DWM 无边框合成、鼠标释放监视、Win32 物理显示器接口、拓扑监听与 WebView2 托管历史窗口，运行其纯策略测试，再验证更新状态与完整 NSIS 生成。可见轮廓由 WebView2 SVG 抗锯齿路径负责，不再使用 GDI `HRGN`。CI runner 不冒充真实显示器拔插、官网真实登录、窗口前台焦点或原生下拉弹层；CI 上传的 debug NSIS 只用于构建回验，正式签名 NSIS 更新资产必须由 Release workflow 注入 Tauri signing secret。

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

- 三个 Logo、圆环方向与真实百分比一致；
- 0% 不绘制虚假最小弧；
- 三个详情都能打开并按设置自动收起；
- Settings 显示 Appearance、Monitoring、Services、About 四个 Tab，并始终使用系统字体；
- About 显示 AI Token Meter、Private AI usage monitor 和真实版本号；
- About 启动后不自动请求 appcast；点击 Check for Updates 后能分别展示新版、最新版和离线安全状态；只有发现新版后 Update Now 才启用；
- 用签名的旧版隔离 fixture 验证 Sparkle 可下载、替换、重新启动到目标版本；绝不在未备份或未授权时覆盖 `/Applications` 正式安装；
- 点击空白处立即关闭，面板内点击不误关；
- DeepSeek 登录交互暂停自动隐藏；
- OpenAI Codex 重置券数量、完整日期、剩余天数无截断，多张券时面板高度受屏幕范围约束；
- 隐藏/恢复悬浮条与多显示器重定位正常；目标屏在线时不因主屏角色或枚举顺序跳屏；
- Automatic 可拖到左右任一侧，Left/Right 只允许垂直移动，重启后恢复目标物理屏、侧边和相对高度；
- 目标屏断开时临时回到当前主屏且配置不变；目标屏重新接入后自动恢复；
- 左右轮廓、阴影、拖动提示和详情展开方向正确镜像，贴边处无透明空白或可见接缝；
- 三个服务 Logo 在 60 点圆环中视觉重量接近，App Icon 在 Finder 与 Dock 小尺寸可辨认；
- VoiceOver 能读出服务、数值和详情状态；
- 浮岛玻璃表面可通过 VoiceOver 调整动作和普通键盘方向键移动；VoiceOver 阅读详情时不会被自动收起打断；
- 退出 App 后无遗留事件监听或刷新任务。
- Small Widget 只有三个 Logo 状态环且无可见文字；Medium 三卡与主应用口径一致；Large 最近重置与重置券数量/到期准确；
- 退出主应用后 Widget 使用最近脱敏缓存，过期后清晰显示陈旧；点击 Widget 只唤醒主应用；
- 浅色、深色、提高对比度与减少透明度环境下三种尺寸仍可读。
