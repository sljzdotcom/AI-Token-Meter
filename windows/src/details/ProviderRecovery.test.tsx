import { fireEvent, render, screen } from "@testing-library/react"
import { expect, it, vi } from "vitest"

import { ProviderDetail } from "./ProviderDetail"
import type { UsageSnapshot } from "../state/usage"

const handlers = {
  onPointerEnter: () => {},
  onPointerLeave: () => {},
  onInteractionStart: () => {},
  onInteractionEnd: () => {},
}

function snapshot(providerId: UsageSnapshot["providerId"], status: UsageSnapshot["status"]): UsageSnapshot {
  return {
    schemaVersion: 1,
    providerId,
    displayName: providerId === "codex" ? "OpenAI Codex" : providerId === "deepseek" ? "DeepSeek" : "Claude Code",
    status,
    fetchedAt: "2026-09-09T00:00:00Z",
    staleAfterSeconds: 300,
  }
}

it("opens Services from non-Gemini actionable details", () => {
  const open = vi.fn()
  render(<ProviderDetail {...handlers} snapshot={snapshot("codex", "notInstalled")} onOpenServicesSettings={open} />)
  fireEvent.click(screen.getByRole("button", {name: "Open Services Settings"}))
  expect(open).toHaveBeenCalledOnce()
})

it("does not offer unrelated official history sync before a DeepSeek API Key is configured", () => {
  render(<ProviderDetail {...handlers} snapshot={snapshot("deepseek", "authenticationRequired")} onOpenServicesSettings={() => {}} onDeepSeekHistorySync={() => {}} />)
  expect(screen.getByRole("button", {name: "Open Services Settings"})).toBeVisible()
  expect(screen.queryByText("Sync official history")).not.toBeInTheDocument()
})

it("puts DeepSeek credential recovery before retained official history", () => {
  render(<ProviderDetail {...handlers} snapshot={{
    ...snapshot("deepseek", "authenticationRequired"),
    statusMessage: "DeepSeek API Key is required",
    dailyHistory: [{date: "2026-09-08", costCny: 1.25, requests: 4, tokens: 1000}],
  }} onOpenServicesSettings={() => {}} onDeepSeekHistorySync={() => {}} />)

  const recovery = screen.getByRole("button", {name: "Open Services Settings"})
  const history = screen.getByRole("heading", {name: "Last 30 days · Official website"})
  expect(recovery.compareDocumentPosition(history) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy()
})

it("keeps Gemini on dedicated retry and supported-version installation controls", () => {
  render(<ProviderDetail {...handlers} snapshot={{...snapshot("gemini", "notInstalled"), displayName: "Google Antigravity"}} onOpenServicesSettings={() => {}} onCheckGeminiStatus={() => {}} onOpenGeminiInstallationGuide={() => {}} />)
  expect(screen.queryByRole("button", {name: "Open Services Settings"})).not.toBeInTheDocument()
  expect(screen.getByRole("button", {name: "Check Antigravity status"})).toBeVisible()
  expect(screen.getByRole("button", {name: "Antigravity CLI installation guide"})).toBeVisible()
  expect(screen.getByText("irm https://antigravity.google/cli/install.ps1 | iex")).toBeVisible()
  expect(screen.getByText("Run agy and complete Google sign-in.")).toBeVisible()
  expect(screen.getByText("Return to AI Token Meter and choose Check Status.")).toBeVisible()
})

it("routes cached Gemini authentication to sign-in while keeping network cache passive", () => {
  const base = {
    ...snapshot("gemini", "cached"),
    displayName: "Google Antigravity",
    usedRatio: 0.25,
    geminiQuotaMetrics: [{label: "Gemini · Weekly", current: 25, limit: 100, unit: "percent" as const, kind: "officialLimit" as const, resetAt: "2026-09-17T10:00:00Z"}],
  }
  const props = {
    ...handlers,
    onCheckGeminiStatus: () => {},
    onOpenGeminiInstallationGuide: () => {},
  }
  const {rerender} = render(<ProviderDetail {...props} snapshot={{...base, statusMessage: "Cached · sign in required"}} />)
  expect(screen.getByText("Run agy and complete Google sign-in.")).toBeVisible()
  expect(screen.queryByText("irm https://antigravity.google/cli/install.ps1 | iex")).not.toBeInTheDocument()

  rerender(<ProviderDetail {...props} snapshot={{...base, statusMessage: "Cached · refresh timed out"}} />)
  expect(screen.queryByText("Run agy and complete Google sign-in.")).not.toBeInTheDocument()
  expect(screen.queryByText("irm https://antigravity.google/cli/install.ps1 | iex")).not.toBeInTheDocument()
})

it("keeps cached quota visible while exposing expired-login recovery", () => {
  const open = vi.fn()
  render(<ProviderDetail {...handlers} snapshot={{
    ...snapshot("codex", "cached"),
    usedRatio: 0.42,
    primaryMetric: {label: "Weekly limit", current: 42, limit: 100, unit: "percent", kind: "officialLimit"},
    statusMessage: "Cached · sign in required",
  }} onOpenServicesSettings={open} />)

  expect(screen.getAllByText("42%")).toHaveLength(2)
  expect(screen.getByText("Cached · sign in required")).toBeVisible()
  fireEvent.click(screen.getByRole("button", {name: "Open Services Settings"}))
  expect(open).toHaveBeenCalledOnce()
})
