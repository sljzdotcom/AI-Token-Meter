# 2026-09-01 详情窗口置前修复

## 背景与目标

桌面已有普通应用窗口时，点击浮动条中的 Claude、Codex 或 DeepSeek 图标，详情窗口会留在普通窗口下方。用户已经主动点击详情入口，因此详情应临时显示在普通应用窗口上方；浮动条自身仍应保持桌面层行为。

对应需求：`REQ-20260901-005`。

## 根因

`FloatingPanelPresentationPolicy` 原来只定义一个层级：桌面图标层级加一。`FloatingPanelController` 创建浮动条和详情面板时都应用该策略。

`orderFrontRegardless()` 只能在当前窗口层级内部置前，无法让桌面层详情跨越普通窗口层级，因此问题不是点击或渲染失败，而是两个职责不同的窗口共享了错误的层级合同。

## 实现与关键决定

- 新增 `FloatingPanelPresentationRole.strip` 与 `.detail`；
- 浮动条继续使用桌面图标层级加一，保持桌面专属行为；
- 详情使用 AppKit 标准 `.floating` 层级，高于普通窗口、低于系统级界面；
- Claude、Codex 继续使用非激活置前，不抢当前应用键盘焦点；
- DeepSeek 继续激活 App 并让 WebView 获得键盘焦点；
- 两种角色继续共享现有 Space 行为，不加入全屏 Space；
- 关闭、自动隐藏、点击空白和切换 Space 仍会把详情 `orderOut`，没有增加永久置顶设置。

## 测试驱动证据

先把原来的“两个面板使用相同层级”测试改为角色合同，并新增详情高于普通窗口的断言。修复前定向测试按预期编译失败：`FloatingPanelPresentationRole`、`level(for:)` 和带角色的 `apply` 均不存在。

实现最小角色策略后：

```text
swift test --filter FloatingPanelPresentationPolicyTests
4 tests in 1 suite passed
```

完整回归：

```text
bash scripts/test.sh
268 tests in 55 suites passed
0 failures
```

4 项需要外部凭据或签名条件的既有测试按门控跳过，没有新增跳过项。

## Release、安装与真实窗口验收

- `bash scripts/build-app.sh` 完成 arm64 Release；本机无 Apple Development identity，Widget 按 `REQ-20260901-003` 既定延期策略跳过，主应用正常构建；
- `codesign --verify --deep --strict` 通过，Bundle `0.1.0` build `1`；
- 构建产物与 `/Applications/AI Token Meter.app` 可执行文件 SHA-256 均为 `e202a51b30a38b04914b100c546580b0f2184baa731ebbcfd206d3468b3efc0c`；
- 旧应用已移动到 `/private/tmp/ai-token-meter-install.80XCq7/AI Token Meter.app`，可在本次系统临时目录仍存在时恢复；
- 在普通 Finder 窗口存在的桌面上，通过真实浮动条打开 Provider 详情；DeepSeek 原生 30 天详情正常显示并保持输入焦点策略；
- 对正在运行且详情可见的已安装 App 做只读实时检查：`KeyboardAccessibleStripPanel` 层级为 `-2147483602`，`InteractivePanel` 层级为 `3`、`isVisible = true`、`isKeyWindow = true`。AppKit 普通窗口层级为 `0`，因此详情实际位于普通应用窗口上方，而浮动条仍在桌面层；
- 检查后调试器已立即分离，没有修改运行时对象或用户偏好。详情自动隐藏仍为用户原来的 8 秒。

## 文档与兼容性

README、Settings 参考和故障排查已区分“浮岛桌面层”与“临时详情 floating 层”。旧版 Changelog 仍保留曾经把两者降为桌面层的历史记录，并新增本次用户可见修复，避免改写历史。

本次不改数据模型、Provider 采集、缓存、账户、Keychain、Widget 快照、通知或隐私边界。

## Git 节点

| 提交 | 内容 |
| --- | --- |
| `e25371c` | 需求、设计规格与实施计划 |
| `abb1a96` | 角色化窗口层级、控制器接线与回归测试 |

## 后续队列

- `REQ-20260901-003`：Widget Apple Development 证书与真实 Gallery 验收，继续延期；
- `REQ-20260901-004`：Mission Control、第二普通 Space、真实指针拖动和多显示器补验，继续受环境限制。

