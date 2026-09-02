# 2026-09-01 真实桌面验收记录

## 范围与结论

本轮按用户要求暂停 Widget 证书与 Gallery 验收，只验收当前安装在 `/Applications/AI Token Meter.app` 的无 Widget 版本。验收使用真实 macOS App、真实 Edge 窗口和系统辅助功能交互完成，不以 SwiftUI Preview 或浏览器原型代替。

当前单桌面、单可操作显示环境内的核心桌面行为通过：浮岛只在桌面层显示，普通最大化与全屏 Edge 都不会被覆盖；三服务入口、详情自动关闭、设置入口、四个设置分类、左右贴边、纵向移动和状态恢复均正常。

Mission Control、第二个普通桌面 Space、真实鼠标指针拖动和多显示器仍受当前自动化/硬件环境限制，保留为环境补验项，不把未取得直接证据的项目写成通过。

## 安装身份

- 安装路径：`/Applications/AI Token Meter.app`
- Bundle Identifier：`com.millerpan.AIMeter`
- 显示名称：`AI Token Meter`
- 版本：`0.1.0`（build `1`）
- 签名：`codesign --verify --deep --strict` 通过
- 当前安装包不包含 `.appex`；因此本轮不涉及 Widget Gallery、App Group 或 WidgetKit 刷新
- 验收代码基线：`main` 的 Widget 合并节点 `da4dc5c`

## 真实操作矩阵

| 项目 | 结果 | 真实证据 |
| --- | --- | --- |
| 浮岛本体 | 通过 | 安装版显示 Claude、Codex、DeepSeek 三个 Logo、圆环与连续深海背景；无多余文字。 |
| 三个服务入口 | 通过 | 三个按钮都暴露独立语义；Claude 实际点击从 `Detail closed` 变为 `Detail open`。 |
| 详情自动隐藏 | 通过 | 当前偏好为 8 秒；Claude 详情打开后等待 9 秒，状态自动恢复为 `Detail closed`。 |
| 设置菜单 | 通过 | 应用菜单中的 `Settings…` 可点击并成功打开设置窗口。 |
| Settings 分类 | 通过 | Appearance、Monitoring、Services、About 四页依次打开；About 显示 `AI Token Meter / Private AI usage monitor`。 |
| 字体与设置隔离 | 通过 | Appearance 显示 `Display font = Antonio`；Settings 本身保持系统字体。 |
| 右侧吸附 | 通过 | 初始状态为 `Right edge, vertical position 98 percent`，右侧 S 曲线与背景裁切正常。 |
| 左侧镜像 | 通过 | 实际切到 Left 后，轮廓和深海背景正确镜像，Logo 与圆环方向不反转；随后恢复 Right。 |
| 纵向移动 | 通过（辅助功能路径） | 实际操作将位置从 98% 移到 88%，再恢复 98%；偏好同步保存。 |
| 普通最大化 Edge | 通过 | Edge 前台截图中没有浮岛或详情覆盖。 |
| Edge 全屏 | 通过 | 实际点击绿色全屏按钮进入独立全屏 Space，截图中没有浮岛或详情；验收后退出全屏。 |
| Space 切换后的详情状态 | 通过 | 进入全屏前后的详情最终均为 closed，未残留覆盖面板。 |
| Mission Control | 自动化环境未覆盖 | Mission Control 与 Dock 系统进程均无法被当前 Computer Use 接口稳定捕获，两次调用分别超时；未继续重复触发。 |
| 两个普通桌面 Space | 当前系统条件未覆盖 | `com.apple.spaces` 只登记一个普通 Space；本轮不新增或删除系统 Space。 |
| 真实指针拖动 | 自动化接口未覆盖 | 非激活 `NSPanel` 坐标拖动返回 `noWindowsAvailable`；已通过可访问性移动与项目自动化拖动测试覆盖逻辑，但不冒充鼠标实测。 |
| 多显示器 | 当前硬件/会话未覆盖 | 当前自动化会话没有暴露第二块可操作显示器；未进行跨屏、插拔和主屏切换。 |

## Widget 证书延期记录

用户于 2026-09-01 明确决定：Widget 证书先放一放。WidgetKit 源码、三尺寸布局、共享快照、条件签名和 227 项回归测试保留在 `main`，但以下项目暂停，待以后取得 Apple Development 证书再继续：

1. 强制 Widget 构建与嵌套签名；
2. 安装包含 `.appex` 的正式开发包；
3. Widget Gallery 发现；
4. Small、Medium、Large 真实桌面验收；
5. App Group 跨进程读写、系统刷新延迟和点击唤醒实测。

延期不影响当前无 Widget 主应用的桌面使用。

## 验收后现场恢复

- 显示字体：Antonio
- 吸附侧边：Right
- 垂直位置：约 98%
- 浮岛：开启
- 详情自动隐藏：8 秒
- 详情窗口：全部关闭
- Edge：已退出全屏，恢复普通窗口
- 未修改登录状态、API Key、Keychain、系统 Space 或显示器设置

## 最终判断

本机当前可提供的真实桌面验收已完成，结论为 `PASS_WITH_ENVIRONMENT_GAPS`。未发现影响当前单桌面使用的功能错误；剩余项都需要当前环境没有提供的系统界面控制或额外硬件，已明确登记而没有伪造通过结果。
