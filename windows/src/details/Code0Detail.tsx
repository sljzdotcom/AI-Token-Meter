import { ProviderLogo } from "../components/ProviderLogo"
import { t, useLocale } from "../localization"
import type { Code0SessionDTO } from "../state/code0Session"
import { code0SessionMessage, code0Text, formatCode0Credits, formatCode0Time } from "../state/code0Presentation"

// Display-only API: no transport/action callback, mount task or focus action.
export function Code0Detail({session}: {session: Code0SessionDTO}) {
  useLocale()
  const snapshot = session.snapshot
  return <section role="dialog" aria-label={t("{name} details", {name: "Code0"})}
    className="provider-detail provider-detail--compact-density provider-detail--code0" tabIndex={-1}>
    <header className="provider-detail__header">
      <span className="provider-detail__identity">
        <span className="provider-detail__logo"><ProviderLogo provider="code0" /></span>
        <span><strong>Code0</strong><small>{t("Official website · Site credits")}</small></span>
      </span>
    </header>
    {snapshot ? <>
      <div className="metric-grid">
        <article className="metric-card"><span>{t("Balance")}</span><strong>{formatCode0Credits(snapshot.primaryMetric.current)}</strong></article>
        <article className="metric-card"><span>{t("Cumulative usage")}</span><strong>{formatCode0Credits(snapshot.secondaryMetric.current)}</strong></article>
      </div>
      <footer>{code0Text("Last successful sync:")} <time dateTime={snapshot.fetchedAt}>{formatCode0Time(snapshot.fetchedAt)}</time></footer>
    </> : <p>{code0Text("Connect Code0 in Services to view website usage.")}</p>}
    <p role="status">{code0SessionMessage(session)}</p>
  </section>
}
