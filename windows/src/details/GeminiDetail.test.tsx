import { render, screen } from "@testing-library/react"
import { expect, it } from "vitest"

import fresh from "../../../contracts/fixtures/gemini-fresh.json"
import type { UsageSnapshot } from "../state/usage"
import { ProviderDetail } from "./ProviderDetail"

const handlers = {
  onPointerEnter: () => {},
  onPointerLeave: () => {},
  onInteractionStart: () => {},
  onInteractionEnd: () => {},
}

it("renders Gemini quota and compact Antigravity CLI information", () => {
  render(<ProviderDetail {...handlers} snapshot={fresh as UsageSnapshot} />)
  for (const label of ["Gemini · Five hour", "Gemini · Weekly"]) {
    expect(screen.getByText(label)).toBeVisible()
  }
  expect(screen.getByText("40% remaining")).toBeVisible()
  expect(screen.getByText("75% remaining")).toBeVisible()
  expect(screen.getByRole("heading", {name: "Claude/GPT shared quota"})).toBeVisible()
  expect(screen.queryByText(/AI Credits/)).not.toBeInTheDocument()
  expect(screen.getByText("Antigravity CLI")).toBeVisible()
  expect(screen.getByText("Current model")).toBeVisible()
  expect(screen.getByText("Gemini 3.8 Flash (High)")).toBeVisible()
  expect(screen.getByText("Available models")).toBeVisible()
  expect(screen.getByText("4 models · 3 families")).toBeVisible()
  expect(screen.getByText("Gemini 3.8 Flash · Gemini 3.7 Flash · Gemini 3.1 Pro")).toBeVisible()
  expect(screen.getByText("CLI version")).toBeVisible()
  expect(screen.getByText("1.2.2")).toBeVisible()
  expect(screen.getAllByText(/^Resets /)).toHaveLength(4)
})

it("renders Antigravity's shared Claude/GPT quota independently from Gemini", () => {
  render(<ProviderDetail {...handlers} snapshot={fresh as UsageSnapshot} />)
  expect(screen.getByText("Claude/GPT · Five hour")).toBeVisible()
  expect(screen.getByText("20% remaining")).toBeVisible()
  expect(screen.getByText("Claude/GPT · Weekly")).toBeVisible()
  expect(screen.getByText("80% remaining")).toBeVisible()
  expect(screen.getByText("60%")).toBeVisible()
})

it("cached failures retain quota and show the recovery reason", () => {
  render(
    <ProviderDetail
      {...handlers}
      snapshot={{ ...fresh, status: "cached", statusMessage: "Cached · sign in required" } as UsageSnapshot}
    />,
  )
  expect(screen.getByText("Cached · sign in required")).toBeVisible()
  expect(screen.getByText("Gemini · Weekly")).toBeVisible()
  expect(screen.getByRole("heading", {name: "Claude/GPT shared quota"})).toBeVisible()
})

it("keeps quota usable when optional CLI model information is unavailable", () => {
  const { antigravityCLIInfo: _, ...withoutCLIInfo } = fresh
  render(<ProviderDetail {...handlers} snapshot={withoutCLIInfo as UsageSnapshot} />)
  expect(screen.getByText("Gemini · Weekly")).toBeVisible()
  expect(screen.getByText("CLI version")).toBeVisible()
  expect(screen.getByText("1.2.2")).toBeVisible()
  expect(screen.queryByText("Current model")).not.toBeInTheDocument()
})

it("hides the shared quota section when the CLI reports no shared model pool", () => {
  const { antigravitySharedQuotaMetrics: _, ...withoutSharedQuota } = fresh
  render(<ProviderDetail {...handlers} snapshot={withoutSharedQuota as UsageSnapshot} />)
  expect(screen.queryByRole("heading", {name: "Claude/GPT shared quota"})).not.toBeInTheDocument()
  expect(screen.getByText("Gemini · Weekly")).toBeVisible()
})
