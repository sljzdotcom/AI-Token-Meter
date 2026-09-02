# 发布流程

## 版本策略

AI Token Meter 使用语义化版本思路：

- MAJOR：不兼容的数据、配置或系统要求变化；
- MINOR：向后兼容的新功能；
- PATCH：向后兼容的问题修复。

当前 `Info.plist` 版本为 `0.1.0`、build `1`。仓库尚未创建 Git tag；在真正发布下一版本前，后续改动保留在 `CHANGELOG.md` 的 `Unreleased`。

## 发布前检查清单

### 代码与测试

- [ ] 工作区只有本次发布所需改动；
- [ ] `bash scripts/test.sh` 全部通过；
- [ ] 环境允许时，真实 CLI 冒烟测试 3/3 通过；
- [ ] `bash scripts/build-app.sh` 完成 release 构建；
- [ ] 若发布 Widget，`AI_METER_INCLUDE_WIDGET=1 bash scripts/build-app.sh` 与 `scripts/verify-widget-bundle.sh` 通过；
- [ ] `git diff --check` 无错误。
- [ ] `scripts/check-docs.sh` 无断链、版本或目录治理错误。

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

当前脚本的 Apple Development 签名仍只用于本机开发。公开分发前还必须补充：

- universal binary（若计划支持 Intel）；
- Hardened Runtime；
- Developer ID Application 签名；
- Apple notarization 与 stapling；
- 可验证的发布校验和；
- 清晰的软件许可证。

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
6. 推送分支与 tag。

只有在版本元数据、Changelog、构建产物和验收同时完成后，才能把 `Unreleased` 宣布为已发布版本。
