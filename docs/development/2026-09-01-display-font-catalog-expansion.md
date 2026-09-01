# 显示字体目录扩充开发验收日志

**日期：** 2026-09-01  
**需求：** `REQ-20260901-007`  
**结果：** 已完成并安装验收

## 背景与范围

原有内容字体只有 System Default、Antonio、DIN Condensed。用户要求加入 Alimama FangYuanTi VF、Fira Code、Leigo、Menlo 和 Alimama DaoLiTi，同时继续保证 Settings 自身永远使用 macOS 系统字体。

本阶段采用已确认的“只使用本机已安装字体”方案：不下载、不安装、不打包、不在仓库重新分发字体文件。

## 实现

`DisplayFontChoice` 现在按固定顺序包含八项：

1. System Default；
2. Antonio；
3. DIN Condensed；
4. Alimama FangYuanTi VF；
5. Fira Code；
6. Leigo；
7. Menlo；
8. Alimama DaoLiTi。

保存值使用稳定 raw value，旧版 Antonio 与 DIN Condensed 偏好无需迁移。字体目录把可用性检测与最终解析统一到同一组候选：

- Fira Code：`Fira Code`、`Fira Code VF`；
- Leigo：`Leigo`、`Leigo Regular`；
- 其他字体使用同名家族。

缺失项在 Settings 中显示 `Not installed` 并禁用。已经保存但当前缺失的字体会安全回退系统字体，同时保留原偏好，重新安装后即可恢复。Fira Code、Leigo 或 Menlo 缺少中文字形时，由 macOS 字体级联补齐。

Settings 和 Widget 不读取内容字体来改变自身字形；浮动条、三个 Provider 详情与菜单内容面板继续通过统一语义字体环境即时应用用户选择。

## 自动化验证

覆盖以下回归：

- 八项顺序、名称、raw value 与偏好往返；
- Fira Code VF 与 Leigo Regular 别名；
- 首选候选优先级；
- 缺失字体禁用与安全回退；
- Settings 系统字体隔离；
- 内容字号偏移与语义字号保持；
- Widget 不读取内容字体偏好。

最终完整套件为 **281 项测试、57 个测试组、0 个失败**。源码与资源扫描确认没有新增 `.ttf`、`.otf`、`.woff` 或 `.woff2` 文件。

## 真实 Settings 验收

最终安装版 Appearance 的字体菜单显示全部八项。独立 AppKit 字体家族检测结果为：

| 字体/候选 | 当前机器状态 |
| --- | --- |
| Antonio | 未注册 |
| DIN Condensed | 已安装 |
| Alimama FangYuanTi VF | 未注册 |
| Fira Code / Fira Code VF | 未注册 |
| Leigo / Leigo Regular | 未注册 |
| Menlo | 已安装 |
| Alimama DaoLiTi | 未注册 |

当前偏好仍保持用户原有的 `antonio`；由于该家族在独立系统字体目录中未注册，内容界面会按设计安全回退到系统字体，偏好本身不被改写。详情自动隐藏时间恢复为 8 秒，浮动条保持右侧约 60%。未注册字体不能在本机完成真实字形对比，这属于安装环境限制，不影响字体目录、检测、禁用和回退功能完成。

## Release 与 Git 证据

- Release 签名、arm64 架构和安装哈希与 [Claude 专用详情日志](2026-09-01-claude-detail-local-activity.md) 使用同一最终候选版；
- `3595c28`：扩充显示字体目录、家族候选和测试；
- 最终用户文档、架构、安全说明和需求状态随本阶段收尾提交归档。

## 授权边界

AI Token Meter 不拥有或再授权这些第三方字体。用户应自行从合法来源安装；项目文档只记录家族名称与兼容行为，不提供字体文件或自动下载入口。
