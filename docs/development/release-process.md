# 发布流程

## 版本策略

AI Token Meter 使用语义化版本思路：

- MAJOR：不兼容的数据、配置或系统要求变化；
- MINOR：向后兼容的新功能；
- PATCH：向后兼容的问题修复。

当前稳定版本为 `0.2.2`、build `6`，稳定 tag 为 `v0.2.2`；当前已公开双平台 Preview 为 `0.3.0-preview.2`、build `9`。稳定版的 macOS ZIP、SHA-256、appcast、EdDSA、公开重新下载、最终 CI 与隔离真实更新均已验证；它没有 Windows 正式资产。双平台 Preview 从 `0.3.0-preview.0` 开始，macOS 与 Windows 使用同一个 `VERSION`、tag 和 GitHub Release。Windows DeepSeek 窗口与密度修复目前只记录在 `CHANGELOG.md` 的 `Unreleased`，下一版本尚未选定；必须等待 Windows runner 和真机验收，不能沿用已公开的 `preview.2` 资产冒充修复版本。

## 发布前检查清单

### 代码与测试

- [ ] 工作区只有本次发布所需改动；
- [ ] `bash scripts/test.sh` 全部通过；
- [ ] `npm --prefix windows test`、`npm --prefix windows run test:density`、`npm --prefix windows run build` 全部通过；
- [ ] 环境允许时，真实 CLI 冒烟测试 3/3 通过；
- [ ] `bash scripts/build-app.sh` 完成 release 构建；
- [ ] `scripts/verify-app-resources.sh "dist/AI Token Meter.app"` 确认主应用资源可由迁移后的 App Bundle 直接解析；
- [ ] `scripts/verify-update-bundle.sh "dist/AI Token Meter.app"` 确认 Sparkle framework、helper、`@rpath`、feed、公钥、手动检查策略和严格签名；
- [ ] 若发布 Widget，`AI_METER_INCLUDE_WIDGET=1 bash scripts/build-app.sh` 与 `scripts/verify-widget-bundle.sh` 通过；
- [ ] `git diff --check` 无错误。
- [ ] `ruby scripts/check-cross-platform-contracts.rb .` 确认根版本、macOS plist、Windows npm/Cargo/Tauri、Provider Schema、fixture 与发布工作流一致。
- [ ] `scripts/check-docs.sh` 无断链、版本或目录治理错误。
- [ ] `scripts/check-public-release.sh <release.zip>` 对当前内容、完整 Git 历史和分发包无未处理的高置信度秘密。
- [ ] `scripts/verify-update-archive.sh appcast.xml <release.zip>` 核对 enclosure 并证明篡改副本会被拒绝。
- [ ] Windows CI 在 `windows-latest` 通过 frontend、Rust、Credential Manager/ConPTY/Win32 集成测试并生成 current-user NSIS。
- [ ] Windows Release job 生成唯一 NSIS `setup.exe` 与配套 `.exe.sig`，错误/缺失 Tauri 签名 secret 会失败而不是生成未签名更新清单。

### 文档

- [ ] README 功能、截图、系统要求和命令与当前实现一致；
- [ ] 用户指南和排障文档已更新；
- [ ] 架构、目录与隐私文档已更新；
- [ ] `CHANGELOG.md` 把 `Unreleased` 内容移入新版本；
- [ ] `docs/development/commit-history.md` 已记录发布节点；
- [ ] `docs/project-status.md` 的版本、验证和未完成事项已更新；
- [ ] 所有 Markdown 相对链接有效。
- [ ] README/用户指南区分 macOS ZIP 与 Windows NSIS，诚实说明 Gatekeeper、SmartScreen、Widget 和真机验收边界。

### 版本元数据

更新 `Sources/AIMeterApp/Resources/Info.plist`：

- `CFBundleShortVersionString`：公开版本；
- `CFBundleVersion`：递增构建号。

不要只修改 README 徽章而遗漏 App Bundle 版本。

还必须同步修改：

- 根 `VERSION`；
- `windows/package.json` 与 `package-lock.json`；
- `windows/src-tauri/Cargo.toml` 与 `Cargo.lock`；
- `windows/src-tauri/tauri.conf.json`。

`scripts/check-cross-platform-contracts.rb` 会阻止上述任一版本漂移。macOS build 号继续单独递增；Windows 使用同一语义版本。

