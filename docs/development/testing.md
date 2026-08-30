# 测试指南

## 普通测试

```bash
swift test
```

当前基线为 100 个测试、22 个测试组、0 个失败。普通测试中有 4 个环境相关检查按设计跳过：1 个 Keychain 生命周期测试和 3 个真实 CLI 冒烟测试。

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
- DeepSeek 30 天补零、缓存与网页负载解析。

## Keychain 集成测试

Keychain 测试会触及当前 macOS 用户的 Keychain，必须显式开启：

```bash
AI_METER_RUN_KEYCHAIN_TESTS=1 swift test --filter KeychainStoreTests
```

只在确认测试环境允许创建和删除测试项目时运行。

## 真实 CLI 冒烟测试

真实测试会调用已安装并登录的 Claude/Codex CLI：

```bash
AI_METER_RUN_CLI_SMOKE=1 swift test --filter CLIIntegrationSmokeTests
```

当前包含：

1. Claude 认证状态；
2. Claude 隔离工作区 `/usage`；
3. Codex `app-server` 速率限制。

这些测试读取真实账户的当前用量，因此不应在未授权的 CI、共享机器或日志会被公开保存的环境中运行。测试结果只能记录成功/失败与耗时，不能提交原始账户输出。

## Release 构建

```bash
swift build -c release
bash scripts/build-app.sh
```

## App Bundle 验证

```bash
plutil -lint "dist/AI Meter.app/Contents/Info.plist"
codesign --verify --deep --strict --verbose=2 "dist/AI Meter.app"
file "dist/AI Meter.app/Contents/MacOS/AIMeterApp"
```

当前预期：

- `Info.plist` 合法；
- ad-hoc 签名通过严格验证；
- 可执行文件为 arm64 Mach-O；
- 最低系统版本为 macOS 14。

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
- 隐藏/恢复悬浮条与多显示器重定位正常；
- VoiceOver 能读出服务、数值和详情状态；
- 退出 App 后无遗留事件监听或刷新任务。

