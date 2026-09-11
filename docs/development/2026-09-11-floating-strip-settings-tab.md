# Settings悬浮条独立页签开发日志

关联需求：REQ-20260911-009。

## 目标与范围

Appearance原先同时包含字体、悬浮条服务、尺寸、显示器、贴边和收起行为，页面过长且职责混杂。本次在macOS与Windows新增第五个Floating Strip页签；Windows简体中文显示“悬浮条”。只移动现有设置，不改变偏好字段、默认值、即时保存、服务排序、多屏回退或自动收起语义。

## 最终结构

- Appearance：macOS保留Display font；Windows保留Language和Display font。
- Floating Strip：按Content and Size、Screen and Position、Behavior三组排列服务显示/顺序、三档尺寸、显示器/贴边、自动收起、显示/收起延迟和详情自动隐藏；macOS同时保留Show floating meter。
- Monitoring：保留刷新间隔、DeepSeek余额基准、用量提醒和登录时启动。
- Services与About保持原职责。
- 悬浮条右键菜单的Settings直接进入Floating Strip；详情恢复操作继续进入Services。

## 验证边界

macOS由SwiftUI/AppKit编译、结构测试和完整回归覆盖。Windows由React交互测试、TypeScript正式构建、Rust宿主页签白名单及真实Chrome页面布局覆盖；当前macOS开发环境不能替代物理Windows 11上的WebView2、DPI和多显示器人工观察，该现场边界继续保留，不把浏览器验证表述为Windows原生实机验收。

## 证据

- 失败先行：macOS四页签实现无法编译新增`.floatingStrip`断言；Windows缺少Floating Strip/悬浮条页签的4项断言失败。
- macOS定向SettingsStructureTests及完整93个测试套件、496项测试通过；本机未安装的CLI、钥匙串与显示器条件项按既有规则跳过。
- Windows 18个测试文件、136项前端测试和production build通过。
- Windows真实Chrome密度门禁通过25项进程生命周期、8项Antigravity详情、24项四Provider浮动条、2项收起态与608个文字角色。
- Windows宿主完整246项Rust测试、格式检查与严格Clippy通过。
- 6份跨平台合同与可移植性回归、275份Markdown文档、公开源码安全检查通过。
- 独立复核最初发现2项Important：连续重复的同页签外部请求可能不触发重选，以及Windows窄窗第五个页签不可达。两项均以失败回归锁定后修复；最终复核Critical/Important/Minor为0/0/0。
- 实现与验证提交`2d7d276`已快进整合到本机`main`；远端推送被自动审批拒绝，未以其他方式上传。

本轮没有新版本发布授权，不修改版本号、标签或更新源。
