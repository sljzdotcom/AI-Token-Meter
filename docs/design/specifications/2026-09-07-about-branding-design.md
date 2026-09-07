# 关于页社交链接与 Windows 品牌头部

关联 REQ-20260907-008/009。用户明确授权直接设计实施，不再等待常规选择；本轮不发布版本。

## 安装体积核对

Swift CLIInstallationScriptBuilder 和 Windows accounts/installation.rs 只生成固定官方 HTTPS 下载脚本。用户点击后，完整下载安装脚本，再由官方脚本获取 CLI；Package.swift 与 Tauri bundle 没有内嵌第三方 CLI。本应用不新增 Node/Homebrew/WSL 依赖包。无需改为在线下载，因为已经如此；不声称精确安装体积差值。

## 视觉选择

推荐紧凑图标链接：Miller 作者行后放 Twitter/X 图标与 @MillerPanYue、GitHub 图标与 GitHub。相较纯文字更易辨识，相较大型社交卡片更节省空间；不增加头像、远程图片或外部图标库。保留系统字体、现有配色、轻量边框和键盘焦点，宽度不足自动换行。

## 精确内容与行为

- Twitter：@MillerPanYue，目标 https://twitter.com/MillerPanYue ，辅助说明 Twitter / X。
- GitHub：GitHub，目标 https://github.com/sljzdotcom/AI-Token-Meter ，辅助说明项目仓库。
- 点击才用系统默认浏览器打开，不在应用 WebView 导航，不自动请求网站，不追踪。
- Windows 标题左侧使用现有 windows/src-tauri/icons/128x128.png 本地软件图标，显示 40px，保持比例；与标题/副标题垂直居中。不改 macOS 既有软件图标。
- 社交图标 14–16px，含文本链接具有完整可访问名称，图标为装饰性；失败显示简短可恢复提示，不泄漏底层错误。
- macOS 使用 SwiftUI 与系统 openURL/NSWorkspace 边界；Windows 后端仅接受固定枚举目标，不接受任意 URL/命令，由前端链接回调打开系统浏览器。

## 验收与边界

覆盖两个链接的准确目标、点击行为、失败反馈、只渲染不导航、中英文名称和本地图标。保留 Settings 系统字体和更新操作。通过双平台构建及回归检查，文档同步；真实 Windows 物理 DPI 与用户浏览器会话不以宿主测试冒充。
