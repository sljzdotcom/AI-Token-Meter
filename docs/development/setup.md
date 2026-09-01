# 开发环境

## 工具链

- Apple Silicon Mac；
- macOS 14 或更新版本；
- Xcode Command Line Tools；
- Swift 6；
- Git。

检查版本：

```bash
swift --version
git --version
```

项目采用 Swift Package，不依赖 Xcode 工程文件或第三方包管理器。推荐使用 `scripts/test.sh`，因为它会把 SwiftPM 与 Clang 缓存放入临时目录，在 Dropbox、受限执行环境和用户级缓存不可写时仍能稳定运行。

## 获取与构建

```bash
git clone <repository-url>
cd AI-Meter
swift build
bash scripts/test.sh
```

打包本机应用：

```bash
bash scripts/build-app.sh
```

脚本默认把高频 SwiftPM 临时状态放到系统临时目录，并输出 `dist/AI Token Meter.app`。可通过 `AI_METER_BUILD_DIR` 指定其他构建目录：

```bash
AI_METER_BUILD_DIR=/private/tmp/ai-meter-build bash scripts/build-app.sh
```

不要把构建缓存、`dist/` 或已签名应用提交到仓库。

## 运行方式

### 正常模式

```bash
open "dist/AI Token Meter.app"
```

正常模式会尝试读取已安装 CLI，并在配置 DeepSeek API Key 后访问官方余额 API。

### Demo 模式

开发界面时可使用固定脱敏数据，避免访问真实账户：

```bash
AI_METER_DEMO_MODE=1 open "dist/AI Token Meter.app"
```

Demo 模式不会启动三项实时采集器，适合布局、无障碍和窗口交互验证。它不能替代真实数据源的集成测试。

## 代码约定

- App 层只处理 macOS 生命周期和 UI；可测试业务逻辑放入 `AIMeterCore`。
- 新数据源先转换成 `UsageSnapshot`，不要让 View 解析外部响应。
- 进程、网络和网页响应必须设置超时、大小或来源边界。
- 错误消息进入缓存、日志或通知前必须去敏。
- 新行为先增加测试，再做最小实现。
- 不在代码、fixture、文档或提交信息中写入真实密钥和账户响应。

更多目录规则见 [代码库结构](../architecture/repository-structure.md)。

## Git 工作流

推荐使用短生命周期功能分支：

```text
codex/<topic>
```

提交采用明确的类型前缀：

- `feat:` 新功能；
- `fix:` 修复；
- `docs:` 文档；
- `test:` 测试；
- `refactor:` 不改变行为的结构调整；
- `chore:` 工具链或维护；
- `release:` 版本发布。

每个关键节点应保持可构建、可测试，并把验证结果写入开发日志。
