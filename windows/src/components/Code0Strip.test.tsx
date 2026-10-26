import { fireEvent, render, screen } from "@testing-library/react"
import { expect, it, vi } from "vitest"
import { FloatingStrip } from "./FloatingStrip"
import { UsageRing } from "./UsageRing"
import { unavailableSnapshots } from "../state/usage"
import { defaultStripPreferences } from "../state/stripPreferences"

it.each([["compact", 402], ["comfortable", 500]] as const)("explicit fifth %s row fits without enabling it by default", (density, height) => {
  expect(defaultStripPreferences.hiddenProviders).toEqual(["code0"])
  const activate = vi.fn()
  render(<FloatingStrip snapshots={unavailableSnapshots} activeProvider={null} onProviderActivate={activate}
    preferences={{...defaultStripPreferences, density, hiddenProviders: []}} />)
  expect(screen.getAllByRole("button", {name: /usage$/})).toHaveLength(5)
  expect(screen.getByRole("navigation").style.getPropertyValue("--strip-height")).toBe(`${height}px`)
  fireEvent.click(screen.getByRole("button", {name: "Code0 usage"}))
  expect(activate).toHaveBeenCalledWith("code0")
})

it("Code0 never publishes percent progress even if an invalid caller supplies a ratio", () => {
  const snapshot = unavailableSnapshots.find(value => value.providerId === "code0")!
  render(<UsageRing snapshot={{...snapshot, status: "fresh", usedRatio: 0.9}} onActivate={() => {}} />)
  expect(screen.getByRole("progressbar")).not.toHaveAttribute("aria-valuenow")
})
