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

it("renders all four official Antigravity windows, source version, and reset times", () => {
  render(<ProviderDetail {...handlers} snapshot={fresh as UsageSnapshot} />)
  for (const label of [
    "Gemini · Five hour",
    "Gemini · Weekly",
    "Claude/GPT · Five hour",
    "Claude/GPT · Weekly",
  ]) {
    expect(screen.getByText(label)).toBeVisible()
  }
  expect(screen.getByText("20% remaining")).toBeVisible()
  expect(screen.getByText("40% remaining")).toBeVisible()
  expect(screen.getByText("75% remaining")).toBeVisible()
  expect(screen.getByText("80% remaining")).toBeVisible()
  expect(screen.getByText(/Antigravity CLI 1.1.28.*\/usage/)).toBeVisible()
  expect(screen.getAllByText(/^Resets /)).toHaveLength(4)
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
  expect(screen.getByText("Claude/GPT · Weekly")).toBeVisible()
})
