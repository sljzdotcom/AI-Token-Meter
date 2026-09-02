# 发布流程

## 版本策略

AI Token Meter 使用语义化版本思路：

- MAJOR：不兼容的数据、配置或系统要求变化；
- MINOR：向后兼容的新功能；
- PATCH：向后兼容的问题修复。

当前稳定版本为 `0.2.0`、build `4`，Git tag 为 `v0.2.0`。后续改动保留在 `CHANGELOG.md` 的 `Unreleased`，直到新的版本资产、appcast、签名验证和 CI 一并完成。

## 发布前检查清单

### 代码与测试

- [ ] 工作区只有本次发布所需改动；
- [ ] `bash scripts/test.sh` 全部通过；
- [ ] 环境允许时，真实 CLI 冒烟测试 3/3 通过；
- [ ] `bash scripts/build-app.sh` 完成 release 构建；
- [ ] `scripts/verify-app-resources.sh "dist/AI Token Meter.app"` 确认主应用资源可由迁移后的 App Bundle 直接解析；
- [ ] `scripts/verify-update-bundle.sh "dist/AI Token Meter.app"` 确认 Sparkle framework、helper、`@rpath`、feed、公钥、手动检查策略和严格签名；
- [ ] 若发布 Widget，`AI_METER_INCLUDE_WIDGET=1 bash scripts/build-app.sh` 与 `scripts/verify-widget-bundle.sh` 通过；
- [ ] `git diff --check` 无错误。
- [ ] `scripts/check-docs.sh` 无断链、版本或目录治理错误。
- [ ] `scripts/check-public-release.sh <release.zip>` 对当前内容、完整 Git 历史和分发包无未处理的高置信度秘密。
- [ ] `scripts/verify-update-archive.sh appcast.xml <release.zip>` 核对 enclosure 并证明篡改副本会被拒绝。

### 文档

- [ ] README 功能、截图、系统要求和命令与当前实现一致；
- [ ] 用户指南和排障文档已更新；
- [ ] 架构、目录与隐私文档已更新；
- [ ] `CHANGELOG.md` 把 `Unreleased` 内容移入新版本；
- [ ] `docs/development/commit-history.md` 已记录发布节点；
- [ ] `docs/project-status.md` 的版本、验证和未完成事项已更新；
- [ ] 所有 Markdown 相对链接有效。

### 版本元数据

更新 `Sources/AIMeterApp/Resources/Info.plist`：

- `CFBundleShortVersionString`：公开版本；
- `CFBundleVersion`：递增构建号。

不要只修改 README 徽章而遗漏 App Bundle 版本。

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

### 生成签名更新资产

Sparkle 固定为 `2.9.4`。生产私钥只保存在维护者 macOS Keychain 的 `com.millerpan.AIMeter` 账户中，不得使用导出私钥命令，也不得把 `.key`、`.pem`、Keychain 导出或原始签名环境写入仓库与日志。

使用官方 Sparkle 工具目录执行单一发布入口：

```bash
SPARKLE_TOOLS_DIR="/path/to/Sparkle/bin" \
scripts/package-update-release.sh 0.2.0 4
```

入口按固定顺序执行：完整测试与文档门禁 → 公开安全扫描 → Release 构建 → Sparkle Bundle 验证 → 最终 ZIP → SHA-256 → 官方工具生成 appcast → 独立 enclosure/EdDSA/篡改验证。ZIP 一旦用于生成 appcast 就不得重建；任何字节变化都必须重新生成 enclosure。

发布资产至少包含：

- `AI-Token-Meter-X.Y.Z-macOS-arm64.zip`；
- 同名 `.sha256`；
- 主分支根目录中的 `appcast.xml`。

先提交最终 appcast 和版本文档，再创建 tag、推送 `main` 与 tag，并从同一个 ZIP 创建 GitHub Release。发布后匿名核对 raw appcast、ZIP 下载、SHA-256 和签名。不要让 appcast 指向草稿、私有、已替换或不存在的资产。

## 本机替换与恢复

开发验收时替换 `/Applications/AI Token Meter.app` 前：

1. 完全退出旧版本；
2. 把旧 App 移到带时间戳的安全备份路径；
3. 复制新构建；
4. 核对安装版与构建版可执行文件校验和；
5. 启动并完成三服务界面验收；
6. 若包含 Widget，在 Gallery 验证三种尺寸、共享数据和点击唤醒；
7. 失败时退出并恢复备份。

不要在应用仍运行时直接覆盖 Bundle。

## Git 发布节点

1. 提交所有版本和文档改动；
2. 确认 `git status` 干净；
3. 创建 `release: AI Token Meter vX.Y.Z` 提交；
4. 创建带注释 tag：`vX.Y.Z`；
5. 保存构建校验和和验收结果到开发日志；
6. 推送 `main` 与 tag，创建 GitHub Release 并上传经过验证的同一份 ZIP/SHA；
7. 等待公开 CI 成功，再匿名下载 Release 资产并与本地 SHA-256 对比；
8. 用当前版隔离副本读取线上 appcast，确认显示最新版且不会触发下载。

只有在版本元数据、Changelog、构建产物和验收同时完成后，才能把 `Unreleased` 宣布为已发布版本。

## 回滚更新发布

- 已安装用户回滚：退出 App，从官方 Release 下载上一稳定版并手动替换；用户偏好和非敏感缓存通常保留。
- 尚未广泛安装的错误版本：保留 Git 历史和发布记录，在 appcast 发布更高修复版本；不要把同一版本号的 ZIP 静默替换成不同字节。
- 必须撤回恶意或损坏资产时，可从 appcast 移除对应 enclosure，但要公开说明并立即发布更高版本；不要重用已经发布的版本/build。
- 私钥疑似泄露时立即停止发布，移除受影响 appcast，轮换公开键并发布需要手动安装的可信恢复版本。旧 App 无法仅靠远程 appcast安全更换信任根。
