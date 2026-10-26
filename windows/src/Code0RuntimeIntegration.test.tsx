import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react"
import { afterEach, beforeEach, expect, it, vi } from "vitest"

const native = vi.hoisted(() => ({
  invoke: vi.fn<(command: string, args?: Record<string, unknown>) => Promise<unknown>>(),
  handlers: new Map<string, Set<(event: {payload: unknown}) => void>>(),
}))
vi.mock("@tauri-apps/api/core", () => ({invoke: native.invoke}))
vi.mock("@tauri-apps/api/event", () => ({
  listen: async (name: string, handler: (event: {payload: unknown}) => void) => {
    const handlers = native.handlers.get(name) ?? new Set()
    handlers.add(handler)
    native.handlers.set(name, handlers)
    return () => { handlers.delete(handler) }
  },
}))
vi.mock("@tauri-apps/api/window", () => ({getCurrentWindow: () => ({label: "detail", startDragging: async () => {}})}))

import { App } from "./App"
import { DetailSurface, SettingsSurface } from "./Shell"
import { SettingsWindow } from "./settings/SettingsWindow"
import { setLocale, type Locale } from "./localization"
import { defaultStripPreferences } from "./state/stripPreferences"
import type { Code0Action, Code0ActionResult, Code0SessionState } from "./state/code0Session"
import type { UsageSnapshot } from "./state/usage"
import { unavailableSnapshots } from "./state/usage"
import { code0BridgeFixture, code0Session, code0Snapshot, deferred } from "./test/code0SessionFixtures"

const a: UsageSnapshot = {...code0Snapshot(101, 202), fetchedAt: "2026-10-26T01:00:00Z"}
const b = code0Session({revision: 2, snapshot: code0Snapshot(8, 3)})
const visiblePreferences = {...defaultStripPreferences, hiddenProviders: []}
const locales: Locale[] = ["en", "zh-CN"]

function emit(name: string, value: unknown) { native.handlers.get(name)?.forEach(handler => handler({payload: value})) }
function ordinaryCalls() { return native.invoke.mock.calls.filter(([command]) => command === "service_account_status") }
function actionCalls() { return native.invoke.mock.calls.filter(([command]) => command === "code0_session_action") }
function openServices() { fireEvent.click(screen.getByRole("tab", {name: "Services"})) }

beforeEach(() => {
  native.handlers.clear()
  native.invoke.mockReset()
  native.invoke.mockImplementation(async (command, args) => {
    if (command === "app_settings") return {locale: "en", detailAutoHideSeconds: 300, stripPreferences: visiblePreferences}
    if (command === "available_displays" || command === "available_wsl_distributions") return []
    if (command === "service_account_status") return {providerId: args?.providerId, connectionState: "unavailable"}
    if (command === "update_state") return {phase: "idle", currentVersion: "fixture"}
    if (command === "deepseek_history_status") return {generation: null, status: "idle"}
    return undefined
  })
})
afterEach(() => { native.handlers.clear() })

it.each([false, true])("inline detail uses current session B or explicit null instead of ordinary A (clear=%s)", async clear => {
  const fixture = code0BridgeFixture(Promise.resolve(b))
  render(<App initialSnapshots={[...unavailableSnapshots.filter(value => value.providerId !== "code0"), a]}
    stripPreferences={visiblePreferences} code0Bridge={fixture.bridge} detailAutoHideSeconds={300} />)
  await act(async () => { await Promise.resolve() })
  fireEvent.click(screen.getByRole("button", {name: "Code0 usage"}))
  if (clear) act(() => fixture.emit(code0Session({revision: 3, state: "disconnected", snapshot: null})))
  const detail = screen.getByRole("dialog", {name: "Code0 details"})
  if (clear) {
    expect(within(detail).queryByText("101 site credits")).not.toBeInTheDocument()
    expect(detail.querySelector("time")).toBeNull()
  } else {
    expect(within(detail).getByText("8.00 site credits")).toBeVisible()
    expect(detail.querySelector("time")).toHaveAttribute("dateTime", b.snapshot!.fetchedAt)
  }
  fireEvent.focus(window)
  expect(fixture.bridge.perform).not.toHaveBeenCalled()
})

