# 2026-09-01 WidgetKit 桌面组件开发日志

## 背景与目标

用户确认在既有贴边浮岛之外增加原生 macOS Widget，并选择：

- 视觉方案 B：沿用黑蓝深海背景与三服务独立品牌渐变；
- 布局方案 A：额度优先、分层详情；
- Small 只显示三个统一大小的 Logo 状态环，不显示文字；
- Medium 显示三张 Provider 卡；
- Large 增加最近重置和 Codex 重置券摘要。

设计规格见[WidgetKit 扩展规格](../design/specifications/2026-09-01-widgetkit-extension-design.md)，测试驱动步骤见[实施计划](../design/implementation-plans/2026-09-01-widgetkit-extension.md)。

## 实现范围

1. `AIMeterCore/Widget` 新增版本化快照、统一展示构建器与原子存储；
2. 主应用在演示启动、真实刷新和 DeepSeek 基准变化后发布共享快照并请求 WidgetKit 更新时间线；
3. `AIMeterWidgetExtension` 新增独立 WidgetKit executable target、固定预览、过期降级和三种尺寸；
4. `aitokenmeter://open` 仅用于从 Widget 唤醒主应用；
5. 构建脚本新增可选 Widget 编译、Apple Development 身份检测、双方 App Group 注入、嵌套签名和验证脚本；
6. 普通无 Widget 构建保持可用，强制 Widget 且签名不足时在修改产物前失败。

打包自检还复现并修复了一个 macOS plist 工具边界：`plutil` 会把 entitlement 中含点号的完整键误作 key path，无法替换或提取 `com.apple.security.application-groups`。实现改用 `/usr/libexec/PlistBuddy` 的冒号路径，并用临时双模板验证真实替换、读取和沙箱布尔值，源码合同禁止回退到错误的 `plutil -extract/-replace` 写法。

## 隐私与安全决定

- Widget 不访问网络、CLI、Keychain 或 WebKit；
- 主应用写入共享容器前重用正式展示口径，并再次清理 Token、API Key、Cookie、邮箱和手机号；
- 共享数据不包含重置券兑换 ID，只包含数量和最近到期；
- 共享文件缺失、损坏或版本未知时显示三项 Unavailable；
- Widget 使用 App Sandbox，主应用与扩展必须使用相同 Apple Team ID 和 App Group。

## 测试驱动证据

每阶段先建立失败测试，再实现最小代码并保存 Git 节点：

| 节点 | 提交 | 结果 |
| --- | --- | --- |
| 设计规格 | `6e197fe` | 明确三尺寸、数据、刷新、隐私与签名边界 |
| 实施计划 | `4dc554c` | 形成测试驱动任务与验收门槛 |
| 共享快照 | `34f2f48` | Builder/Store/隐私回归通过 |
| 主应用发布 | `1ae2544` | 发布时机、失败隔离和深链元数据通过 |
| Widget 界面 | `8e747fd` | Small/Medium/Large、时间线和源码安全合同通过，真实 target 编译成功 |
| 条件打包 | `4cd2edd` | 无 Widget 构建、签名缺失保护、脚本与 entitlement 合同通过 |
| plist 键路径修复 | `7f69523` | 用 PlistBuddy 正确替换/读取带点号 entitlement 键，并锁定回归 |

最终自动化结果：

```text
224 tests in 48 suites passed
0 failures
4 environment-gated checks skipped as designed
```

另外完成：

- `AIMeterWidgetExtension` 独立 SwiftPM 编译成功；
- Widget 与主应用 plist/entitlement 模板通过 `plutil`；
- `AI_METER_INCLUDE_WIDGET=0` release 构建、ad-hoc 签名和严格验证通过；
- 无 Widget 包内确认不存在 `.appex` 与 `AIWidgetAppGroupIdentifier`；
- `AI_METER_INCLUDE_WIDGET=1` 在缺少开发签名时按设计以明确说明停止。

## 当前机器签名状态与实机验收

`security find-identity -v -p codesigning` 返回 `0 valid identities found`。因此本阶段没有伪造以下结果：

- `/Applications` 中已安装 Widget 版；
- Widget Gallery 已发现 AI Token Meter；
- Small/Medium/Large 的真实桌面截图；
- App Group 在正式开发签名下的进程间读写；
- WidgetKit 实际系统刷新延迟。

代码、测试、文档、普通 App release 构建和签名缺失保护均已完成。取得 Apple Development 证书后，只需运行强制 Widget 构建、Bundle 验证、安全替换安装，再按[测试指南](testing.md#手工界面验收)完成真实桌面验收。

## 已知系统行为

- 时间线以 30 分钟作为下一次建议刷新点，但 WidgetKit 会按系统预算安排，不能承诺准点或逐秒更新；
- 主应用刷新会调用 `reloadTimelines`，它是更新请求，不是强制即时渲染；
- Gallery 预览永远使用固定示例，不触及真实账户；
- Small 无可见文字，但仍提供包含 Provider 与数值的 VoiceOver 标签。
