import { act, fireEvent, render, screen, waitFor } from "@testing-library/react"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

const tauri = vi.hoisted(() => {
  const listeners = new Map<string, Set<(event: { payload: unknown }) => void>>()
  return {
    invoke: vi.fn((_command: string) => Promise.resolve<unknown>(undefined)),
    listeners,
    listen: vi.fn((event: string, handler: (event: { payload: unknown }) => void) => {
      const handlers = listeners.get(event) ?? new Set()
      handlers.add(handler)
      listeners.set(event, handlers)
      return Promise.resolve(() => handlers.delete(handler))
    }),
  }
})

const detailSettings = {
  displayFont: "Antonio",
  edge: "right",
  detailAutoHideSeconds: 8,
  refreshIntervalSeconds: 300,
  deepseekBalanceBaselineCents: 10_000,
  notificationsEnabled: false,
  launchAtLogin: false,
  claudeCli: { mode: "auto", customPath: null, wslDistribution: null },
  codexCli: { mode: "auto", customPath: null, wslDistribution: null },
}

vi.mock("@tauri-apps/api/core", () => ({ invoke: tauri.invoke }))
vi.mock("@tauri-apps/api/event", () => ({ listen: tauri.listen }))
vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({ label: "detail", startDragging: () => Promise.resolve() }),
}))

import { App } from "./App"
import { MeterClipPaths } from "./components/FloatingStrip"
import { UsageRing } from "./components/UsageRing"
import { ProviderDetail } from "./details/ProviderDetail"
import { DetailSurface } from "./Shell"
import { SettingsWindow } from "./settings/SettingsWindow"
import type { ProviderCliSettings, ServiceAccountStatus } from "./settings/SettingsWindow"
import type { UsageSnapshot } from "./state/usage"

const snapshots: UsageSnapshot[] = [
  {
    schemaVersion: 1,
    providerId: "claude",
    displayName: "Claude Code",
    status: "fresh",
    usedRatio: 0.23,
    primaryMetric: { label: "Session", current: 23, limit: 100, unit: "percent", kind: "officialLimit" },
    fetchedAt: "2026-09-03T00:00:00Z",
    staleAfterSeconds: 300,
    localActivity: { periodDays: 30, sessions: 12, tokens: 230000, activeDays: 8, longestSessionSeconds: 5400 },
  },
  {
    schemaVersion: 1,
    providerId: "codex",
    displayName: "OpenAI Codex",
    status: "fresh",
    usedRatio: 0.05,
    primaryMetric: { label: "Weekly limit", current: 5, limit: 100, unit: "percent", kind: "officialLimit" },
    fetchedAt: "2026-09-03T00:00:00Z",
    staleAfterSeconds: 300,
    resetCredits: [{ kind: "fullUsageReset", count: 1, expiresAt: "2026-09-21T00:25:00Z" }],
  },
  {
    schemaVersion: 1,
    providerId: "deepseek",
    displayName: "DeepSeek",
    status: "fresh",
    usedRatio: 0.2201,
    primaryMetric: { label: "Available balance", current: 77.99, unit: "cny", kind: "balance" },
    fetchedAt: "2026-09-03T00:00:00Z",
    staleAfterSeconds: 300,
    dailyHistory: [
      { date: "2026-09-02", costCny: 1.25, requests: 4, tokens: 1000 },
      { date: "2026-09-03", costCny: 2.5, requests: 7, tokens: 2500 },
    ],
  },
]

function emitTauriEvent(event: string, payload: unknown) {
  tauri.listeners.get(event)?.forEach((handler) => handler({ payload }))
}

function registerTauriListener(event: string, handler: (event: { payload: unknown }) => void) {
  const handlers = tauri.listeners.get(event) ?? new Set()
  handlers.add(handler)
  tauri.listeners.set(event, handlers)
  return Promise.resolve(() => handlers.delete(handler))
}

async function showDeepSeekDetail() {
  render(<DetailSurface />)
  await act(async () => {
    await Promise.resolve()
  })
  act(() => emitTauriEvent("active-detail-changed", snapshots[2]))
  return screen.getByRole("dialog", { name: "DeepSeek details" })
}

beforeEach(() => {
  tauri.listeners.clear()
  tauri.listen.mockReset()
  tauri.listen.mockImplementation(registerTauriListener)
  tauri.invoke.mockReset()
  tauri.invoke.mockImplementation((command: string) => {
    if (command === "app_settings") return Promise.resolve(detailSettings)
    if (command === "deepseek_history_status") {
      return Promise.resolve({ generation: null, status: "idle" })
    }
    if (command === "open_deepseek_history") {
      return Promise.resolve({ generation: 1, status: "opening" })
    }
    return Promise.resolve(undefined)
  })
})

