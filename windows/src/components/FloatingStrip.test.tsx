import { fireEvent, render, screen } from "@testing-library/react"
import { describe, it, expect, vi } from "vitest"
import { FloatingStrip, meterContourPath } from "./FloatingStrip"
import { unavailableSnapshots } from "../state/usage"
import { UsageRing } from "./UsageRing"
import behavior from "../../../contracts/fixtures/auxiliary/strip-behavior.json"
import { defaultStripPreferences } from "../state/stripPreferences"

describe("compact floating strip interactions", () => {
  it("uses the approved rounded shoulder path at every density", () => {
    expect(behavior.expandedTransition).toEqual({
      windowFrameAnimation: false,
      contentHideDelayMilliseconds: 140,
      contentRevealDelayMilliseconds: 180,
    })
    expect(behavior.expandedContour).toMatchObject({referenceWidth: 65, compactShoulderDepth: 70, comfortableShoulderDepth: 88})
    expect(meterContourPath("mini", 4)).toContain("M 65 4 C 63 18 54 29 37 30 C 18 31 5 42 1 58 C 0 62 0 66 0 70")
    expect(meterContourPath("compact", 4)).toContain("M 78 4")
    expect(meterContourPath("comfortable", 4)).toContain("M 108 5.0285714285714285")
    expect(meterContourPath("comfortable", 4)).toContain("0 88")
  })
  it.each([
    ["mini", "left", 65, 286], ["mini", "right", 65, 286],
    ["compact", "left", 78, 286], ["compact", "right", 78, 286],
    ["comfortable", "left", 108, 356], ["comfortable", "right", 108, 356],
  ] as const)("%s/%s has undecorated draggable background and click-only providers", (density, edge, width, height) => {
    const activate = vi.fn()
    const drag = vi.fn()
    window.addEventListener("meter-drag-requested", drag)
    try {
      const { container } = render(<div className={`meter-edge--${edge}`}>
        <FloatingStrip snapshots={unavailableSnapshots} activeProvider={null} onProviderActivate={activate}
          preferences={{...defaultStripPreferences, density, hiddenProviders: ["gemini"]}} />
      </div>)
      const strip = screen.getByRole("navigation")
      expect(strip.style.getPropertyValue("--strip-width")).toBe(`${width}px`)
      expect(strip.style.getPropertyValue("--strip-height")).toBe(`${height}px`)
      if (density === "mini") expect((width - 48) / 2).toBe(8.5)
      if (density === "compact") expect((width - 48) / 2).toBe(15)
      // Dispatch pointerdown with a real button value; jsdom lacks PointerEvent.
      fireEvent(strip, new MouseEvent("pointerdown", { bubbles: true, button: 0 }))
      expect(drag).toHaveBeenCalledTimes(1)
      for (const [index, button] of screen.getAllByRole("button", {name: /usage$/}).entries()) {
        fireEvent(button, new MouseEvent("pointerdown", { bubbles: true, button: 0 }))
        fireEvent.click(button)
        expect(activate).toHaveBeenNthCalledWith(index + 1, defaultStripPreferences.orderedProviders[index])
      }
      expect(activate).toHaveBeenCalledTimes(3)
      expect(drag).toHaveBeenCalledTimes(1)
      expect(container.querySelector(".floating-strip__drag-handle")).toBeNull()
    } finally {
      window.removeEventListener("meter-drag-requested", drag)
    }
  })
  it.each(behavior.densities)("matches shared $id dimensions", density => {
    render(<FloatingStrip snapshots={unavailableSnapshots} activeProvider={null} onProviderActivate={() => {}}
      preferences={{...defaultStripPreferences, density: density.id as "mini" | "compact" | "comfortable", hiddenProviders: ["gemini"]}} />)
    const style = screen.getByRole("navigation").style
    expect(style.getPropertyValue("--strip-width")).toBe(`${density.width}px`)
    expect(style.getPropertyValue("--strip-height")).toBe(`${density.height}px`)
    expect(style.getPropertyValue("--strip-ring")).toBe(`${density.ring}px`)
  })
  it("cached authentication failures retain quota and expose an action", () => {
    render(<UsageRing snapshot={{...unavailableSnapshots[0], status: "cached", usedRatio: 0.25, statusMessage: "Cached · sign in required"}} onActivate={() => {}} />)
    expect(screen.getByRole("progressbar")).toHaveAttribute("aria-valuenow", "25")
    expect(screen.getByRole("button")).toHaveAccessibleDescription("Action required")
  })

  it("cached DeepSeek authentication failures use the same action-required signal", () => {
    render(<UsageRing snapshot={{...unavailableSnapshots[2], status: "cached", usedRatio: 0.25, statusMessage: "Cached · authentication required"}} onActivate={() => {}} />)

    expect(screen.getByRole("button")).toHaveAccessibleDescription("Action required")
  })
  it("refreshing keeps the last quota while announcing the separate operation", () => {
    render(<UsageRing snapshot={{...unavailableSnapshots[0], status: "refreshing", usedRatio: 0.25}} onActivate={() => {}} />)
    expect(screen.getByRole("progressbar")).toHaveAttribute("aria-valuenow", "25")
    expect(screen.getByRole("button")).toHaveAccessibleDescription("Refreshing")
  })
  it("renders only selected services in configured order", () => {
    render(<FloatingStrip snapshots={unavailableSnapshots} activeProvider={null} onProviderActivate={() => {}}
      preferences={{...defaultStripPreferences, density: "compact", orderedProviders: ["deepseek", "codex", "claude"], hiddenProviders: ["codex"], hiddenUntil: null }} />)
    expect(screen.getAllByRole("button", {name: /usage$/}).map(b => b.getAttribute("aria-label"))).toEqual(["DeepSeek usage", "Claude Code usage"])
  })
  it("right clicking never starts dragging", () => {
    const drag = vi.fn()
    window.addEventListener("meter-drag-requested", drag)
    render(<FloatingStrip snapshots={unavailableSnapshots} activeProvider={null} onProviderActivate={() => {}} />)
    fireEvent.pointerDown(screen.getByRole("navigation"), { button: 2 })
    expect(drag).not.toHaveBeenCalled()
    window.removeEventListener("meter-drag-requested", drag)
  })
  it("folded handle exposes expansion instead of provider buttons", () => {
    const interaction = vi.fn()
    render(<FloatingStrip snapshots={unavailableSnapshots} activeProvider={null} onProviderActivate={() => {}} folded onInteraction={interaction} />)
    expect(screen.queryByRole("button", {name: /Claude/})).not.toBeInTheDocument()
    const expand = screen.getByRole("button", {name: "Expand floating meter"})
    expect(expand.querySelector("span")).toBeInTheDocument()
    fireEvent.click(expand)
    expect(interaction).toHaveBeenCalledWith("pointer", true)
  })

  it("does not render a bottom Settings entry at rest, hover, or focus", () => {
    render(<FloatingStrip snapshots={unavailableSnapshots} activeProvider={null} onProviderActivate={() => {}} />)
    const strip = screen.getByRole("navigation")
    expect(screen.queryByRole("button", {name: "Settings"})).not.toBeInTheDocument()
    expect(strip.querySelector(".floating-strip__settings-zone")).toBeNull()
    fireEvent.pointerEnter(strip)
    fireEvent.focus(strip)
    expect(screen.queryByRole("button", {name: "Settings"})).not.toBeInTheDocument()
  })
})
