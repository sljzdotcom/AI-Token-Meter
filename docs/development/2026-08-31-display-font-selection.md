# 全局显示字体选择开发与验收日志

**日期：** 2026-08-31

**分支：** `codex/deep-sea-background`

**基础节点：** `2229b62b304fd95532f3282ba4f0bf46da283921`

## 1. 背景与目标

本阶段为 AI Meter 增加 System Default、Antonio、DIN Condensed 三种全局显示字体，以及恢复默认操作。新安装和旧版本升级仍以 macOS System Default 为初始状态；第三方字体只从 macOS 已注册字体家族解析，AI Meter 不下载、安装或分发字体文件。

## 2. 实现范围与关键决定

- `DisplayFontChoice` 使用 `system`、`antonio`、`din-condensed` 三个稳定持久化值。
- `AppModel` 负责即时更新和 `UserDefaults` 持久化，不负责 AppKit 字体可用性。
- `AIMeterTypography` 集中处理字体目录、语义字号/字重、环境传播和安全回退；视图不直接读取偏好或散落字体家族名。
- Settings 固定展示三个选项。缺失字体显示 `Not installed` 并禁用；`Restore Default Font` 只在自定义选择下启用。
- 自定义字体不改变 Logo、SF Symbols、圆环、品牌颜色、深海背景、窗口几何或 DeepSeek WebKit 页面。

## 3. TDD 红绿证据

任务 1 的偏好测试先因 `DisplayFontChoice` / `DisplayFontPreferenceStore` 不存在而编译失败；核心实现后，AppModel 测试又因 `displayFontChoice`、设置和恢复方法不存在而失败。最小实现后，3 个偏好测试和 2 个 AppModel 启动测试通过。提交：`8a6d8df`。

任务 2 的 `TypographyTests` 先因 `DisplayFontCatalog`、`AIMeterTypography`、`AIMeterTextStyle` 不存在而失败；最小语义字体层和视图迁移后，字体、视觉、拖动和详情布局测试通过。提交：`9051e3f`。

任务 3 的 Settings 展示测试先因 `DisplayFontSettingsPresentation` 不存在而失败；实现三选项、缺失状态、预览和恢复规则后，字体、视觉和 AppModel 回归通过。提交：`2229b62`。

独立审查没有发现关键或重要问题。逐阶段原始报告保存在 `.superpowers/sdd/2026-08-31-display-font-selection/`（该目录为本地 SDD 工作记录，不进入正式仓库）。

## 4. 完整自动化验证

首次最终候选验证运行：

```bash
bash scripts/test.sh
```

结果：170 个测试、35 个测试套件通过，0 失败；4 个环境门控检查按设计跳过：1 个真实 Keychain 隔离读写检查，以及 Claude auth、Claude collector、Codex collector 三个已安装 CLI 冒烟检查。

最终提交前再次运行同一完整测试命令；结果保持一致。测试期间 SwiftPM/WebKit 对受限缓存目录的诊断不影响退出码或测试结论。

## 5. Release 构建与安装身份

```bash
bash scripts/build-app.sh
codesign --verify --deep --strict "dist/AI Meter.app"
plutil -lint "dist/AI Meter.app/Contents/Info.plist"
shasum -a 256 "dist/AI Meter.app/Contents/MacOS/AIMeterApp"
```

- 版本：`0.1.0`（build `1`）
- Release 可执行文件大小：2,910,240 bytes
- 构建时间：2026-08-31 23:09:21 +0800
- 候选版 SHA-256：`ca8a83ea29abb5f761dced018e9f664053311d4c28f939546f59200ec1822052`
- ad-hoc 签名验证：通过
- Info.plist：`OK`

安装前完全退出 AI Meter，将旧 bundle 移至：

```text
/private/tmp/AI-Meter-app-backup-20260831-230934/AI Meter.app
```

旧 bundle 可执行文件 SHA-256 为 `fbeada3e35dfe5d61449aac929ae6748d61a8a084882fcc639616a24195ddd25`。使用 `ditto` 安装候选版后，`/Applications/AI Meter.app` 再次通过签名和 plist 校验；安装版 SHA-256 与候选版完全一致，`cmp` 退出码为 0。