it.each([false, true])("native detail ignores late ordinary Code0 payloads after B or clear (clear=%s)", async clear => {
  const fixture = code0BridgeFixture(Promise.resolve(b))
  render(<DetailSurface code0Bridge={fixture.bridge} />)
  await act(async () => { await Promise.resolve() })
  act(() => emit("active-detail-changed", a))
  if (clear) act(() => fixture.emit(code0Session({revision: 3, state: "disconnected", snapshot: null})))
  act(() => { emit("snapshot-updated", a); emit("active-detail-changed", a) })
  const detail = screen.getByRole("dialog", {name: "Code0 details"})
  if (clear) {
    expect(within(detail).queryByText("101 site credits")).not.toBeInTheDocument()
    expect(detail.querySelector("time")).toBeNull()
  } else expect(within(detail).getByText("8.00 site credits")).toBeVisible()
  expect(fixture.bridge.perform).not.toHaveBeenCalled()
  expect(actionCalls()).toEqual([])
})

const actions: Array<{action: Code0Action; state: Code0SessionState; value: boolean; en: string; zh: string}> = [
  {action: "connect", state: "disconnected", value: false, en: "Connect Code0", zh: "连接 Code0"},
  {action: "loginCompleted", state: "connecting", value: false, en: "I've finished signing in", zh: "我已完成登录"},
  {action: "syncNow", state: "paused", value: true, en: "Sync Code0", zh: "同步 Code0"},
  {action: "disconnect", state: "paused", value: true, en: "Disconnect Code0", zh: "断开 Code0"},
]
it.each(locales.flatMap(locale => actions.map(action => ({...action, locale}))))("Services forwards only explicit $action in $locale", async ({action, state, value, en, zh, locale}) => {
  setLocale(locale)
  const onAction = vi.fn(async (): Promise<Code0ActionResult> => ({kind: "started"}))
  render(<SettingsWindow displayFont="System Default" onDisplayFontChange={() => {}} requestedTab="Services"
    code0Session={code0Session({state, snapshot: value ? b.snapshot : null})} onCode0Action={onAction} />)
  expect(onAction).not.toHaveBeenCalled()
  fireEvent.click(screen.getByRole("button", {name: locale === "en" ? en : zh}))
  await act(async () => { await Promise.resolve() })
  expect(onAction.mock.calls).toEqual([[action]])
  for (const name of ["Claude Code", "OpenAI Codex", "DeepSeek", "Google Antigravity"]) expect(screen.getByText(name)).toBeVisible()
})

it.each(locales.flatMap(locale => (["loginTimedOut", "authenticationChanged"] as const).map(failure => ({locale, failure}))))("integrated paused nil retains Disconnect after $failure in $locale", async ({locale, failure}) => {
  setLocale(locale)
  const onAction = vi.fn(async (): Promise<Code0ActionResult> => ({kind: "started"}))
  render(<SettingsWindow displayFont="System Default" onDisplayFontChange={() => {}} requestedTab="Services"
    code0Session={code0Session({state: "paused", snapshot: null, failure})} onCode0Action={onAction} />)
  const label = locale === "en" ? "Disconnect Code0" : "断开 Code0"
  expect(screen.getAllByRole("button", {name: label})).toHaveLength(1)
  fireEvent.click(screen.getByRole("button", {name: label}))
  await act(async () => { await Promise.resolve() })
  expect(onAction.mock.calls).toEqual([["disconnect"]])
})

it.each(["closing", "cleanupPending", "syncing"] as const)("integrated Services preserves the %s busy gate", state => {
  const onAction = vi.fn(async (): Promise<Code0ActionResult> => ({kind: "denied"}))
  render(<SettingsWindow displayFont="System Default" onDisplayFontChange={() => {}} requestedTab="Services"
    code0Session={code0Session({state})} onCode0Action={onAction} />)
  const button = screen.getByRole("button", {name: state === "syncing" ? "Sync Code0" : "Connect Code0"})
  expect(button).toBeDisabled()
  fireEvent.click(button)
  expect(onAction).not.toHaveBeenCalled()
})

