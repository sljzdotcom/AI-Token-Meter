import type { Code0Action, Code0ActionResult, Code0SessionDTO } from "../state/code0Session"

export type SettingsCode0Props = {
  session: Code0SessionDTO
  onAction: (action: Code0Action) => Promise<Code0ActionResult>
}

// Deny/no-op RED surface. Rendering and focus cannot invoke a connection.
export function SettingsCode0(_props: SettingsCode0Props) {
  return <section aria-label="Code0"><h2>Code0</h2></section>
}
