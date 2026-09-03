import { act, fireEvent, render, screen } from "@testing-library/react"
import { describe, expect, it, vi } from "vitest"

import { App } from "./App"
import { UsageRing } from "./components/UsageRing"
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

describe("Windows meter interface", () => {
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
    render(<App initialSnapshots={snapshots} />)
    fireEvent.click(screen.getByRole("button", { name: "OpenAI Codex usage" }))

    expect(screen.getByRole("dialog", { name: "OpenAI Codex details" })).toBeVisible()
    expect(screen.getByText("Official quota")).toBeVisible()
    expect(screen.getByText("Reset credits")).toBeVisible()

    fireEvent.pointerDown(document.body)
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument()
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
