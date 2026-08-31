# 测试指南

## 普通测试

```bash
bash scripts/test.sh
```

当前基线为 137 个测试、29 个测试组、0 个失败。普通测试中有 4 个环境相关检查按设计跳过：1 个 Keychain 生命周期测试和 3 个真实 CLI 冒烟测试。

普通测试覆盖：

- 用量模型和百分比边界；
- ANSI/终端输出净化；
- Claude 与 Codex 解析 fixture；
- 进程超时、取消和终止；
- DeepSeek API 映射与敏感错误清理；
- 刷新协调、缓存降级和通知阈值；
- 菜单栏与圆环展示计算；
- 自动隐藏、外部点击和交互暂停；
- Codex 重置额度映射；
- Codex 重置券到期自然日状态、稳定排序、不完整明细提示和自适应详情高度；
- Codex 本机 SQLite 数值行解析、30 天窗口、连续天数、最长会话与紧凑文案；
- DeepSeek 30 天补零、缓存、当前官网 amount/cost 分片解析与完整性合并。
- 浮岛位置偏好默认值、NaN/损坏回退、持久化和垂直夹紧；
- 左右贴边、自动吸附、固定侧拖动、显示器断开回退和详情展开方向；
- 浮岛轮廓渲染边缘、品牌 Logo 光学校正、视觉层级和 App Icon Bundle 声明；
- 拖动柄无障碍移动、详情交互状态所有权、键盘/VoiceOver 自动隐藏暂停和非颜色状态标记。

## Keychain 集成测试

Keychain 测试会触及当前 macOS 用户的 Keychain，必须显式开启：

```bash
AI_METER_RUN_KEYCHAIN_TESTS=1 bash scripts/test.sh --filter KeychainStoreTests
```

只在确认测试环境允许创建和删除测试项目时运行。

## 真实 CLI 冒烟测试

真实测试会调用已安装并登录的 Claude/Codex CLI：

```bash
AI_METER_RUN_CLI_SMOKE=1 bash scripts/test.sh --filter CLIIntegrationSmokeTests
```

当前包含：

1. Claude 认证状态；
2. Claude 隔离工作区 `/usage`；
3. Codex `app-server` 速率限制与本机聚合活动。

这些测试读取真实账户的当前用量，因此不应在未授权的 CI、共享机器或日志会被公开保存的环境中运行。测试结果只能记录成功/失败与耗时，不能提交原始账户输出。

## Release 构建

```bash
bash scripts/build-app.sh
```

## App Bundle 验证

```bash
plutil -lint "dist/AI Meter.app/Contents/Info.plist"
codesign --verify --deep --strict --verbose=2 "dist/AI Meter.app"
file "dist/AI Meter.app/Contents/MacOS/AIMeterApp"
test -s "dist/AI Meter.app/Contents/Resources/AppIcon.icns"
```

当前预期：

- `Info.plist` 合法；
- ad-hoc 签名通过严格验证；
- 可执行文件为 arm64 Mach-O；
- 最低系统版本为 macOS 14。
- `AppIcon.icns` 存在且 `CFBundleIconFile` 指向 `AppIcon`。

## 文档和差异检查

```bash
git diff --check
```

发布前还应验证所有 Markdown 相对链接存在、README 版本号与 `Info.plist` 一致、示例命令可从仓库根目录执行。

## 手工界面验收

至少验证：

- 三个 Logo、圆环方向与真实百分比一致；
- 0% 不绘制虚假最小弧；
- 三个详情都能打开并按设置自动收起；
- 点击空白处立即关闭，面板内点击不误关；
- DeepSeek 登录交互暂停自动隐藏；
- Codex 重置券数量、完整日期、剩余天数无截断，多张券时面板高度受屏幕范围约束；
- 隐藏/恢复悬浮条与多显示器重定位正常；
- Automatic 可拖到左右任一侧，Left/Right 只允许垂直移动，重启后恢复位置；
- 左右轮廓、阴影、拖动提示和详情展开方向正确镜像，贴边处无透明空白或可见接缝；
- 三个服务 Logo 在 60 点圆环中视觉重量接近，App Icon 在 Finder 与 Dock 小尺寸可辨认；
- VoiceOver 能读出服务、数值和详情状态；
- 拖动柄可通过 VoiceOver 调整动作和普通键盘方向键移动；VoiceOver 阅读详情时不会被自动收起打断；
- 退出 App 后无遗留事件监听或刷新任务。