it("settings mount and focus keep Code0 independent of late ordinary account results", async () => {
  const late = deferred<unknown>()
  const invoke = native.invoke.getMockImplementation()!
  native.invoke.mockImplementation((command, args) => command === "service_account_status" && args?.providerId === "claude" ? late.promise : invoke(command, args))
  const fixture = code0BridgeFixture(Promise.resolve(b))
  try {
    render(<SettingsSurface code0Bridge={fixture.bridge} />)
    openServices()
    await act(async () => { await Promise.resolve() })
    const code0Section = await screen.findByRole("region", {name: "Code0"})
    await waitFor(() => expect(code0Section.querySelector("time[datetime]")).toHaveAttribute("datetime", b.snapshot!.fetchedAt))
    act(() => fixture.emit(code0Session({revision: 3, state: "paused", failure: "authenticationChanged", snapshot: null})))
    await act(async () => {
      late.resolve({providerId: "code0", connectionState: "connected", accountLabel: "discard"})
      await late.promise
    })
    fireEvent.focus(window)
    expect(screen.getByRole("button", {name: "Disconnect Code0"})).toBeEnabled()
    expect(screen.queryByText("discard")).not.toBeInTheDocument()
    const currentCode0Section = screen.getByRole("region", {name: "Code0"})
    expect(currentCode0Section.querySelector("time[datetime]")).toBeNull()
    expect(within(currentCode0Section).getByRole("button", {name: "Connect Code0"})).toBeEnabled()
    expect(within(currentCode0Section).queryByRole("button", {name: "Sync Code0"})).not.toBeInTheDocument()
    expect(fixture.bridge.perform).not.toHaveBeenCalled()
    expect(actionCalls()).toEqual([])
    expect(ordinaryCalls().map(([, args]) => args?.providerId)).not.toContain("code0")
    expect(ordinaryCalls().every(([, args]) => args?.retryUsage === false)).toBe(true)
  } finally {
    await act(async () => { late.resolve({providerId: "claude", connectionState: "unavailable"}); await late.promise })
  }
})

it("a stable surface handler blocks a second submit across session and locale rerenders", async () => {
  const reply = deferred<Code0ActionResult>()
  const fixture = code0BridgeFixture(Promise.resolve(b))
  fixture.bridge.perform = vi.fn(() => reply.promise)
  try {
    const view = render(<SettingsSurface code0Bridge={fixture.bridge} />)
    openServices()
    const button = await screen.findByRole("button", {name: "Sync Code0"})
    fireEvent.click(button)
    act(() => { fixture.emit(code0Session({...b, revision: 3})); setLocale("zh-CN") })
    expect(screen.getByRole("button", {name: "同步 Code0"})).toBeDisabled()
    fireEvent.click(screen.getByRole("button", {name: "同步 Code0"}))
    expect(fixture.bridge.perform).toHaveBeenCalledOnce()
    expect(fixture.bridge.read).toHaveBeenCalledOnce()
    expect(fixture.bridge.onUpdated).toHaveBeenCalledOnce()
    view.unmount()
    await act(async () => { reply.resolve({kind: "busy"}); await reply.promise })
    expect(fixture.stop).toHaveBeenCalledOnce()
  } finally {
    await act(async () => { reply.resolve({kind: "denied"}); await reply.promise })
  }
})

it("a cold default backend has no implicit Code0 action on opening Services or focus", async () => {
  render(<SettingsSurface />)
  openServices()
  await act(async () => { await Promise.resolve() })
  fireEvent.focus(window)
  expect(actionCalls()).toEqual([])
  expect(ordinaryCalls().map(([, args]) => args?.providerId)).not.toContain("code0")
})

it("production Services installs only a stable read-only state subscription before any explicit action", async () => {
  const invoke = native.invoke.getMockImplementation()!
  native.invoke.mockImplementation((command, args) => command === "code0_session_state" ? Promise.resolve(b) : invoke(command, args))
  const view = render(<SettingsSurface />)
  openServices()
  await act(async () => { await Promise.resolve() })
  view.rerender(<SettingsSurface />)
  fireEvent.focus(window)
  await act(async () => { await Promise.resolve() })
  expect(native.invoke.mock.calls.filter(([command]) => command === "code0_session_state")).toEqual([["code0_session_state"]])
  expect(native.handlers.get("code0-session-updated")?.size).toBe(1)
  expect(screen.getByRole("button", {name: "Sync Code0"})).toBeEnabled()
  expect(actionCalls()).toEqual([])
  view.unmount()
  expect(native.handlers.get("code0-session-updated")?.size).toBe(0)
})
