import { useEffect, useState } from "react"
import { decodeCode0Session, disabledCode0SessionBridge, emptyCode0Session, type Code0SessionBridge, type Code0SessionDTO } from "./code0Session"

type Projection = {bridge: Code0SessionBridge; value: Code0SessionDTO}

export function useCode0Session(bridge: Code0SessionBridge = disabledCode0SessionBridge) {
  const [projection, setProjection] = useState<Projection | null>(null)
  useEffect(() => {
    let disposed = false
    let unlisten: (() => void) | undefined
    const accept = (raw: unknown) => {
      if (disposed) return
      const value = decodeCode0Session(raw)
      if (!value) return
      setProjection(current => {
        if (disposed || current?.bridge === bridge && value.revision <= current.value.revision) return current
        return {bridge, value}
      })
    }
    const attach = async () => {
      try {
        const stop = await bridge.onUpdated(accept)
        if (disposed) { stop(); return }
        unlisten = stop
      } catch { /* A read may still show the last committed projection. */ }
      if (disposed) return
      // Subscribe before the initial read, then arbitrate both with one revision
      // gate. A failed read never clears an already accepted publication.
      try { accept(await bridge.read()) } catch { /* Keep current B, including null. */ }
    }
    void attach()
    return () => { disposed = true; unlisten?.() }
  }, [bridge])
  // A replacement bridge starts cold immediately; an old runtime's value or
  // queued response cannot appear while its replacement is still being read.
  return projection?.bridge === bridge ? projection.value : emptyCode0Session
}
