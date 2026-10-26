import { useEffect, useRef, useState } from "react"
import { useLocale } from "../localization"
import { ProviderLogo } from "../components/ProviderLogo"
import type { Code0Action, Code0ActionResult, Code0SessionDTO } from "../state/code0Session"
import { code0ActionLabel, code0ActionMessage, code0SessionMessage, code0Text, code0WaitMessage, formatCode0Time } from "../state/code0Presentation"

export type SettingsCode0Props = {
  session: Code0SessionDTO
  onAction: (action: Code0Action) => Promise<Code0ActionResult>
}
type Operation = {handler: SettingsCode0Props["onAction"]; revision: number}
type Reply = Operation & {result: Code0ActionResult}

export function SettingsCode0({session, onAction}: SettingsCode0Props) {
  useLocale()
  const [pending, setPending] = useState<Operation | null>(null)
  const [reply, setReply] = useState<Reply | null>(null)
  const operation = useRef<Operation | null>(null)
  const mounted = useRef(true)
  const current = useRef({session, onAction})
  current.current = {session, onAction}
  useEffect(() => {
    mounted.current = true
    return () => { mounted.current = false; operation.current = null }
  }, [])
  const isPending = pending?.handler === onAction
  const replyMessage = reply?.handler === onAction && reply.revision === session.revision ? code0ActionMessage(reply.result) : null
  const waitMessage = code0WaitMessage(session.retryAfterSeconds)

  const perform = async (action: Code0Action) => {
    if (operation.current?.handler === onAction || !canPerform(action, session)) return
    const attempt: Operation = {handler: onAction, revision: session.revision}
    operation.current = attempt
    setPending(attempt)
    setReply(null)
    try {
      const result = await onAction(action)
      if (mounted.current && operation.current === attempt && current.current.onAction === attempt.handler
        && current.current.session.revision === attempt.revision) setReply({...attempt, result})
    } catch {
      if (mounted.current && operation.current === attempt && current.current.onAction === attempt.handler
        && current.current.session.revision === attempt.revision) setReply({...attempt, result: {kind: "denied"}})
    } finally {
      if (operation.current === attempt) {
        operation.current = null
        if (mounted.current) setPending(value => value === attempt ? null : value)
      }
    }
  }

  return <section aria-label="Code0" className="service-card">
    <span className="service-identity">
      <strong><ProviderLogo provider="code0" /> Code0</strong>
      <small role="status">{code0SessionMessage(session)}</small>
      {session.snapshot ? <small>{code0Text("Last successful sync:")} <time dateTime={session.snapshot.fetchedAt}>{formatCode0Time(session.snapshot.fetchedAt)}</time></small> : null}
    </span>
    <div className="service-actions">
      {actionsFor(session).map(action => <button key={action} type="button" disabled={isPending || !canPerform(action, session)}
        onClick={() => { void perform(action) }}>{code0ActionLabel(action)}</button>)}
      {waitMessage || replyMessage ? <small role="status">{waitMessage ?? replyMessage}</small> : null}
      <small>{code0Text("Sync attempts are at least five minutes apart. A failed attempt pauses automatic sync.")}</small>
      <small>{code0Text("Website balance and cumulative site credits use this dedicated Code0 connection.")}</small>
    </div>
  </section>
}

function actionsFor(session: Code0SessionDTO): Code0Action[] {
  switch (session.state) {
    case "disconnected": return ["connect"]
    case "connecting": return ["loginCompleted", "disconnect"]
    case "paused": case "active": return session.snapshot ? ["syncNow", "disconnect"] : ["connect", "disconnect"]
    case "syncing": return ["syncNow", "disconnect"]
    case "closing": case "cleanupPending": return ["connect"]
  }
}
function canPerform(action: Code0Action, session: Code0SessionDTO) {
  switch (action) {
    case "connect": return session.state === "disconnected" || session.state === "paused"
    case "loginCompleted": return session.state === "connecting"
    case "syncNow": return (session.state === "paused" || session.state === "active") && !(session.retryAfterSeconds !== null && session.retryAfterSeconds > 0)
    case "disconnect": return ["connecting", "syncing", "paused", "active"].includes(session.state)
  }
}
