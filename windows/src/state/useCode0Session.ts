import { useEffect } from "react"
import { disabledCode0SessionBridge, emptyCode0Session, type Code0SessionBridge } from "./code0Session"

export function useCode0Session(bridge: Code0SessionBridge = disabledCode0SessionBridge) {
  useEffect(() => {
    let disposed = false
    let unlisten: (() => void) | undefined
    // Read/listen lifecycle only; this first checkpoint intentionally admits no
    // projection yet. Display never calls perform or obtains website authority.
    bridge.onUpdated(() => {}).then(stop => {
      if (disposed) stop()
      else unlisten = stop
    }).catch(() => {})
    bridge.read().catch(() => {})
    return () => { disposed = true; unlisten?.() }
  }, [bridge])
  return emptyCode0Session
}
