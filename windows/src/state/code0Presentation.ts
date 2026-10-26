import { getLocale, t } from "../localization"
import type { Code0Action, Code0ActionResult, Code0SessionDTO, Code0SessionFailure, Code0SessionState } from "./code0Session"

// Code0-only wording reuses the existing locale store; it introduces no language
// preference, persistence or subscription system of its own.
const zh = {
  "Connect Code0": "连接 Code0",
  "I've finished signing in": "我已完成登录",
  "Sync Code0": "同步 Code0",
  "Disconnect Code0": "断开 Code0",
  "Code0 is not connected.": "Code0 尚未连接。",
  "Finish signing in in the Code0 window, then confirm below.": "请在 Code0 窗口完成登录，然后在下方确认。",
  "Checking Code0 website usage…": "正在检查 Code0 网站用量…",
  "Closing the Code0 window…": "正在关闭 Code0 窗口…",
  "Code0 automatic sync is enabled for this run.": "Code0 自动同步已在本次运行中启用。",
  "Code0 automatic sync is paused.": "Code0 自动同步已暂停。",
  "Showing cached Code0 data. Choose Sync Code0 to validate this connection.": "正在显示 Code0 缓存数据，请选择同步 Code0 以验证此连接。",
  "Code0 is paused. Connect again or disconnect this connection.": "Code0 已暂停。请重新连接或断开此连接。",
  "Code0 cleanup is not confirmed. New connections are blocked.": "尚未确认 Code0 清理完成，无法建立新连接。",
  "Code0 validation is required. Connect or sync explicitly.": "Code0 需要验证，请明确选择连接或同步。",
  "Code0 connection is unavailable or already in use. Close any other Code0 session, then try again.": "Code0 连接不可用或已被占用。请关闭其他 Code0 会话后重试。",
  "Code0 storage could not be confirmed. Automatic sync is paused.": "无法确认 Code0 存储状态，自动同步已暂停。",
  "The Code0 page requested an unsupported action. Sync is paused.": "Code0 页面请求了不支持的操作，同步已暂停。",
  "Code0 sync did not complete. Automatic sync is paused.": "Code0 同步未完成，自动同步已暂停。",
  "Code0 authentication changed. Connect again to validate the current account.": "Code0 登录状态已更改，请重新连接以验证当前账户。",
  "Code0 sign-in timed out. Connect again.": "Code0 登录已超时，请重新连接。",
  "Code0 sync timed out. Previous data and success time are unchanged.": "Code0 同步超时，原有数据和成功时间保持不变。",
  "Code0 can sync again in {seconds} seconds.": "再过 {seconds} 秒即可同步 Code0。",
  "Code0 is busy. Wait for the current operation to finish.": "Code0 正忙，请等待当前操作完成。",
  "The Code0 action could not start.": "无法开始 Code0 操作。",
  "Last successful sync:": "上次成功同步：",
  "Sync attempts are at least five minutes apart. A failed attempt pauses automatic sync.": "两次同步尝试至少间隔五分钟；尝试失败后会暂停自动同步。",
  "Website balance and cumulative site credits use this dedicated Code0 connection.": "此专用 Code0 连接用于显示网站余额与累计已用站内额度。",
  "Connect Code0 in Services to view website usage.": "请在“服务”中连接 Code0，以查看网站用量。",
} as const

type TextKey = keyof typeof zh
const actionKeys: Record<Code0Action, TextKey> = {
  connect: "Connect Code0", loginCompleted: "I've finished signing in", syncNow: "Sync Code0", disconnect: "Disconnect Code0",
}
const failureKeys: Record<Code0SessionFailure, TextKey> = {
  validationRequired: "Code0 validation is required. Connect or sync explicitly.",
  profileBusy: "Code0 connection is unavailable or already in use. Close any other Code0 session, then try again.",
  storageFailed: "Code0 storage could not be confirmed. Automatic sync is paused.",
  cleanupFailed: "Code0 cleanup is not confirmed. New connections are blocked.",
  policyRejected: "The Code0 page requested an unsupported action. Sync is paused.",
  navigationFailed: "Code0 sync did not complete. Automatic sync is paused.",
  invalidResponse: "Code0 sync did not complete. Automatic sync is paused.",
  authenticationChanged: "Code0 authentication changed. Connect again to validate the current account.",
  loginTimedOut: "Code0 sign-in timed out. Connect again.",
  syncTimedOut: "Code0 sync timed out. Previous data and success time are unchanged.",
}
const stateKeys: Record<Code0SessionState, TextKey> = {
  disconnected: "Code0 is not connected.", connecting: "Finish signing in in the Code0 window, then confirm below.",
  syncing: "Checking Code0 website usage…", closing: "Closing the Code0 window…",
  active: "Code0 automatic sync is enabled for this run.",
  paused: "Showing cached Code0 data. Choose Sync Code0 to validate this connection.",
  cleanupPending: "Code0 cleanup is not confirmed. New connections are blocked.",
}

export function code0Text(key: TextKey, values: Record<string, number> = {}) {
  const text = getLocale() === "zh-CN" ? zh[key] : key
  return text.replace(/\{(\w+)\}/g, (placeholder, name: string) => String(values[name] ?? placeholder))
}
export function code0ActionLabel(action: Code0Action) { return code0Text(actionKeys[action]) }
export function code0SessionMessage(session: Code0SessionDTO) {
  if (session.state === "cleanupPending") return code0Text(stateKeys.cleanupPending)
  if (session.failure) return code0Text(failureKeys[session.failure])
  if (session.state === "paused" && !session.snapshot) return code0Text("Code0 is paused. Connect again or disconnect this connection.")
  if (session.state === "active" && !session.isAutomaticallySyncing) return code0Text("Code0 automatic sync is paused.")
  return code0Text(stateKeys[session.state])
}
export function code0WaitMessage(seconds: number | null) {
  return seconds !== null && Number.isSafeInteger(seconds) && seconds > 0 && seconds <= 300
    ? code0Text("Code0 can sync again in {seconds} seconds.", {seconds}) : null
}
export function code0ActionMessage(result: Code0ActionResult | null) {
  if (!result || result.kind === "started") return null
  if (result.kind === "busy") return code0Text("Code0 is busy. Wait for the current operation to finish.")
  if (result.kind === "throttled") return code0WaitMessage(result.secondsRemaining) ?? code0Text("The Code0 action could not start.")
  return code0Text("The Code0 action could not start.")
}
export function formatCode0Credits(value: number) {
  if (!Number.isFinite(value) || value < 0) return "—"
  const amount = new Intl.NumberFormat(getLocale(), {minimumFractionDigits: 2, maximumFractionDigits: 2}).format(value)
  return t("{amount} site credits", {amount})
}
export function formatCode0Time(value: string) {
  const date = new Date(value)
  return Number.isFinite(date.getTime())
    ? new Intl.DateTimeFormat(getLocale(), {dateStyle: "medium", timeStyle: "short"}).format(date) : "—"
}
