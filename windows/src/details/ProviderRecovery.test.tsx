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

it("keeps Gemini on its dedicated retry and documentation controls", () => {
  render(<ProviderDetail {...handlers} snapshot={{...snapshot("gemini", "notInstalled"), displayName: "Gemini"}} onOpenServicesSettings={() => {}} onCheckGeminiStatus={() => {}} onOpenGeminiDocumentation={() => {}} />)
  expect(screen.queryByRole("button", {name: "Open Services Settings"})).not.toBeInTheDocument()
  expect(screen.getByRole("button", {name: "Check Gemini status"})).toBeVisible()
  expect(screen.getByRole("button", {name: "Gemini CLI documentation"})).toBeVisible()
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