## 6. 本机字体与 UI 验收状态

安装前 `com.millerpan.AIMeter` 中没有 `appearance.displayFont`，因此新版首次启动保持 System Default，没有擅自迁移界面。原浮岛位置为 Right，`normalizedCenterY = 0.9696969696969697`，即辅助功能描述中的右侧 97%。

已有直接证据的实机步骤：

1. Settings 的 `Display font` 初始值为 System Default，`Restore Default Font` 禁用；三个选项顺序为 System Default、Antonio、DIN Condensed。
2. 选择 Antonio 后，当前 Settings 窗口立即以 Antonio 重绘，恢复按钮启用。
3. Claude、Codex、DeepSeek 按钮均可在不重启的情况下切换 `Detail open` / `Detail closed`；当前数据分别为 Claude 0%、Codex 10%、DeepSeek ¥77.99。该 AX 状态只证明面板交互，不证明其内部文字实际使用的字体或是否截断。
4. 选择 DIN Condensed 后，当前 Settings 窗口立即重绘，标题、说明、百分比和按钮文字保持可读且无 Settings 截断。
5. 点击 `Restore Default Font` 后回到 System Default，恢复按钮重新禁用。
6. 再选择 Antonio，真实退出并重新启动 `/Applications/AI Meter.app`；Settings 仍显示 Antonio，`defaults read com.millerpan.AIMeter appearance.displayFont` 返回 `antonio`。
7. 最终浮岛辅助功能状态为 `Right edge, vertical position 97 percent`，三个详情均关闭；Provider Logo、圆环、深海背景和品牌状态有浮岛截图/独立观察，拖动与布局边界有自动化回归证据。

Computer Use 生成的 Settings 截图保存在本次本机临时目录：

- `/private/tmp/AI-Meter-Task4-01-system-right97.png`
- `/private/tmp/AI-Meter-Task4-02-antonio-settings.png`
- `/private/tmp/AI-Meter-Task4-04-din-settings.png`

当前 Computer Use 的 app-window capture 不包含非激活的详情 `NSPanel` 或菜单栏弹窗，系统级 `screencapture` 也因缺少屏幕录制权限返回 `could not create image from display`。因此无法自动保存或直接观察这些面板的像素级结果。Provider `Detail open/closed` 辅助功能状态、源码字体边界扫描、170 项自动化回归和浮岛视觉确认是补充证据，但不能证明菜单/详情文字的实际字体、字形、截断或小号文字可读性。

仍待在具备直接观察能力的环境中手工完成：

- [ ] Antonio：观察菜单栏面板及 Claude、Codex、DeepSeek 三个详情，核对英文、数字、百分比、余额、长日期、说明和小号文字的字体与截断。
- [ ] DIN Condensed：重复上述菜单与三个详情检查，重点核对小号文字与字形可读性。
- [ ] 在上述观察中确认详情自动隐藏行为未因字体切换出现视觉或交互退化。

这些未完成项不影响已经验证的 Settings 即时切换、默认恢复、偏好持久化、Release/安装身份或最终位置，但在人工观察完成前，不宣告全量真实 UI 验收通过。

## 7. 安全与隐私检查

- 未下载、复制或提交第三方字体文件，也未修改用户字体目录。
- 未读取或记录 API Key、Cookie、授权头或完整账户响应。
- 安装前的应用已保留为可恢复备份，没有删除材料数据。
- 字体偏好和浮岛位置属于非敏感 `UserDefaults`；Keychain 和服务配置保持不变。

## 8. 当前状态

- 安装位置：`/Applications/AI Meter.app`
- 安装身份：SHA-256 `ca8a83ea29abb5f761dced018e9f664053311d4c28f939546f59200ec1822052`
- 显示字体：Antonio
- 浮岛位置：Right，97%
- 备份：`/private/tmp/AI-Meter-app-backup-20260831-230934/AI Meter.app`
- 验收门控：菜单与三个详情在 Antonio/DIN 下的字体、截断、字形和小号文字可读性待手工观察
- 推送/合并：未执行；保留 `codex/deep-sea-background` 分支和工作树等待上层任务处理。
