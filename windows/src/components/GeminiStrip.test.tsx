import { fireEvent, render, screen } from "@testing-library/react"
import { expect, it } from "vitest"
import { FloatingStrip } from "./FloatingStrip"
import { ProviderDetail } from "../details/ProviderDetail"
import { defaultStripPreferences } from "../state/stripPreferences"
import { unavailableSnapshots } from "../state/usage"

it.each([['mini','left',65,344],['mini','right',65,344],['compact','left',78,344],['compact','right',78,344],['comfortable','left',108,428],['comfortable','right',108,428]] as const)("four %s/%s buttons fit the same native size and remain selectable", (density,edge,width,height) => {
  let selected = ''
  const {container} = render(<div className={`meter-edge--${edge}`}><FloatingStrip snapshots={unavailableSnapshots} activeProvider={null} onProviderActivate={id => {selected=id}} preferences={{...defaultStripPreferences,density}} /></div>)
  const nav = screen.getByRole('navigation')
  expect(nav.style.getPropertyValue('--strip-width')).toBe(`${width}px`)
  expect(nav.style.getPropertyValue('--strip-height')).toBe(`${height}px`)
  expect(screen.getAllByRole('button', {name: /usage$/})).toHaveLength(4)
  fireEvent.click(screen.getByRole('button',{name:'Google Antigravity usage'}))
  expect(selected).toBe('gemini')
  expect(screen.getByRole('progressbar',{name:'Google Antigravity usage'})).not.toHaveAttribute('aria-valuenow')
  expect(screen.getByRole('button',{name:'Google Antigravity usage'}).style.getPropertyValue('--provider-accent')).toBe('#3ED6B2')
  expect(container.querySelector('.floating-strip__drag-handle')).toBeNull()
})

it('Gemini detail identifies CLI scope and unavailable quota without Claude labels or percentages', () => {
  const snapshot = unavailableSnapshots.find(s => s.providerId === 'gemini')!
  render(<ProviderDetail snapshot={snapshot} onPointerEnter={()=>{}} onPointerLeave={()=>{}} onInteractionStart={()=>{}} onInteractionEnd={()=>{}} />)
  expect(screen.getByText('Google Antigravity')).toBeVisible()
  expect(screen.queryByText(/Local Claude/)).not.toBeInTheDocument()
  expect(screen.queryByText(/0%/)).not.toBeInTheDocument()
  expect(screen.getByText(/Installation and sign-in status have not been checked/)).toBeVisible()
})