afterEach(() => {
  vi.useRealTimers()
})

describe("Windows meter interface", () => {
  it("provides the exact normalized macOS silhouette for both screen edges", () => {
    const { container } = render(<MeterClipPaths />)

    expect(container.querySelector("#meter-clip-right path")).toHaveAttribute(
      "d",
      "M 1 0.0449438 C 0.9074074 0.0646067 0.8148148 0.0758427 0.6111111 0.0786517 C 0.2685185 0.0814607 0 0.1516854 0 0.247191 L 0 0.752809 C 0 0.8483146 0.2685185 0.9185393 0.6111111 0.9213483 C 0.8148148 0.9241573 0.9074074 0.9353933 1 0.9550562 Z",
    )
    expect(container.querySelector("#meter-clip-left path")).toHaveAttribute(
      "d",
      "M 0 0.0449438 C 0.0925926 0.0646067 0.1851852 0.0758427 0.3888889 0.0786517 C 0.7314815 0.0814607 1 0.1516854 1 0.247191 L 1 0.752809 C 1 0.8483146 0.7314815 0.9185393 0.3888889 0.9213483 C 0.1851852 0.9241573 0.0925926 0.9353933 0 0.9550562 Z",
    )
  })

  it("renders three equally-sized logo-only rings with distinct provider colors", () => {
    const { container } = render(<App initialSnapshots={snapshots} />)

    expect(screen.getByRole("button", { name: "Claude Code usage" })).toBeVisible()
    expect(screen.getByRole("button", { name: "OpenAI Codex usage" })).toBeVisible()
    expect(screen.getByRole("button", { name: "DeepSeek usage" })).toBeVisible()
    expect(container.querySelectorAll(".provider-logo--uniform")).toHaveLength(3)
    expect(container.querySelectorAll(".usage-ring")).toHaveLength(3)
    expect(container.querySelector(".provider-status-text")).not.toBeInTheDocument()
  })

  it("uses consumed balance for DeepSeek and never invents zero for unavailable data", () => {
    const { rerender } = render(<UsageRing snapshot={snapshots[2]} onActivate={() => {}} />)
    expect(screen.getByRole("progressbar", { name: "DeepSeek usage" })).toHaveAttribute(
      "aria-valuenow",
      "22.01",
    )

    const unavailable = { ...snapshots[0], status: "unavailable" as const, usedRatio: null }
    rerender(<UsageRing snapshot={unavailable} onActivate={() => {}} />)
    expect(screen.getByRole("progressbar", { name: "Claude Code usage" })).not.toHaveAttribute("aria-valuenow")
    expect(screen.queryByText("0%")).not.toBeInTheDocument()
  })

  it("opens one rich detail and closes it on an outside click", () => {
    const { container } = render(<App initialSnapshots={snapshots} />)
    const codexButton = container.querySelector<HTMLButtonElement>(
      'button[aria-label="OpenAI Codex usage"]',
    )
    expect(codexButton).toBeInTheDocument()
    fireEvent.click(codexButton!)

    const detail = container.querySelector<HTMLElement>(
      '[role="dialog"][aria-label="OpenAI Codex details"]',
    )
    expect(detail).toBeInTheDocument()
    expect(detail).toHaveTextContent("Official quota")
    expect(detail).toHaveTextContent("Reset credits")

    fireEvent.pointerDown(document.body)
    expect(container.querySelector('[role="dialog"]')).not.toBeInTheDocument()
  })

  it("renders official DeepSeek history totals and a scaled daily chart", () => {
    render(<App initialSnapshots={snapshots} />)
    fireEvent.click(screen.getByRole("button", { name: "DeepSeek usage" }))

    expect(screen.getByRole("img", { name: "DeepSeek cost for the last 30 days" })).toBeVisible()
    expect(screen.getByText("¥3.75")).toBeVisible()
    expect(screen.getByText("11", { selector: "strong" })).toBeVisible()
    expect(screen.getByText("3.5K")).toBeVisible()
    expect(screen.queryByText("Usage history will appear here after official-page sync.")).not.toBeInTheDocument()
  })

  it("keeps existing history visible and exposes retry after a later sync fails", async () => {
    await showDeepSeekDetail()

    act(() => emitTauriEvent("deepseek-history-status", { generation: 7, status: "opening" }))
    act(() => emitTauriEvent("deepseek-history-status", { generation: 7, status: "failed" }))

    expect(screen.getByRole("img", { name: "DeepSeek cost for the last 30 days" })).toBeVisible()
    expect(screen.getByRole("alert")).toHaveTextContent("Official history sync could not be started. Try again.")
    fireEvent.click(screen.getByRole("button", { name: "Try again" }))
    expect(tauri.invoke).toHaveBeenCalledWith("open_deepseek_history")
  })

  it("offers a working retry when DeepSeek history is unavailable", () => {
    const sync = vi.fn()
    const withoutHistory = snapshots.map((snapshot) => snapshot.providerId === "deepseek"
      ? { ...snapshot, dailyHistory: [] }
      : snapshot)
    render(<App initialSnapshots={withoutHistory} onDeepSeekHistorySync={sync} />)
    fireEvent.click(screen.getByRole("button", { name: "DeepSeek usage" }))
    fireEvent.click(screen.getByRole("button", { name: "Sync official history" }))
    expect(sync).toHaveBeenCalledOnce()
  })

  it("shows opening immediately, disables repeat sync, and keeps the detail visible", async () => {
    vi.useFakeTimers()
    const withoutHistory = { ...snapshots[2], dailyHistory: [] }
    const dialog = await showDeepSeekDetail()
    act(() => emitTauriEvent("active-detail-changed", withoutHistory))

    fireEvent.click(screen.getByRole("button", { name: "Sync official history" }))

    expect(screen.getByText("Opening official page…")).toBeVisible()
    expect(screen.getByRole("button", { name: "Sync official history" })).toBeDisabled()
    act(() => vi.advanceTimersByTime(9_000))
    expect(dialog).toBeVisible()
    vi.useRealTimers()
  })

  it("reports a rejected sync command as a recoverable error", async () => {
    tauri.invoke.mockImplementation((command: string) => {
      if (command === "app_settings") return Promise.resolve(detailSettings)
      if (command === "deepseek_history_status") {
        return Promise.resolve({ generation: null, status: "idle" })
      }
      return command === "open_deepseek_history"
        ? Promise.reject(new Error("window unavailable"))
        : Promise.resolve(undefined)
    })
    const withoutHistory = { ...snapshots[2], dailyHistory: [] }
    await showDeepSeekDetail()
    act(() => emitTauriEvent("active-detail-changed", withoutHistory))

    fireEvent.click(screen.getByRole("button", { name: "Sync official history" }))
    await screen.findByRole("alert")

    expect(screen.getByRole("alert")).toHaveTextContent("Official history sync could not be started. Try again.")
    expect(screen.getByRole("button", { name: "Try again" })).toBeEnabled()
  })

  it("accepts status only for the active generation and ignores a delayed older terminal", async () => {
    vi.useFakeTimers()
    let rejectFirstAttempt: (error: Error) => void = () => {}
    let attempts = 0
    tauri.invoke.mockImplementation((command: string) => {
      if (command === "app_settings") return Promise.resolve(detailSettings)
      if (command === "deepseek_history_status") {
        return Promise.resolve({ generation: null, status: "idle" })
      }
      if (command !== "open_deepseek_history") return Promise.resolve(undefined)
      attempts += 1
      return attempts === 1
        ? new Promise((_, reject) => { rejectFirstAttempt = reject })
        : Promise.resolve({ generation: 2, status: "opening" })
    })
    const withoutHistory = { ...snapshots[2], dailyHistory: [] }
    const dialog = await showDeepSeekDetail()
    act(() => emitTauriEvent("active-detail-changed", withoutHistory))

    fireEvent.click(screen.getByRole("button", { name: "Sync official history" }))
    act(() => emitTauriEvent("deepseek-history-status", { generation: 1, status: "opening" }))
    act(() => emitTauriEvent("deepseek-history-status", { generation: 1, status: "failed" }))
    fireEvent.click(screen.getByRole("button", { name: "Try again" }))
    await act(async () => { await Promise.resolve() })
    act(() => emitTauriEvent("deepseek-history-status", { generation: 2, status: "active" }))
    act(() => emitTauriEvent("deepseek-history-status", { generation: 1, status: "failed" }))
    await act(async () => {
      rejectFirstAttempt(new Error("late first attempt"))
      await Promise.resolve()
    })

    expect(screen.getByText("Sync in progress")).toBeVisible()
    act(() => vi.advanceTimersByTime(9_000))
    expect(dialog).toBeVisible()
    vi.useRealTimers()
  })

  it("does not let a late open response regress an already-active generation", async () => {
    let resolveOpen: (value: unknown) => void = () => {}
    tauri.invoke.mockImplementation((command: string) => {
      if (command === "app_settings") return Promise.resolve(detailSettings)
      if (command === "deepseek_history_status") {
        return Promise.resolve({ generation: null, status: "idle" })
      }
      if (command === "open_deepseek_history") {
        return new Promise((resolve) => { resolveOpen = resolve })
      }
      return Promise.resolve(undefined)
    })
    const withoutHistory = { ...snapshots[2], dailyHistory: [] }
    await showDeepSeekDetail()
    act(() => emitTauriEvent("active-detail-changed", withoutHistory))

    fireEvent.click(screen.getByRole("button", { name: "Sync official history" }))
    act(() => emitTauriEvent("deepseek-history-status", { generation: 5, status: "opening" }))
    act(() => emitTauriEvent("deepseek-history-status", { generation: 5, status: "active" }))
    await act(async () => {
      resolveOpen({ generation: 5, status: "opening" })
      await Promise.resolve()
    })

    expect(screen.getByText("Sync in progress")).toBeVisible()
  })

  it("does not let the initial status query overwrite a user-started attempt", async () => {
    let resolveInitialStatus: (value: unknown) => void = () => {}
    tauri.invoke.mockImplementation((command: string) => {
      if (command === "app_settings") return Promise.resolve(detailSettings)
      if (command === "deepseek_history_status") {
        return new Promise((resolve) => { resolveInitialStatus = resolve })
      }
      if (command === "open_deepseek_history") return new Promise(() => {})
      return Promise.resolve(undefined)
    })
    const withoutHistory = { ...snapshots[2], dailyHistory: [] }
    await showDeepSeekDetail()
    act(() => emitTauriEvent("active-detail-changed", withoutHistory))
    await waitFor(() => {
      expect(tauri.invoke).toHaveBeenCalledWith("deepseek_history_status")
    })

    fireEvent.click(screen.getByRole("button", { name: "Sync official history" }))
    expect(screen.getByText("Opening official page…")).toBeVisible()
    await act(async () => {
      resolveInitialStatus({ generation: 99, status: "active" })
      await Promise.resolve()
    })

    expect(screen.getByText("Opening official page…")).toBeVisible()
  })

  it.each([
    ["opening", "Opening official page…"],
    ["active", "Sync in progress"],
  ] as const)("preserves an event-confirmed %s session when the command and recovery query fail", async (status, message) => {
    vi.useFakeTimers()
    let rejectOpen: (error: Error) => void = () => {}
    let statusQueries = 0
    tauri.invoke.mockImplementation((command: string) => {
      if (command === "app_settings") return Promise.resolve(detailSettings)
      if (command === "deepseek_history_status") {
        statusQueries += 1
        return statusQueries === 1
          ? Promise.resolve({ generation: null, status: "idle" })
          : Promise.reject(new Error("query unavailable"))
      }
      if (command === "open_deepseek_history") {
        return new Promise((_, reject) => { rejectOpen = reject })
      }
      return Promise.resolve(undefined)
    })
    const withoutHistory = { ...snapshots[2], dailyHistory: [] }
    const dialog = await showDeepSeekDetail()
    act(() => emitTauriEvent("active-detail-changed", withoutHistory))

    fireEvent.click(screen.getByRole("button", { name: "Sync official history" }))
    act(() => emitTauriEvent("deepseek-history-status", { generation: 12, status }))
    await act(async () => {
      rejectOpen(new Error("open response lost"))
      await Promise.resolve()
      await Promise.resolve()
    })

    expect(screen.getByText(message)).toBeVisible()
    act(() => vi.advanceTimersByTime(9_000))
    expect(dialog).toBeVisible()
    vi.useRealTimers()
  })

  it("reports a recoverable failure when the open command returns no session generation", async () => {
    tauri.invoke.mockImplementation((command: string) => {
      if (command === "app_settings") return Promise.resolve(detailSettings)
      if (command === "deepseek_history_status") {
        return Promise.resolve({ generation: null, status: "idle" })
      }
      if (command === "open_deepseek_history") {
        return Promise.resolve({ generation: null, status: "idle" })
      }
      return Promise.resolve(undefined)
    })
    const withoutHistory = { ...snapshots[2], dailyHistory: [] }
    await showDeepSeekDetail()
    act(() => emitTauriEvent("active-detail-changed", withoutHistory))

    fireEvent.click(screen.getByRole("button", { name: "Sync official history" }))
    await act(async () => { await Promise.resolve() })

    expect(screen.getByRole("alert")).toBeVisible()
    expect(screen.getByRole("button", { name: "Try again" })).toBeEnabled()
  })

  it.each(["completed", "cancelled"] as const)(
    "accepts a generation-bound %s status from command recovery",
    async (terminalStatus) => {
      let statusQueries = 0
      tauri.invoke.mockImplementation((command: string) => {
        if (command === "app_settings") return Promise.resolve(detailSettings)
        if (command === "deepseek_history_status") {
          statusQueries += 1
          return Promise.resolve(statusQueries === 1
            ? { generation: null, status: "idle" }
            : { generation: 18, status: terminalStatus })
        }
        if (command === "open_deepseek_history") {
          return Promise.reject(new Error("open response lost"))
        }
        return Promise.resolve(undefined)
      })
      const withoutHistory = { ...snapshots[2], dailyHistory: [] }
      await showDeepSeekDetail()
      act(() => emitTauriEvent("active-detail-changed", withoutHistory))

      fireEvent.click(screen.getByRole("button", { name: "Sync official history" }))
      await act(async () => {
        await Promise.resolve()
        await Promise.resolve()
      })

      expect(screen.queryByRole("alert")).not.toBeInTheDocument()
      expect(screen.getByRole("button", { name: "Sync official history" })).toBeEnabled()
    },
  )

  it("keeps the previous generation as a retry floor while the new open response is pending", async () => {
    let resolveRetry: (value: unknown) => void = () => {}
    let attempts = 0
    tauri.invoke.mockImplementation((command: string) => {
      if (command === "app_settings") return Promise.resolve(detailSettings)
      if (command === "deepseek_history_status") {
        return Promise.resolve({ generation: null, status: "idle" })
      }
      if (command !== "open_deepseek_history") return Promise.resolve(undefined)
      attempts += 1
      return attempts === 1
        ? Promise.resolve({ generation: 1, status: "opening" })
        : new Promise((resolve) => { resolveRetry = resolve })
    })
    const withoutHistory = { ...snapshots[2], dailyHistory: [] }
    await showDeepSeekDetail()
    act(() => emitTauriEvent("active-detail-changed", withoutHistory))

    fireEvent.click(screen.getByRole("button", { name: "Sync official history" }))
    await act(async () => { await Promise.resolve() })
    act(() => emitTauriEvent("deepseek-history-status", { generation: 1, status: "failed" }))
    fireEvent.click(screen.getByRole("button", { name: "Try again" }))

    act(() => emitTauriEvent("deepseek-history-status", { generation: 1, status: "active" }))
    expect(screen.getByText("Opening official page…")).toBeVisible()
    act(() => emitTauriEvent("deepseek-history-status", { generation: 2, status: "opening" }))
    expect(screen.getByText("Opening official page…")).toBeVisible()
    await act(async () => {
      resolveRetry({ generation: 2, status: "opening" })
      await Promise.resolve()
    })

    expect(screen.getByText("Opening official page…")).toBeVisible()
  })

  it("cleans up successfully registered detail listeners when another registration fails", async () => {
    const activeStop = vi.fn()
    const historyStop = vi.fn()
    tauri.listen.mockImplementation((event: string, _handler: (event: { payload: unknown }) => void) => {
      if (event === "snapshot-updated") return Promise.reject(new Error("subscription unavailable"))
      if (event === "active-detail-changed") return Promise.resolve(activeStop)
      if (event === "deepseek-history-status") return Promise.resolve(historyStop)
      return Promise.resolve(vi.fn())
    })

    const { unmount } = render(<DetailSurface />)
    await act(async () => {
      await Promise.resolve()
      await Promise.resolve()
    })
    unmount()

    expect(activeStop).toHaveBeenCalledOnce()
    expect(historyStop).toHaveBeenCalledOnce()
  })

  it("does not start history sync when neither events nor the status query are available", async () => {
    tauri.listen.mockImplementation((event: string, handler: (event: { payload: unknown }) => void) => {
      if (event === "deepseek-history-status") return Promise.reject(new Error("listener unavailable"))
      return registerTauriListener(event, handler)
    })
    tauri.invoke.mockImplementation((command: string) => {
      if (command === "app_settings") return Promise.resolve(detailSettings)
      if (command === "deepseek_history_status") return Promise.reject(new Error("query unavailable"))
      return Promise.resolve(undefined)
    })
    const withoutHistory = { ...snapshots[2], dailyHistory: [] }
    await showDeepSeekDetail()
    act(() => emitTauriEvent("active-detail-changed", withoutHistory))

    expect(screen.getByRole("button", { name: "Sync official history" })).toBeDisabled()
    expect(tauri.invoke).not.toHaveBeenCalledWith("open_deepseek_history")
  })

  it("uses the status query as a recovery path when event registration fails", async () => {
    tauri.listen.mockImplementation((event: string, handler: (event: { payload: unknown }) => void) => {
      if (event === "deepseek-history-status") return Promise.reject(new Error("listener unavailable"))
      return registerTauriListener(event, handler)
    })
    tauri.invoke.mockImplementation((command: string) => {
      if (command === "app_settings") return Promise.resolve(detailSettings)
      if (command === "deepseek_history_status") {
        return Promise.resolve({ generation: null, status: "idle" })
      }
      if (command === "open_deepseek_history") {
        return Promise.resolve({ generation: 7, status: "active" })
      }
      return Promise.resolve(undefined)
    })
    const withoutHistory = { ...snapshots[2], dailyHistory: [] }
    await showDeepSeekDetail()
    act(() => emitTauriEvent("active-detail-changed", withoutHistory))

    fireEvent.click(screen.getByRole("button", { name: "Sync official history" }))
    expect(await screen.findByText("Sync in progress")).toBeVisible()
    expect(tauri.invoke).toHaveBeenCalledWith("open_deepseek_history")
  })

  it("queries while syncing so a backend event-send failure cannot leave opening stuck", async () => {
    vi.useFakeTimers()
    let statusQueries = 0
    tauri.invoke.mockImplementation((command: string) => {
      if (command === "app_settings") return Promise.resolve(detailSettings)
      if (command === "deepseek_history_status") {
        statusQueries += 1
        return Promise.resolve(statusQueries === 1
          ? { generation: null, status: "idle" }
          : { generation: 9, status: "active" })
      }
      if (command === "open_deepseek_history") {
        return Promise.resolve({ generation: 9, status: "opening" })
      }
      return Promise.resolve(undefined)
    })
    const withoutHistory = { ...snapshots[2], dailyHistory: [] }
    await showDeepSeekDetail()
    act(() => emitTauriEvent("active-detail-changed", withoutHistory))

    fireEvent.click(screen.getByRole("button", { name: "Sync official history" }))
    await act(async () => { await Promise.resolve() })
    expect(screen.getByText("Opening official page…")).toBeVisible()
    await act(async () => { await vi.advanceTimersByTimeAsync(501) })

    expect(screen.getByText("Sync in progress")).toBeVisible()
    vi.useRealTimers()
  })

  it.each(["completed", "cancelled", "failed"] as const)("resumes the auto-hide countdown after %s", async (terminalStatus) => {
    vi.useFakeTimers()
    const withoutHistory = { ...snapshots[2], dailyHistory: [] }
    const dialog = await showDeepSeekDetail()
    act(() => emitTauriEvent("active-detail-changed", withoutHistory))

    fireEvent.click(screen.getByRole("button", { name: "Sync official history" }))
    act(() => emitTauriEvent("deepseek-history-status", { generation: 1, status: "active" }))
    expect(screen.getByText("Sync in progress")).toBeVisible()
    act(() => vi.advanceTimersByTime(9_000))
    expect(dialog).toBeVisible()

    act(() => emitTauriEvent("deepseek-history-status", { generation: 1, status: terminalStatus }))
    act(() => vi.advanceTimersByTime(8_100))
    expect(screen.queryByRole("dialog", { name: "DeepSeek details" })).not.toBeInTheDocument()
    vi.useRealTimers()
  })

  it("auto-hides details and pauses the timer while the user interacts", () => {
    vi.useFakeTimers()
    render(<App initialSnapshots={snapshots} detailAutoHideSeconds={2} />)
    fireEvent.click(screen.getByRole("button", { name: "Claude Code usage" }))
    const dialog = screen.getByRole("dialog")
    fireEvent.pointerEnter(dialog)
    act(() => vi.advanceTimersByTime(3_000))
    expect(dialog).toBeVisible()
    fireEvent.pointerLeave(dialog)
    act(() => vi.advanceTimersByTime(2_100))
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument()
    vi.useRealTimers()
  })

  it("keeps Settings on the system font while display font changes immediately", () => {
    const changed = vi.fn()
    render(<SettingsWindow displayFont="Antonio" onDisplayFontChange={changed} />)

    const settings = screen.getByRole("dialog", { name: "AI Token Meter Settings" })
    expect(settings).toHaveClass("settings-window--system-font")
    fireEvent.change(screen.getByLabelText("Display font"), { target: { value: "Menlo" } })
    expect(changed).toHaveBeenCalledWith("Menlo")
    expect(screen.getByRole("tab", { name: "Appearance" })).toBeVisible()
    expect(screen.getByRole("tab", { name: "Monitoring" })).toBeVisible()
    expect(screen.getByRole("tab", { name: "Services" })).toBeVisible()
    expect(screen.getByRole("tab", { name: "About" })).toBeVisible()
  })

  it("renders Provider details at the compact Windows density", () => {
    const { container } = render(
      <ProviderDetail
        onInteractionEnd={() => {}}
        onInteractionStart={() => {}}
        onPointerEnter={() => {}}
        onPointerLeave={() => {}}
        snapshot={snapshots[0]}
      />,
    )

    const detail = screen.getByRole("dialog", { name: "Claude Code details" })
    const identityTitle = container.querySelector(".provider-detail__identity strong")
    const headline = container.querySelector(".provider-detail__headline")
    const sectionTitle = container.querySelector(".detail-section h2")
    const cardValue = container.querySelector(".metric-card strong")

    expect(detail).toHaveClass("provider-detail--compact-density")
    expect(identityTitle).toBeInTheDocument()
    expect(headline).toBeInTheDocument()
    expect(sectionTitle).toBeInTheDocument()
    expect(cardValue).toBeInTheDocument()
  })

  it("renders Settings with compact system typography and readable display-font controls", () => {
    render(<SettingsWindow displayFont="Antonio" onDisplayFontChange={() => {}} />)

    const settings = screen.getByRole("dialog", { name: "AI Token Meter Settings" })
    const title = settings.querySelector("header strong")
    const displayFont = screen.getByLabelText("Display font")

    expect(settings).toHaveClass("settings-window--compact-density", "settings-window--system-font")
    expect(title).toBeInTheDocument()
    expect(displayFont).toBeVisible()
  })

  it("edits refresh cadence and DeepSeek balance baseline in Monitoring", () => {
    const refresh = vi.fn()
    const baseline = vi.fn()
    render(
      <SettingsWindow
        deepseekBalanceBaselineCents={10_000}
        displayFont="Antonio"
        onDeepSeekBalanceBaselineCentsChange={baseline}
        onDisplayFontChange={() => {}}
        onRefreshIntervalSecondsChange={refresh}
        refreshIntervalSeconds={300}
        requestedTab="Monitoring"
      />,
    )

    fireEvent.change(screen.getByLabelText("Refresh interval seconds"), { target: { value: "120" } })
    expect(refresh).not.toHaveBeenCalled()
    expect(screen.getByLabelText("Refresh interval seconds")).toHaveValue(120)
    fireEvent.blur(screen.getByLabelText("Refresh interval seconds"))
    expect(refresh).toHaveBeenCalledWith(120)
    fireEvent.change(screen.getByLabelText("Refresh interval seconds"), { target: { value: "120.5" } })
    fireEvent.keyDown(screen.getByLabelText("Refresh interval seconds"), { key: "Enter" })
    expect(refresh).toHaveBeenCalledTimes(1)
    expect(screen.getByLabelText("Refresh interval seconds")).toHaveValue(300)
    fireEvent.change(screen.getByLabelText("DeepSeek balance baseline"), { target: { value: "250.5" } })
    expect(baseline).not.toHaveBeenCalled()
    fireEvent.keyDown(screen.getByLabelText("DeepSeek balance baseline"), { key: "Enter" })
    expect(baseline).toHaveBeenCalledWith(25_050)
    fireEvent.change(screen.getByLabelText("DeepSeek balance baseline"), { target: { value: "250.505" } })
    fireEvent.blur(screen.getByLabelText("DeepSeek balance baseline"))
    expect(baseline).toHaveBeenCalledTimes(1)
    expect(screen.getByLabelText("DeepSeek balance baseline")).toHaveValue(100)
  })

  it("keeps usage alerts and launch-at-login as explicit Monitoring toggles", () => {
    const alerts = vi.fn()
    const launch = vi.fn()
    render(
      <SettingsWindow
        displayFont="Antonio"
        launchAtLogin={false}
        notificationsEnabled={false}
        onDisplayFontChange={() => {}}
        onLaunchAtLoginChange={launch}
        onNotificationsEnabledChange={alerts}
        requestedTab="Monitoring"
      />,
    )

    fireEvent.click(screen.getByLabelText("Usage alerts at 70% and 90%"))
    expect(alerts).toHaveBeenCalledWith(true)
    fireEvent.click(screen.getByLabelText("Open AI Token Meter at login"))
    expect(launch).toHaveBeenCalledWith(true)
  })

  it("checks and installs updates only through separate explicit About actions", () => {
    const check = vi.fn()
    const install = vi.fn()
    const { rerender } = render(
      <SettingsWindow
        displayFont="Antonio"
        onDisplayFontChange={() => {}}
        onCheckForUpdates={check}
        onInstallUpdate={install}
        requestedTab="About"
        updateState={{ phase: "idle", currentVersion: "0.2.2" }}
      />,
    )
    fireEvent.click(screen.getByRole("button", { name: "Check for Updates" }))
    expect(check).toHaveBeenCalledOnce()
    expect(screen.getByRole("button", { name: "Update Now" })).toBeDisabled()

    rerender(
      <SettingsWindow
        displayFont="Antonio"
        onDisplayFontChange={() => {}}
        onCheckForUpdates={check}
        onInstallUpdate={install}
        requestedTab="About"
        updateState={{ phase: "available", currentVersion: "0.2.2", availableVersion: "0.3.0-preview.1" }}
      />,
    )
    fireEvent.click(screen.getByRole("button", { name: "Update Now" }))
    expect(install).toHaveBeenCalledOnce()
    expect(screen.getByText("Version 0.3.0-preview.1 is available.")).toBeVisible()
  })

  it("shows current service identities and keeps sign-in and API replacement explicit", () => {
    const signIn = vi.fn()
    const check = vi.fn()
    const replaceKey = vi.fn()
    const statuses: ServiceAccountStatus[] = [
      {
        providerId: "claude",
        connectionState: "connected",
        accountLabel: "member@example.com",
        accountDetail: "Claude Code · Max",
        runtimeSource: "Native Windows",
        cliVersion: "2.1.223",
      },
      {
        providerId: "codex",
        connectionState: "signInRequired",
        runtimeSource: "WSL · Ubuntu",
        cliVersion: "0.148.0",
      },
      {
        providerId: "deepseek",
        connectionState: "connected",
        accountLabel: "API Key ••••7xyz",
        accountDetail: "Windows Credential Manager",
      },
    ]
    render(
      <SettingsWindow
        displayFont="Antonio"
        onBeginServiceSignIn={signIn}
        onCheckServiceStatus={check}
        onDisplayFontChange={() => {}}
        onReplaceDeepSeekKey={replaceKey}
        requestedTab="Services"
        serviceStatuses={statuses}
      />,
    )

    expect(screen.getByText("member@example.com")).toBeVisible()
    expect(screen.getByText("Native Windows · CLI 2.1.223")).toBeVisible()
    expect(screen.getByText("WSL · Ubuntu · CLI 0.148.0")).toBeVisible()
    fireEvent.click(screen.getByRole("button", { name: "Sign in again to Claude Code" }))
    expect(signIn).toHaveBeenCalledWith("claude")
    fireEvent.click(screen.getByRole("button", { name: "Check OpenAI Codex status" }))
    expect(check).toHaveBeenCalledWith("codex")

    fireEvent.click(screen.getByRole("button", { name: "Replace DeepSeek API Key" }))
    expect(replaceKey).toHaveBeenCalledWith()
    expect(screen.queryByLabelText("DeepSeek API Key")).not.toBeInTheDocument()
    expect(screen.getByText("Windows opens a protected credential prompt; the Key never enters this WebView.")).toBeVisible()
  })

  it("lets each CLI service choose Auto, native Windows or an explicit WSL distribution", () => {
    const change = vi.fn()
    const claude: ProviderCliSettings = {
      mode: "auto",
      customPath: null,
      wslDistribution: null,
    }
    const codex: ProviderCliSettings = {
      mode: "wsl",
      customPath: null,
      wslDistribution: "Ubuntu-24.04",
    }

    render(
      <SettingsWindow
        cliSettings={{ claude, codex }}
        displayFont="Antonio"
        onCliSettingsChange={change}
        onDisplayFontChange={() => {}}
        requestedTab="Services"
        wslDistributions={["Ubuntu-24.04", "Debian"]}
      />,
    )

    fireEvent.change(screen.getByLabelText("Claude Code runtime"), {
      target: { value: "nativeWindows" },
    })
    expect(change).toHaveBeenCalledWith("claude", {
      ...claude,
      mode: "nativeWindows",
      wslDistribution: null,
    })

    expect(screen.getByLabelText("OpenAI Codex WSL distribution")).toHaveValue("Ubuntu-24.04")
    fireEvent.change(screen.getByLabelText("OpenAI Codex WSL distribution"), {
      target: { value: "Debian" },
    })
    expect(change).toHaveBeenCalledWith("codex", {
      ...codex,
      wslDistribution: "Debian",
    })
  })
})
