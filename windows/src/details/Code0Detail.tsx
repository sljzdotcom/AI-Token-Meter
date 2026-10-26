import type { Code0SessionDTO } from "../state/code0Session"

// Display-only API: no transport/action callback, mount task or focus action.
// The first RED surface has no numeric rendering until behavior RED is confirmed.
export function Code0Detail(_props: {session: Code0SessionDTO}) {
  return <section role="dialog" aria-label="Code0 details"><h2>Code0</h2></section>
}
