import { act, fireEvent, render, screen } from "@testing-library/react"
import { expect, it, vi } from "vitest"
import { setLocale, type Locale } from "../localization"
import type { Code0Action, Code0ActionResult, Code0SessionState } from "../state/code0Session"
import { code0Session, deferred } from "../test/code0SessionFixtures"
import { SettingsCode0 } from "./SettingsCode0"

const locales: Locale[] = ["en", "zh-CN"]
const actions: Array<{action: Code0Action; state: Code0SessionState; hasSnapshot: boolean; labels: Record<Locale, string>}> = [
  {action: "connect", state: "disconnected", hasSnapshot: false, labels: {en: "Connect Code0", "zh-CN": "连接 Code0"}},
  {action: "loginCompleted", state: "connecting", hasSnapshot: false, labels: {en: "I've finished signing in", "zh-CN": "我已完成登录"}},
  {action: "syncNow", state: "paused", hasSnapshot: true, labels: {en: "Sync Code0", "zh-CN": "同步 Code0"}},
  {action: "disconnect", state: "paused", hasSnapshot: true, labels: {en: "Disconnect Code0", "zh-CN": "断开 Code0"}},
]

it.each(locales.flatMap(locale => actions.map(value => ({...value, locale}))))("only an explicit $action click authorizes that one operation in $locale", ({action, state, hasSnapshot, labels, locale}) => {
  setLocale(locale)
  const onAction = vi.fn(async (): Promise<Code0ActionResult> => ({kind: "started"}))
  const current = code0Session({state, snapshot: hasSnapshot ? code0Session().snapshot : null})
  render(<SettingsCode0 session={current} onAction={onAction} />)
  expect(onAction).not.toHaveBeenCalled()
  fireEvent.click(screen.getByRole("button", {name: labels[locale]}))
  expect(onAction).toHaveBeenCalledOnce()
  expect(onAction).toHaveBeenCalledWith(action)
})

it.each(locales.flatMap(locale => (["loginTimedOut", "authenticationChanged"] as const).map(failure => ({locale, failure}))))("paused nil snapshot retains Disconnect after $failure in $locale", ({locale, failure}) => {
  setLocale(locale)
  const onAction = vi.fn(async (): Promise<Code0ActionResult> => ({kind: "started"}))
  render(<SettingsCode0 session={code0Session({state: "paused", failure, snapshot: null})} onAction={onAction} />)
  expect(onAction).not.toHaveBeenCalled()
  const label = locale === "en" ? "Disconnect Code0" : "断开 Code0"
  expect(screen.getAllByRole("button", {name: label})).toHaveLength(1)
  fireEvent.click(screen.getByRole("button", {name: label}))
  expect(onAction.mock.calls).toEqual([["disconnect"]])
})

it.each(locales)("mount, focus, locale and state rerender never authorize Code0 in %s", locale => {
  setLocale(locale)
  const onAction = vi.fn(async (): Promise<Code0ActionResult> => ({kind: "denied"}))
  const {container, rerender} = render(<SettingsCode0 session={code0Session()} onAction={onAction} />)
  fireEvent.focus(container.querySelector("section")!)
  act(() => setLocale(locale === "en" ? "zh-CN" : "en"))
  rerender(<SettingsCode0 session={code0Session({revision: 2, state: "active", isAutomaticallySyncing: true})} onAction={onAction} />)
  expect(onAction).not.toHaveBeenCalled()
})

it.each(["closing", "cleanupPending"] as const)("unconfirmed %s cannot create another connection", state => {
  const onAction = vi.fn(async (): Promise<Code0ActionResult> => ({kind: "denied"}))
  render(<SettingsCode0 session={code0Session({state})} onAction={onAction} />)
  const connect = screen.getByRole("button", {name: "Connect Code0"})
  expect(connect).toBeDisabled()
  fireEvent.click(connect)
  expect(onAction).not.toHaveBeenCalled()
})

it("sync in progress disables duplicate Sync while retaining explicit Disconnect", () => {
  const onAction = vi.fn(async (): Promise<Code0ActionResult> => ({kind: "busy"}))
  render(<SettingsCode0 session={code0Session({state: "syncing"})} onAction={onAction} />)
  expect(screen.getByRole("button", {name: "Sync Code0"})).toBeDisabled()
  expect(screen.getByRole("button", {name: "Disconnect Code0"})).toBeEnabled()
  expect(onAction).not.toHaveBeenCalled()
})

it.each(locales)("300-second throttle is presentation only, never an automatic retry in %s", locale => {
  setLocale(locale)
  const onAction = vi.fn(async (): Promise<Code0ActionResult> => ({kind: "denied"}))
  const label = locale === "en" ? "Sync Code0" : "同步 Code0"
  const {rerender} = render(<SettingsCode0 session={code0Session({retryAfterSeconds: 300})} onAction={onAction} />)
  expect(screen.getByRole("button", {name: label})).toBeDisabled()
  expect(screen.getByText(locale === "en" ? "Code0 can sync again in 300 seconds." : "再过 300 秒即可同步 Code0。")).toBeVisible()
  rerender(<SettingsCode0 session={code0Session({revision: 2, retryAfterSeconds: 299})} onAction={onAction} />)
  expect(screen.getByRole("button", {name: label})).toBeDisabled()
  rerender(<SettingsCode0 session={code0Session({revision: 3, retryAfterSeconds: null})} onAction={onAction} />)
  expect(screen.getByRole("button", {name: label})).toBeEnabled()
  expect(onAction).not.toHaveBeenCalled()
})

it("a pending explicit response cannot submit twice and a busy result does not queue a retry", async () => {
  const response = deferred<Code0ActionResult>()
  const onAction = vi.fn((_action: Code0Action) => response.promise)
  render(<SettingsCode0 session={code0Session()} onAction={onAction} />)
  try {
    const sync = screen.getByRole("button", {name: "Sync Code0"})
    fireEvent.click(sync)
    expect(sync).toBeDisabled()
    fireEvent.click(sync)
    expect(onAction.mock.calls).toEqual([["syncNow"]])
    await act(async () => { response.resolve({kind: "busy"}); await response.promise })
    expect(screen.getByText("Code0 is busy. Wait for the current operation to finish.")).toBeVisible()
    expect(onAction.mock.calls).toEqual([["syncNow"]])
  } finally {
    await act(async () => { response.resolve({kind: "busy"}); await response.promise })
  }
})

it.each(locales)("fixed storage failure redraws while retaining the successful time in %s", locale => {
  setLocale(locale)
  const onAction = vi.fn(async (): Promise<Code0ActionResult> => ({kind: "denied"}))
  const current = code0Session()
  const {container, rerender} = render(<SettingsCode0 session={current} onAction={onAction} />)
  rerender(<SettingsCode0 session={{...current, revision: 2, failure: "storageFailed"}} onAction={onAction} />)
  expect(screen.getByText(locale === "en" ? "Code0 storage could not be confirmed. Automatic sync is paused." : "无法确认 Code0 存储状态，自动同步已暂停。")).toBeVisible()
  expect(container.querySelector("time")).toHaveAttribute("datetime", current.snapshot!.fetchedAt)
  expect(onAction).not.toHaveBeenCalled()
})
