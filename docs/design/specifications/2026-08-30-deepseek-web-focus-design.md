# AI Meter DeepSeek 登录输入焦点修复设计

日期：2026-08-30  
状态：方案 A 已批准，等待书面规格复核

## 背景

DeepSeek 详情页使用嵌入式 `WKWebView` 打开官方 `platform.deepseek.com` 登录页。详情能够显示，但手机号和验证码输入框无法获得键盘焦点，因此无法在 AI Meter 的隔离 WebKit 会话中完成首次登录。

现有实现把详情窗口创建为无边框 `NSPanel`，并统一设置 `becomesKeyOnlyIfNeeded = true`。DeepSeek 分支虽然调用了 `NSApp.activate` 和 `makeKeyAndOrderFront`，但无边框面板没有显式声明可以成为 Key Window。Computer Use 验证也显示详情选择已打开，却没有可聚焦的 WebKit 编辑控件。问题位于 AppKit 窗口焦点层，不是手机号、DeepSeek 账号或验证码服务。

## 目标

- DeepSeek 详情打开后可以成为 Key Window。
- 用户点击官方登录页的手机号、验证码等输入框时能够正常获得键盘焦点。
- Claude、Codex 的只读详情继续保持不抢占当前应用焦点。
- 保留当前空白点击关闭、自动隐藏、悬停暂停和 DeepSeek 登录期间暂停自动隐藏的行为。
- 修复后在 AI Meter 的隔离 WebKit 会话中完成一次真实登录验收。

## 非目标

- 不改用外部浏览器登录，也不共享 Safari、Chrome、Edge 的 Cookie。
- 不修改 DeepSeek 登录页、验证码规则或官方网络请求。
- 不在 AI Meter 中保存手机号、验证码或登录表单内容。
- 不新增自动登录、验证码历史、账号切换或退出登录界面。
- 不改变 Claude、Codex、DeepSeek 的用量采集口径。

## 方案选择

采用方案 A：为需要键盘交互的详情建立明确可成为 Key Window 的 `NSPanel` 子类，并让 DeepSeek 的 `WKWebView` 成为窗口 First Responder。

没有采用只修改 `becomesKeyOnlyIfNeeded` 的方案，因为无边框面板是否能够成为 Key Window 仍取决于 `canBecomeKey`，单改属性不能形成完整契约。没有采用外部浏览器登录方案，因为外部浏览器会话不会自动共享给 AI Meter 的隔离 WebKit 数据存储。

## 窗口设计

### 交互式详情面板

在 `Sources/AIMeterApp/System` 新增专用的无边框面板类型：

- `canBecomeKey` 固定返回 `true`；
- `canBecomeMain` 固定返回 `false`，避免详情成为普通主窗口；
- 保持透明背景、floating level、跨 Space 和全屏辅助窗口行为；
- `becomesKeyOnlyIfNeeded` 对交互式详情设为 `false`。

右侧 84 × 300 悬浮条仍使用 `.nonactivatingPanel`，不改变其日常不抢焦点的行为。

### 提供商激活策略

- DeepSeek：激活 AI Meter，令详情面板成为 Key Window，并在 SwiftUI 宿主安装完成后把 First Responder 交给 `WKWebView`。用户随后点击网页输入框即可输入。
- Claude/Codex：继续使用 `orderFrontRegardless` 显示只读详情，不调用 App 激活，也不主动设置 First Responder。
- 切换离开 DeepSeek 或关闭详情：隐藏详情窗口并清理 First Responder，不能把已移除的 WebView 留在响应链中。

提供商与激活行为之间的映射放在可单测的纯策略中，AppKit 控制器只负责执行策略。

## 数据与隐私边界

- 手机号只输入到 AI Meter 内 `https://platform.deepseek.com` 官方页面。
- 验证码只从用户已授权查看的最新 DeepSeek iMessage/SMS 中读取，并只输入到同一官方页面。
- 手机号和验证码不写入源码、fixture、日志、开发文档、截图、缓存或 `UserDefaults`。
- `DeepSeekWebSession` 继续限制 HTTPS 官方主机；业务缓存仍只保存标准化的日期、成本、请求数和 Token 数。
- 登录 Cookie 继续由 App 自己的 WebKit 数据存储管理，不导入或导出其他浏览器 Cookie。

## 错误处理

- 如果面板仍不能成为 Key Window，详情保持可关闭，其他服务不受影响，并停止自动填写。
- 如果 DeepSeek 页面出现 CAPTCHA、法律协议或额外安全确认，按 Computer Use 确认政策停下交由用户处理。
- 如果验证码未到达、已过期或多条消息无法可靠区分，不猜测、不尝试旧码，向用户说明状态。
- 如果登录失败，不能把失败页面或验证码写入日志；保留官方页面供用户重试。

## 测试设计

### 自动化测试

1. DeepSeek 提供商映射为“需要激活、需要 Key Window、需要 WebView First Responder”。
2. Claude 与 Codex 映射为“只读展示、不激活、不设置网页 First Responder”。
3. 关闭或切换详情会清理交互焦点状态。
4. 现有外部点击、自动隐藏和登录暂停测试继续通过。
5. 完整核心测试保持 100 个现有测试全部通过，并增加本次焦点策略测试。

### 本机界面验收

1. 点击 DeepSeek 圆环，确认 AI Meter 成为活动应用且详情保持显示。
2. 点击手机号输入框，确认出现插入光标并可输入数字。
3. 点击“获取验证码”，确认请求只发送到 DeepSeek 官方页面。
4. 从“信息”读取最新 DeepSeek 验证码并填入，确认登录成功且不记录验证码。
5. 登录后确认最近 30 天统计能够同步或清楚显示官网/缓存状态。
6. 再次打开 Claude、Codex 详情，确认不会无故抢占当前应用焦点。
7. 确认详情自动隐藏、空白点击关闭、窗口切换和退出清理没有回归。

## 构建、安装与恢复

- 实现前保留 `/Applications/AI Meter.app` 的可恢复备份。
- 运行完整测试、release 构建、Info.plist 和严格签名验证。
- 安装版与构建版可执行文件校验和一致后再启动验收。
- 如果真实输入焦点仍失败，退出新版本并恢复备份，不在登录页面反复盲点或输入。

## 成功标准

- DeepSeek 官方手机号与验证码输入框能够稳定获得焦点并接受输入。
- 真实验证码登录成功，AI Meter 隔离会话随后可以加载用量页。
- 电话号码、验证码和原始登录数据没有进入仓库、日志或缓存。
- Claude/Codex 焦点行为和现有 详情关闭规则没有回归。

