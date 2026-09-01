# Settings 分类与品牌迁移开发日志

**日期：** 2026-09-01
**分支：** `codex/settings-tabs-brand-migration`
**状态：** 功能、文档、Release 构建和安装验收完成；主分支合并在本文后续更新
**规格：** [`docs/superpowers/specs/2026-09-01-settings-tabs-and-brand-migration-design.md`](../superpowers/specs/2026-09-01-settings-tabs-and-brand-migration-design.md)
**计划：** [`docs/superpowers/plans/2026-09-01-settings-tabs-and-brand-migration.md`](../superpowers/plans/2026-09-01-settings-tabs-and-brand-migration.md)

## 目标

1. 把持续增长的 Settings 分成清晰的顶部 Tab；
2. 将可见名称改为 **AI Token Meter**，副标题改为 **Private AI usage monitor**；
3. 保留 Bundle、Keychain、偏好、缓存和 Claude 工作区的兼容身份；
4. 保留已经确认的无文字仪表指针 App Icon；
5. 为后续独立 WidgetKit 项目建立稳定品牌与共享展示边界。

## 设计结果

Settings 固定为四个顶部 Tab：

| Tab | 职责 |
| --- | --- |
| Appearance | 浮岛显示、侧边、字体、详情自动隐藏 |
| Monitoring | 刷新、阈值通知、登录时启动 |
| Services | Claude 一次性工作区设置、DeepSeek 余额基准与 API Key |
| About | App Icon、产品名称、副标题、版本与项目/隐私入口 |

`SettingsMessageKind` 让异步反馈回到所属 Tab：Claude 与 DeepSeek 归 Services，登录项归 Monitoring。Settings 根视图继续使用 `.settings` 字体作用域，因此不受 Antonio、DIN Condensed 或内容字号偏好影响。

## 品牌与兼容边界

可见名称和构建产物迁移到 AI Token Meter，但以下身份有意保持不变：

- Bundle Identifier：`com.millerpan.AIMeter`；
- 可执行文件：`AIMeterApp`；
- Keychain 服务和账户标识；
- `UserDefaults` suite；
- `Application Support/AI Meter` 缓存目录；
- `Application Support/AI Meter/ClaudeUsageWorkspace` 批准目录。

这样升级不需要复制或重新保存敏感数据，也不会让现有 Claude 工作区批准失效。

## 测试驱动实现

### Settings 分类

先增加 Settings 信息架构测试，确认四个 Tab 的顺序和反馈路由在实现前失败；随后拆出四个职责视图并让 `SettingsView` 成为轻量 `TabView` 根。提交：

- `e5f8e94 feat: organize settings into four tabs`

### 品牌迁移

先增加 `AppBrandTests`、真实 `Info.plist` 元数据测试和菜单栏无数据文案回归；失败证据分别证明集中品牌模型、Bundle 显示名和旧辅助功能文案尚未迁移。随后增加 `AppBrand`，更新界面、Bundle 和构建脚本，并保留兼容身份。提交：

- `1f7f6f4 feat: rename the product to AI Token Meter`

## 自动化验证

功能完成后执行：

```bash
bash scripts/test.sh
```

结果：196 个测试、41 个测试组、0 失败。3 个依赖真实本机账户或 Keychain 授权的环境门控检查按设计跳过。输出中的 WebKit 缓存目录权限提示来自测试沙盒，不影响测试结果。

新增回归覆盖：

- 四个 Settings Tab 的稳定顺序；
- Claude、DeepSeek 和登录项反馈路由；
- 可见品牌名称、副标题和版本文案；
- 真实 `Info.plist` 中的新显示名；
- 旧 Bundle Identifier、可执行文件名和 App Icon 声明不变；
- 菜单栏有额度与无额度两种辅助功能文案。

## Release 与本机验收

执行 `bash scripts/build-app.sh` 成功生成：

```text
dist/AI Token Meter.app
```

候选包验证结果：

- `Info.plist` 通过 `plutil -lint`；
- 显示名和 Bundle 名均为 `AI Token Meter`；
- Bundle Identifier 为 `com.millerpan.AIMeter`；
- 可执行文件为 `AIMeterApp`；
- `CFBundleIconFile` 仍为 `AppIcon`；
- 严格代码签名验证通过；
- 主可执行文件为 arm64 Mach-O；
- SHA-256：`77df9719b42db425b925d3d9e878e2b7539aa4da6c6a0bdc553a01793bd5b064`。

安装前旧应用被可恢复地移动到：

```text
/private/tmp/AI-Token-Meter-brand-migration-20260901-1145/AI Meter.app
```

候选包安装到 `/Applications/AI Token Meter.app`。安装版再次通过 plist、严格签名和 arm64 架构检查；构建版与安装版主可执行文件 SHA-256 完全一致，`cmp` 退出码为 0。旧 `/Applications/AI Meter.app` 不再存在，因此“应用程序”目录只保留新显示名称。

真实运行验收：

| 项目 | 结果 |
| --- | --- |
| 应用菜单 | 菜单名、About、Hide、Quit 均显示 AI Token Meter；Settings… 可稳定打开 |
| Appearance | Right、Antonio、8 seconds 与升级前偏好保持；设置页面使用系统字体 |
| Monitoring | 5 分钟刷新、70%/90% 提醒和 Open AI Token Meter at login 正常显示 |
| Services | Claude、Codex 认证说明和 DeepSeek 配置正确归类；既有 Keychain 状态显示 `Stored securely in Keychain` |
| About | 显示 AI Token Meter、Private AI usage monitor、Version 0.1.0 (1) 和隐私说明 |
| 浮岛 | 右侧 98% 位置恢复；Claude、Codex、DeepSeek 三项状态正常呈现 |

## 安全与隐私检查

- 没有新增网络端点、遥测或凭证读取；
- 没有移动、复制或重新保存 Keychain 项目；
- 旧数据目录只作为兼容路径继续读取和写入；
- 文档不包含 API Key、Cookie、OAuth Token 或未脱敏账户响应。

## 后续工作

- WidgetKit 是单独阶段，不与本次品牌迁移混合；
- 桌面层仍需补验 Mission Control、左右普通 Space、真实指针拖动和可用时的多显示器场景。
