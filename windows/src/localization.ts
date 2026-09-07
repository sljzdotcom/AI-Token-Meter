import { useSyncExternalStore } from "react"

export type Locale = "en" | "zh-CN"
const zh = {
  "Install CLI": "安装 CLI", "Waiting for Terminal…": "等待终端…",
  "Downloads and runs the official installer in Terminal.": "将在终端下载并运行官方安装器。",
  "Official installation instructions": "官方安装说明",
  "The installation guide could not be opened.": "无法打开安装说明。",
  "Use the official instructions for WSL or correct the custom CLI path, then choose Check Status.": "请按官方说明在 WSL 中安装，或修正自定义 CLI 路径，然后检查状态。",
  "Complete the official installation in Terminal. Status will update automatically.": "请在终端完成官方安装，状态将自动更新。",
  "Complete sign-in in Terminal. Status will update automatically.": "请在终端完成登录，状态将自动更新。",
  "CLI detected. You can now check the account or sign in.": "已检测到 CLI，可以检查账户或登录。",
  "Account connected.": "账户已连接。",
  "The operation is not confirmed. Finish in Terminal, then choose Check Status or retry.": "尚未确认操作完成。请在终端完成后检查状态或重试。",
  "The terminal could not be opened. Choose Check Status or retry.": "无法打开终端。请检查状态或重试。",
  "5h limit": "5 小时额度", "Usage limit": "用量额度", "Balance baseline": "余额基准", "{minutes}m limit": "{minutes} 分钟额度",
  "Windows Credential Manager": "Windows 凭据管理器", "Connected account": "已连接账户", "Configured provider": "已配置服务", "ChatGPT account": "ChatGPT 账户", "API Key account": "API 密钥账户", "OAuth account": "OAuth 账户", "Claude Code account": "Claude Code 账户",
  "Microsoft YaHei": "微软雅黑", "SimHei": "黑体", "KaiTi": "楷体",
  "Update service is temporarily unavailable": "更新服务暂不可用", "Update status is temporarily unavailable": "更新状态暂不可用",
  "Check for an update before installing": "安装前请先检查更新", "The update could not be verified": "无法验证更新",
  "The selected update is no longer available": "所选更新已不可用", "The available update changed; check again": "可用更新已改变，请重新检查",
  "Active usage checks did not stop; try the update again": "进行中的用量检查未停止，请重试更新", "The signed update could not be installed": "无法安装签名更新",
  "Another update operation is already running": "另一项更新操作正在进行",
  "Appearance": "外观", "Monitoring": "监测", "Services": "服务", "About": "关于",
  "Language": "语言", "Display language changes immediately in all windows.": "语言更改会立即应用到所有窗口。",
  "AI Token Meter Settings": "AI Token Meter 设置", "Settings categories": "设置分类",
  "Private AI usage, at a glance.": "私密的 AI 用量，一目了然。",
  "Floating strip size": "浮动条大小", "Compact saves space; Comfortable keeps larger rings.": "紧凑模式节省空间；舒适模式保留较大的用量环。",
  "Compact": "紧凑", "Comfortable": "舒适", "Fold when idle": "闲置时折叠",
  "Interaction, open details and refreshes keep the meter expanded.": "交互、详情打开或刷新时，浮动条保持展开。",
  "Never": "从不", "After 5 seconds": "5 秒后", "After 15 seconds": "15 秒后",
  "Floating strip services": "浮动条服务", "Keep at least one visible. Hidden services continue monitoring.": "至少显示一个服务。隐藏的服务仍会继续监测。",
  "Move {name} up": "上移 {name}", "Move {name} down": "下移 {name}", "Restore default order": "恢复默认顺序",
  "Display font": "显示字体", "Applies to the meter, menu and detail panels. Settings always uses the system font.": "应用于浮动条和详情面板。设置始终使用系统字体。",
  "Restore default font": "恢复默认字体", "System Default": "系统默认", "Not installed": "未安装", "Unavailable": "暂不可用",
  "Screen edge": "屏幕边缘", "The meter follows the selected display and stays outside the taskbar.": "浮动条跟随所选显示器，并避开任务栏。",
  "Right": "右侧", "Left": "左侧", "Display mode": "显示模式", "Primary display": "系统主显示器", "Selected display": "指定显示器", "All displays": "所有显示器",
  "Choose where the floating strip appears.": "选择浮动条显示的位置。", "Display": "显示器", "Primary": "主屏", "Offline display": "离线显示器",
  "An offline selection temporarily uses the primary display.": "所选显示器离线时，暂时回退到主屏。", "Move to primary display": "移到主显示器",
  "Refresh interval": "刷新间隔", "Scheduled refreshes never overlap; a manual refresh replaces an older task.": "定时刷新不会重叠；手动刷新会替换较早的任务。",
  "Refresh interval seconds": "刷新间隔（秒）", "seconds": "秒", "DeepSeek balance baseline": "DeepSeek 余额基准",
  "The DeepSeek ring shows the amount consumed from this reference balance.": "DeepSeek 用量环显示相对于基准余额的已用金额。",
  "Usage alerts": "用量提醒", "Notify once at 70% and again at 90%; dropping below 10% re-arms the alerts.": "在 70% 和 90% 时各提醒一次；降至 10% 以下后重新启用提醒。",
  "Usage alerts at 70% and 90%": "70% 和 90% 用量提醒", "Launch at login": "登录时启动",
  "Start the meter after you sign in to Windows.": "登录 Windows 后启动浮动条。", "Open AI Token Meter at login": "登录时打开 AI Token Meter",
  "Detail auto-hide": "详情自动隐藏", "Interaction pauses the countdown.": "交互时暂停倒计时。", "Detail auto-hide seconds": "详情自动隐藏（秒）",
  "Sign in again": "重新登录", "Sign in": "登录", "Sign in again to": "重新登录", "Sign in to": "登录",
  "Check {name} status": "检查 {name} 状态", "Check Status": "检查状态",
  "Windows opens a protected credential prompt; the Key never enters this WebView.": "Windows 会打开受保护的凭据提示框；密钥不会进入此网页视图。",
  "Replace DeepSeek API Key": "替换 DeepSeek API 密钥", "Check DeepSeek status": "检查 DeepSeek 状态", "Replace API Key": "替换 API 密钥",
  "Version": "版本", "Author · Miller": "作者 · Miller", "Checking…": "正在检查…", "Check for Updates": "检查更新", "Installing…": "正在安装…", "Update Now": "立即更新",
  "Automatic": "自动", "Native Windows": "Windows 原生", "Choose distribution": "选择发行版", "Optional custom CLI path": "可选的自定义 CLI 路径",
  "{name} runtime": "{name} 运行环境", "{name} WSL distribution": "{name} WSL 发行版", "{name} custom CLI path": "{name} 自定义 CLI 路径",
  "You’re up to date.": "当前已是最新版本。", "Version {version} is available.": "有新版本 {version} 可用。", "Downloading signed update…": "正在下载签名更新…",
  "Installing signed update…": "正在安装签名更新…", "Update check failed.": "更新检查失败。", "Updates are checked only when you ask.": "仅在您请求时检查更新。",
  "Connected": "已连接", "No API Key stored": "尚未保存 API 密钥", "Sign-in required": "需要登录", "CLI not installed": "未安装 CLI", "Checking account…": "正在检查账户…", "Account status unavailable": "账户状态暂不可用",
  "Account status is temporarily unavailable.": "账户状态暂不可用。", "The account status check did not complete.": "账户状态检查未完成。",
  "Floating strip settings could not be saved.": "无法保存浮动条设置。", "Launch at login could not be changed.": "无法更改登录时启动设置。",
  "Saving CLI runtime and refreshing this service…": "正在保存 CLI 运行环境并刷新服务…", "CLI runtime saved.": "CLI 运行环境已保存。", "The CLI runtime setting could not be saved.": "无法保存 CLI 运行环境设置。",
  "Complete sign-in in the new terminal window, then choose Check Status.": "请在新终端窗口中完成登录，然后选择“检查状态”。", "The sign-in window could not be opened.": "无法打开登录窗口。",
  "Open the protected Windows prompt to replace the API Key.": "请在受保护的 Windows 提示框中替换 API 密钥。", "DeepSeek accepted the replacement API Key.": "DeepSeek 已接受新的 API 密钥。",
  "The replacement was not saved. The existing API Key remains active.": "替换未保存，现有 API 密钥仍然有效。",
  "{name} details": "{name} 详情", "Official quota": "官方额度", "Reset credits": "重置次数", "Full usage reset": "完整用量重置", "Expiration": "到期时间", "{count} available": "可用 {count} 次",
  "Last {days} days · This PC": "最近 {days} 天 · 本机", "Tokens": "Token", "Sessions": "会话", "Active days": "活跃天数", "Last 30 days · Official website": "最近 30 天 · 官方网站",
  "Updated": "更新于", "Cached · {minutes} min ago": "缓存 · {minutes} 分钟前", "Fresh": "最新", "Refreshing": "正在刷新", "Needs sign-in": "需要登录", "Needs setup": "需要设置",
  "Resets {date}": "重置时间 {date}", "Official value": "官方数据", "Official balance · API usage": "官方余额 · API 用量", "Official quota · Local OpenAI Codex activity": "官方额度 · 本机 OpenAI Codex 活动", "Official quota · Local Claude Code activity": "官方额度 · 本机 Claude Code 活动",
  "Available": "可用", "Cached": "缓存", "Sign in required": "需要登录", "Setup required": "需要设置", "Format changed": "格式已更改", "recently": "刚刚",
  "Cost": "费用", "Requests": "请求", "DeepSeek cost for the last 30 days": "DeepSeek 最近 30 天费用", "Official website · Updated": "官方网站 · 更新于",
  "Official history status is temporarily unavailable.": "官方历史状态暂不可用。", "Opening official page…": "正在打开官方页面…", "Sync in progress": "正在同步",
  "Official history sync could not be started. Try again.": "无法启动官方历史同步，请重试。", "Usage history will appear here after official-page sync.": "与官方页面同步后，用量历史会显示在此处。", "Try again": "重试", "Sync official history": "同步官方历史",
  "Expand floating meter": "展开浮动条", "AI usage providers": "AI 用量服务", "{name} usage": "{name} 用量", "Action required": "需要操作",
  "Session": "当前会话", "Weekly limit": "每周额度", "Available balance": "可用余额", "Daily limit": "每日额度", "Session limit": "会话额度", "Weekly": "每周", "Balance": "余额",
  "Settings": "设置", "Refresh": "刷新", "Close": "关闭",
} as const

export const dictionaries = { en: Object.fromEntries(Object.keys(zh).map(key => [key, key])) as Record<string, string>, "zh-CN": zh as Record<string, string> }
let currentLocale: Locale = "en"
const listeners = new Set<() => void>()
export function setLocale(value: unknown) {
  const next = value === "zh-CN" ? "zh-CN" : "en"
  if (next === currentLocale) return
  currentLocale = next
  if (typeof document !== "undefined") document.documentElement.lang = next
  listeners.forEach(listener => listener())
}
export function getLocale() { return currentLocale }
export function useLocale() { return useSyncExternalStore(callback => { listeners.add(callback); return () => listeners.delete(callback) }, getLocale, () => "en" as Locale) }
export function t(key: string, values: Record<string, string | number> = {}) {
  const duration = key.match(/^(\d+)m limit$/)
  if (currentLocale === "zh-CN" && duration) return t("{minutes}m limit", {minutes: duration[1]})
  const text = dictionaries[currentLocale][key] ?? key
  return text.replace(/\{(\w+)\}/g, (placeholder, name: string) => String(values[name] ?? placeholder))
}
