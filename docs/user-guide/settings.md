# 设置参考

使用菜单栏齿轮或 `⌘,` 打开设置。

## Appearance

### Show floating meter

- 默认：开启。
- 作用：显示或隐藏贴边浮岛。
- 关闭后：菜单栏入口仍然可用。

### Screen edge

- **Automatic（默认）**：可用顶部短横拖动柄上下移动，也可横向拖到另一侧；松手后吸附最近边缘。
- **Left**：固定在屏幕左侧，仍可上下拖动。
- **Right**：固定在屏幕右侧，仍可上下拖动。
- 设置变更立即生效，不需要重启应用。
- AI Meter 会保存当前显示器、最后侧边和相对垂直位置；已保存显示器断开时回到主屏幕、Automatic、右侧和垂直中点，可见区域改变时自动夹紧。

三个服务圆环只负责打开详情。移动浮岛必须从顶部短横拖动柄开始，因此不会与服务点击互相冲突。

键盘或 VoiceOver 用户可聚焦顶部短横：上/下方向键按 10% 步进移动，左/右方向键会明确把侧边偏好设为 Left/Right；VoiceOver 也提供同名自定义动作并朗读当前侧边与垂直位置。

### Detail auto-hide

- 可选：3、5、8、15、30 秒。
- 默认：8 秒。
- 作用：点击某个圆环后，详情面板在无交互时自动收起。
- 例外：鼠标悬停、详情内键盘焦点、VoiceOver 运行和 DeepSeek 登录交互期间暂停倒计时；点击悬浮条和详情以外的区域会立即关闭。

## Monitoring

### Refresh interval

- 固定：5 分钟。
- 启动时会先刷新一次，也可从菜单栏手动刷新。

### Usage alerts at 70% and 90%

- 默认：关闭。
- 开启时 macOS 会请求通知权限。
- 只对有明确上限的额度比例生效，并抑制同一周期的重复通知。

### Open AI Meter at login

- 默认：关闭。
- 使用 macOS 登录项服务注册当前应用。
- 移动应用位置后应关闭再开启一次，以刷新路径。

## DeepSeek

### Balance baseline

- 默认：¥100。
- 最小有效值：¥1。
- 作用：决定 DeepSeek 圆环的参考起点，不会影响账户、充值或消费。

### DeepSeek API Key

- **Save**：去除首尾空白后写入 macOS Keychain，并立即刷新。
- **Remove**：从 Keychain 删除密钥并刷新状态。
- 设置页只显示是否已安全保存，不回显密钥内容。

## 本地持久化

| 内容 | 保存位置/机制 | 敏感性 |
| --- | --- | --- |
| DeepSeek API Key | macOS Keychain | 敏感，不进入普通偏好或缓存 |
| 外观、通知、基准、自动隐藏时间与浮岛位置 | `UserDefaults` | 非敏感 |
| 最近一次统一用量快照 | `Application Support/AI Meter` | 非敏感，写入前清理敏感文本 |
| DeepSeek 标准化每日用量 | `Application Support/AI Meter` | 非敏感聚合数据 |
| DeepSeek 官网登录会话 | App 隔离 WebKit 数据存储 | 敏感会话，由 WebKit 管理，不写入业务缓存 |
