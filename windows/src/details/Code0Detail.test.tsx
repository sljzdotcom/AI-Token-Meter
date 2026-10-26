import { act, fireEvent, render, screen } from "@testing-library/react"
import { expect, it, vi } from "vitest"
const native = vi.hoisted(() => ({invoke: vi.fn(async () => undefined)}))
vi.mock("@tauri-apps/api/core", () => ({invoke: native.invoke}))
import { setLocale, type Locale } from "../localization"
import { code0Session, code0Snapshot } from "../test/code0SessionFixtures"
import { Code0Detail } from "./Code0Detail"

const locales: Locale[] = ["en", "zh-CN"]

it.each(locales)("renders balance and cumulative site credits with the original native success time in %s", locale => {
  setLocale(locale)
  const current = code0Session()
  const {container} = render(<Code0Detail session={current} />)
  expect(screen.getByText(locale === "en" ? "Balance" : "余额")).toBeVisible()
  expect(screen.getByText(locale === "en" ? "Cumulative usage" : "累计已用")).toBeVisible()
  expect(screen.getByText(locale === "en" ? "2.50 site credits" : "2.50 站内额度")).toBeVisible()
  expect(screen.getByText(locale === "en" ? "1.50 site credits" : "1.50 站内额度")).toBeVisible()
  expect(container.querySelector("time")).toHaveAttribute("datetime", current.snapshot!.fetchedAt)
  expect(container.textContent).not.toMatch(/[$¥%]|USD|CNY|API Key|tokens/i)
  expect(screen.queryByRole("button")).not.toBeInTheDocument()
})

it("zero balance and zero cumulative usage remain valid values, never unavailable or a ratio", () => {
  const {container} = render(<Code0Detail session={code0Session({snapshot: code0Snapshot(0, 0)})} />)
  expect(screen.getAllByText("0.00 site credits")).toHaveLength(2)
  expect(container.textContent).not.toContain("%")
})

it("explicit null clears old numeric data and its successful time", () => {
  const {container, rerender} = render(<Code0Detail session={code0Session()} />)
  expect.soft(screen.queryByText("2.50 site credits")).toBeVisible()
  expect.soft(container.querySelector("time")).toHaveAttribute("datetime", "2026-10-27T04:00:00Z")
  rerender(<Code0Detail session={code0Session({revision: 2, state: "disconnected", snapshot: null})} />)
  expect(screen.queryByText("2.50 site credits")).not.toBeInTheDocument()
  expect(screen.queryByText("1.50 site credits")).not.toBeInTheDocument()
  expect(container.querySelector("time")).toBeNull()
  expect(screen.getByText("Connect Code0 in Services to view website usage.")).toBeVisible()
})

it.each(locales)("failure keeps prior values and success time while independently showing paused state in %s", locale => {
  setLocale(locale)
  const current = code0Session()
  const {container, rerender} = render(<Code0Detail session={current} />)
  rerender(<Code0Detail session={{...current, revision: 2, failure: "syncTimedOut"}} />)
  expect(screen.getByText(locale === "en" ? "Code0 sync timed out. Previous data and success time are unchanged." : "Code0 同步超时，原有数据和成功时间保持不变。")).toBeVisible()
  expect(container.querySelector("time")).toHaveAttribute("datetime", current.snapshot!.fetchedAt)
  expect(screen.getByText(locale === "en" ? "2.50 site credits" : "2.50 站内额度")).toBeVisible()
})

it("mount, focus, language and numeric rerender remain display-only with zero native commands", () => {
  const before = native.invoke.mock.calls.length
  const {container, rerender} = render(<Code0Detail session={code0Session()} />)
  fireEvent.focus(screen.getByRole("dialog"))
  act(() => setLocale("zh-CN"))
  rerender(<Code0Detail session={code0Session({revision: 2, snapshot: code0Snapshot(8, 3)})} />)
  expect(native.invoke.mock.calls.length).toBe(before)
  expect(screen.queryByRole("button")).not.toBeInTheDocument()
  expect(container.querySelector("input")).toBeNull()
})
