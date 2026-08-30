# DeepSeek 内嵌登录输入焦点修复日志

日期：2026-08-31  
分支：`codex/deepseek-web-focus`

## 背景与目标

DeepSeek 官方登录页能够在 AI Meter 的详情窗口内加载，但手机号和验证码输入框无法获得键盘焦点。目标是在不改用外部浏览器、不共享其他浏览器 Cookie 的前提下，让隔离的 WebKit 会话可以完成一次官方登录，同时不改变 Claude、Codex 只读详情的焦点行为。

## 问题证据

- 详情选择状态能够打开，说明圆环点击与 SwiftUI 内容切换正常；
- 无障碍检查只能看到悬浮条按钮，网页编辑控件没有进入可交互响应链；
- 详情使用无边框 `NSPanel`，但没有显式允许成为 Key Window，并统一启用了 `becomesKeyOnlyIfNeeded`；
- DeepSeek 分支虽然激活 App 并调用 `makeKeyAndOrderFront`，窗口自身仍缺少完整的键盘焦点契约。

## 实现与关键决定

1. 新增 `FloatingDetailInteractionPolicy`：只有 DeepSeek 请求 App 激活与网页 First Responder；Claude、Codex 保持被动。
2. 新增 `InteractivePanel`：无边框详情允许成为 Key Window，但不会成为 Main Window。
3. 悬浮条继续使用 `.nonactivatingPanel`，交互行为不变。
4. DeepSeek 详情安装 SwiftUI 宿主并成为 Key Window 后，在下一轮主线程调度中把 First Responder 交给仍属于当前详情窗口的 `WKWebView`。
5. 切换或关闭详情时先清理 First Responder，避免已经移除的 WebView 留在响应链。

## 测试驱动证据

- 策略测试先因 `FloatingDetailInteractionPolicy` 不存在而编译失败；最小实现后通过。
- 面板能力测试先因 `InteractivePanel` 不存在而编译失败；最小实现后通过。
- 接入控制器后，定向测试与完整测试均通过：103 个测试、24 个套件、0 失败；4 个需要本机环境的集成检查保持既有跳过状态。

## 安全与隐私

- 手机号和验证码仅进入 `https://platform.deepseek.com` 官方页面；
- 源码、测试、fixture、缓存和日志均不保存手机号、验证码、Cookie 或表单内容；
- 登录 Cookie 继续由 AI Meter 自己的默认 WebKit 数据存储管理；
- CAPTCHA、法律协议与额外安全确认不自动处理。

## 本机验收

待 release 构建、签名、安装和真实登录验证完成后补充。验收记录只写结论，不记录任何登录敏感数据。

## Git 节点

- `6d8b6d1`：实施计划；
- `cbbc4c9`：提供商详情焦点策略；
- `583a1c7`：可交互详情面板与控制器接入。

## 与原计划的差异

原计划引用了不存在的 `docs/development/development-log.md`。仓库实际采用按日期拆分的开发日志，因此本次新增本文件并更新 `docs/development/README.md` 索引，功能范围与验证标准不变。
