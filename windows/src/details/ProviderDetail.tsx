import type { PointerEventHandler } from "react"

import type { UsageMetric, UsageSnapshot } from "../state/usage"
import { ProviderLogo } from "../components/ProviderLogo"
import { DeepSeekHistory, type DeepSeekHistoryStatus } from "./DeepSeekDetail"

type ProviderDetailProps = {
  snapshot: UsageSnapshot
  onPointerEnter: PointerEventHandler<HTMLElement>
  onPointerLeave: PointerEventHandler<HTMLElement>
  onInteractionStart: () => void
  onInteractionEnd: () => void
  onDeepSeekHistorySync?: () => void
  deepseekHistoryStatus?: DeepSeekHistoryStatus
  deepseekHistoryStatusPathAvailable?: boolean
}

export function ProviderDetail({
  snapshot,
  onPointerEnter,
  onPointerLeave,
  onInteractionStart,
  onInteractionEnd,
  onDeepSeekHistorySync,
  deepseekHistoryStatus,
  deepseekHistoryStatusPathAvailable,
}: ProviderDetailProps) {
  const percent = snapshot.usedRatio == null ? null : Math.round(snapshot.usedRatio * 100)
  return (
    <section
      aria-label={`${snapshot.displayName} details`}
      className={`provider-detail provider-detail--compact-density provider-detail--${snapshot.providerId}`}
      onBlur={(event) => {
        if (!event.currentTarget.contains(event.relatedTarget)) onInteractionEnd()
      }}
      onFocus={onInteractionStart}
      onPointerDown={(event) => event.stopPropagation()}
      onPointerEnter={onPointerEnter}
      onPointerLeave={onPointerLeave}
      role="dialog"
      tabIndex={-1}
    >
      <header className="provider-detail__header">
        <span className="provider-detail__identity">
          <span className="provider-detail__logo"><ProviderLogo provider={snapshot.providerId} /></span>
          <span>
            <strong>{snapshot.displayName}</strong>
            <small>{subtitle(snapshot.providerId)}</small>
          </span>
        </span>
        <strong className="provider-detail__headline">{percent == null ? statusLabel(snapshot.status) : `${percent}%`}</strong>
      </header>

      <section aria-label="Official quota" className="detail-section">
        <h2>Official quota</h2>
        <div className="metric-grid">
          {snapshot.primaryMetric ? <MetricCard metric={snapshot.primaryMetric} /> : <UnavailableCard status={snapshot.status} />}
          {snapshot.secondaryMetric ? <MetricCard metric={snapshot.secondaryMetric} /> : null}
        </div>
      </section>

      {snapshot.providerId === "codex" && snapshot.resetCredits?.length ? (
        <section className="detail-section">
          <h2>Reset credits</h2>
          {snapshot.resetCredits.map((credit) => (
            <article className="reset-credit" key={`${credit.kind}-${credit.expiresAt}`}>
              <span><strong>Full usage reset</strong><small>Expiration</small></span>
              <span><strong>{formatDate(credit.expiresAt)}</strong><small>{credit.count} available</small></span>
            </article>
          ))}
        </section>
      ) : null}

      {snapshot.localActivity ? (
        <section className="detail-section">
          <h2>Last {snapshot.localActivity.periodDays} days · This PC</h2>
          <div className="activity-grid">
            <Stat label="Tokens" value={compact(snapshot.localActivity.tokens)} />
            <Stat label="Sessions" value={String(snapshot.localActivity.sessions)} />
            <Stat label="Active days" value={String(snapshot.localActivity.activeDays)} />
          </div>
        </section>
      ) : null}

      {snapshot.providerId === "deepseek" ? (
        <section className="detail-section detail-section--history">
          <h2>Last 30 days · Official website</h2>
          <DeepSeekHistory
            onSync={onDeepSeekHistorySync}
            snapshot={snapshot}
            statusPathAvailable={deepseekHistoryStatusPathAvailable}
            syncStatus={deepseekHistoryStatus}
          />
        </section>
      ) : null}

      <footer>Updated {formatTime(snapshot.fetchedAt)}</footer>
    </section>
  )
}

function MetricCard({ metric }: { metric: UsageMetric }) {
  const fraction = metric.limit ? Math.min(metric.current / metric.limit, 1) : 0
  return (
    <article className="metric-card">
      <span><small>{metric.label}</small><strong>{formatMetric(metric)}</strong></span>
      {metric.limit ? <span aria-hidden="true" className="metric-bar"><i style={{ width: `${fraction * 100}%` }} /></span> : null}
      <small>{metric.resetDescription ?? (metric.resetAt ? `Resets ${formatDate(metric.resetAt)}` : "Official value")}</small>
    </article>
  )
}

function UnavailableCard({ status }: { status: UsageSnapshot["status"] }) {
  return <article className="metric-card metric-card--empty">{statusLabel(status)}</article>
}

function Stat({ label, value }: { label: string; value: string }) {
  return <article><strong>{value}</strong><small>{label}</small></article>
}

function subtitle(provider: UsageSnapshot["providerId"]) {
  if (provider === "deepseek") return "Official balance · API usage"
  if (provider === "codex") return "Official quota · Local OpenAI Codex activity"
  return "Official quota · Local Claude Code activity"
}

function statusLabel(status: UsageSnapshot["status"]) {
  return ({
    fresh: "Available",
    cached: "Cached",
    refreshing: "Refreshing",
    notInstalled: "Not installed",
    authenticationRequired: "Sign in required",
    setupRequired: "Setup required",
    unavailable: "Unavailable",
    unrecognizedOutput: "Format changed",
  } as const)[status]
}

function formatMetric(metric: UsageMetric) {
  if (metric.unit === "percent") return `${Math.round(metric.current)}%`
  if (metric.unit === "cny") return `¥${metric.current.toFixed(2)}`
  return compact(metric.current)
}

function compact(value: number) {
  return new Intl.NumberFormat("en", { notation: "compact", maximumFractionDigits: 1 }).format(value)
}

function formatDate(value: string) {
  const date = new Date(value)
  return Number.isNaN(date.valueOf()) ? value : new Intl.DateTimeFormat("en", { dateStyle: "medium", timeStyle: "short" }).format(date)
}

function formatTime(value: string) {
  const date = new Date(value)
  return Number.isNaN(date.valueOf()) ? "recently" : new Intl.DateTimeFormat("en", { hour: "2-digit", minute: "2-digit" }).format(date)
}