### 打包与签名

```bash
bash scripts/build-app.sh
plutil -lint "dist/AI Token Meter.app/Contents/Info.plist"
codesign --verify --deep --strict --verbose=2 "dist/AI Token Meter.app"
file "dist/AI Token Meter.app/Contents/MacOS/AIMeterApp"
```

无 Widget 构建可明确执行：

```bash
AI_METER_INCLUDE_WIDGET=0 bash scripts/build-app.sh
```

Widget 构建需要 Xcode 中可用的 Apple Development 证书：

```bash
AI_METER_INCLUDE_WIDGET=1 bash scripts/build-app.sh
scripts/verify-widget-bundle.sh "dist/AI Token Meter.app"
```

默认 `auto` 只有在同时检测到身份和 Team ID 时才嵌入 `AITokenMeterWidget.appex`；显式 `1` 在条件不足时必须失败。构建计算 `${TEAM_ID}.com.millerpan.AIMeter`，写入双方 entitlements 与 Bundle 元数据，先签扩展再签主应用。不得把 ad-hoc 签名 Widget 当成成功产物。

当前 GitHub 预览发行允许提供明确标注为 ad-hoc、未公证的 Apple Silicon ZIP，用户需要通过 Finder 右键“打开”。要升级为无需该提示的正式 macOS 发行渠道，还必须补充：

- universal binary（若计划支持 Intel）；
- Hardened Runtime；
- Developer ID Application 签名；
- Apple notarization 与 stapling；
- 可验证的发布校验和（GitHub 预览发行已经要求）；
- 清晰的软件许可证（当前为 MIT）。

Windows NSIS 使用 current-user 安装模式与 WebView2 download bootstrapper。首个 Preview 可没有 Authenticode，但必须附 SHA-256 并说明 SmartScreen；应用内更新仍必须有 Tauri minisign 签名。Authenticode 与 updater signature 解决不同问题，文档不得混称“已签名发布者”。

### 生成签名更新资产

Sparkle 固定为 `2.9.4`。生产私钥只保存在维护者 macOS Keychain 的 `com.millerpan.AIMeter` 账户中，不得使用导出私钥命令，也不得把 `.key`、`.pem`、Keychain 导出或原始签名环境写入仓库与日志。

使用官方 Sparkle 工具目录执行单一发布入口：

```bash
SPARKLE_TOOLS_DIR="/path/to/Sparkle/bin" \
scripts/package-update-release.sh 0.2.2 6
```

入口按固定顺序执行：完整测试与文档门禁 → 公开安全扫描 → Release 构建 → Sparkle Bundle 验证 → 最终 ZIP → SHA-256 → 官方工具生成 appcast → 独立 enclosure/EdDSA/篡改验证。ZIP 一旦用于生成 appcast 就不得重建；任何字节变化都必须重新生成 enclosure。

发布资产至少包含：

- `AI-Token-Meter-X.Y.Z-macOS-arm64.zip`；
- 同名 `.sha256`；
- Release staging 目录中的 `appcast.xml`。

先提交版本与文档，再创建 tag、推送 `main` 与 tag，并从同一个 ZIP 创建 GitHub 草稿 Release。稳定版根 `appcast.xml` 只能在 Release 资产已公开后由发布 workflow 更新，避免已安装用户读到指向草稿、私有或不存在资产的 enclosure。发布后匿名核对 raw appcast、ZIP 下载、SHA-256 和签名。

## 双平台草稿 Release

macOS Sparkle 私钥继续只保存在维护者 Keychain；Windows Tauri 私钥只允许存入 GitHub Actions Secret `TAURI_SIGNING_PRIVATE_KEY`，密码（如有）使用 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`。两种私钥都不得进入 Git、命令输出、Artifact 或 Release。

完成版本/CHANGELOG/文档提交后，在 `main` 干净工作树执行：

```bash
SPARKLE_TOOLS_DIR="/absolute/path/to/Sparkle/bin" \
scripts/package-cross-platform-release.sh X.Y.Z-preview.N BUILD
```

该入口按以下顺序工作：

1. 本机完整测试并用 Keychain 生成已签名 macOS ZIP、SHA-256 和 appcast；
2. 确认生成过程没有修改源码或公开稳定 appcast，创建并推送同版本 tag；
3. 创建包含 macOS 三项资产的 GitHub 草稿 Release；
4. 以该 tag 触发 `release.yml`；
5. macOS job 重新下载草稿资产、核对 SHA 与 Sparkle 签名事实；Windows job 重跑测试并生成 NSIS `setup.exe`、`.exe.sig` 和 SHA，并用应用内同一 Tauri 公钥实际验证安装器更新资产；
6. publish job 生成只含 `windows-x86_64` 的 `latest.json`，校验并上传 Windows 资产，随后才把草稿改为公开；
7. 稳定版在资产公开后才提交根 `appcast.xml`，随后稳定版与 Preview 都推进独立 `windows-preview-feed`；Preview Release Notes 会明确说明无 Authenticode 时的 SmartScreen `unknown publisher` 提示、SHA-256 人工核验与 minisign 更新边界。

workflow 会先备份根 `appcast.xml` 与固定 Preview feed，并用全局并发锁串行化不同版本的发布。公开后的 feed 步骤失败时，补偿步骤只恢复本轮实际触碰过的 feed；随后重新读取两份公开 feed，只有二者均确认不再引用目标版本，且本轮确实执行过草稿到公开的转换，才把目标 Release 改回草稿。任何恢复、鉴权、网络或存在性探测失败都会保留公开资产，避免形成“更新源可见但下载资产被撤下”的断链。只有明确 HTTP 404 才代表 Preview feed 不存在。

同版本恢复重跑可识别已经公开的 Release，下载并复用线上实际 Windows installer/updater/signature 来重建 feed，不使用可能字节不同的重编译归档，也绝不会因为重跑失败而撤下既有公开版本。公开前失败则 Release 原本就保持草稿。Windows updater endpoint 固定为 GitHub Release 的 `latest/download/latest.json`；macOS 继续读取仓库根 appcast。双方版本必须相同，但清单格式不混用。

## 本机替换与恢复

开发验收时替换 `/Applications/AI Token Meter.app` 前：

1. 完全退出旧版本；
2. 把旧 App 移到带时间戳的安全备份路径；
3. 复制新构建；
4. 核对安装版与构建版可执行文件校验和；
5. 启动并完成三服务界面验收；
6. 若包含 Widget，在 Gallery 验证三种尺寸、共享数据和点击唤醒；
7. 失败时退出并恢复备份。

Windows 隔离升级必须使用普通测试用户，从 `preview.0` 手动检查并升级到 `preview.1`，确认设置/Credential Manager 保留、采集子进程在安装前结束、NSIS 置前、原位替换和重新启动。另用错误签名 feed 证明旧版本仍可启动。没有这两组证据不能把 Windows 更新标为完成。

不要在应用仍运行时直接覆盖 Bundle。

## Git 发布节点

1. 提交所有版本和文档改动；
2. 确认 `git status` 干净；
3. 创建 `release: AI Token Meter vX.Y.Z` 提交；
4. 创建带注释 tag：`vX.Y.Z`；
5. 保存构建校验和和验收结果到开发日志；
6. 通过跨平台入口推送 `main` 与 tag、创建草稿 Release，并由双平台 workflow 上传经验证的同一组 macOS/Windows 资产；
7. 等待公开 CI 成功，再匿名下载 Release 资产并与本地 SHA-256 对比；
8. 用当前版隔离副本读取线上 appcast，确认显示最新版且不会触发下载。

只有在版本元数据、Changelog、构建产物和验收同时完成后，才能把 `Unreleased` 宣布为已发布版本。

## 回滚更新发布

- 已安装用户回滚：退出 App，从官方 Release 下载上一稳定版并手动替换；用户偏好和非敏感缓存通常保留。
- 尚未广泛安装的错误版本：保留 Git 历史和发布记录，在 appcast 发布更高修复版本；不要把同一版本号的 ZIP 静默替换成不同字节。
- 必须撤回恶意或损坏资产时，可从 appcast 移除对应 enclosure，但要公开说明并立即发布更高版本；不要重用已经发布的版本/build。
- 私钥疑似泄露时立即停止发布，移除受影响 appcast，轮换公开键并发布需要手动安装的可信恢复版本。旧 App 无法仅靠远程 appcast安全更换信任根。
