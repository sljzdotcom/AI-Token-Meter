import type { PointerEventHandler } from "react"
import { t, useLocale, getLocale } from "../localization"

import type { UsageMetric, UsageSnapshot } from "../state/usage"
import { ProviderLogo } from "../components/ProviderLogo"
import { DeepSeekHistory, type DeepSeekHistoryStatus } from "./DeepSeekDetail"

type ProviderDetailProps = {
  snapshot: UsageSnapshot
  onPointerEnter: PointerEventHandler<HTMLElement>
  onPointerLeave: PointerEventHandler<HTMLElement>
  onInteractionStart: () => void
  onInteractionEnd: () => void
  onCheckGeminiStatus?: () => void
  onOpenGeminiDocumentation?: () => void
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
  onCheckGeminiStatus,
  onOpenGeminiDocumentation,
  deepseekHistoryStatus,
  deepseekHistoryStatusPathAvailable,
}: ProviderDetailProps) {
  useLocale()
  const percent = snapshot.usedRatio == null ? null : Math.round(snapshot.usedRatio * 100)
  return (
    <section
      aria-label={t("{name} details", {name: snapshot.displayName})}
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
            <strong>{snapshot.providerId === "gemini" ? "Gemini CLI" : snapshot.displayName}</strong>
            <small>{t(subtitle(snapshot.providerId))}</small>
          </span>
        </span>
        <strong className="provider-detail__headline">{percent == null ? t(statusLabel(snapshot.status)) : `${percent}%`}</strong>
      </header>

      <section aria-label={t("Official quota")} className="detail-section">
        <h2>{t("Official quota")}</h2>
        <div className="metric-grid">
          {snapshot.primaryMetric ? <MetricCard metric={snapshot.primaryMetric} /> : <UnavailableCard status={snapshot.status} />}
          {snapshot.secondaryMetric ? <MetricCard metric={snapshot.secondaryMetric} /> : null}
        </div>
      </section>

      {snapshot.providerId === "codex" && snapshot.resetCredits?.length ? (
        <section className="detail-section">
          <h2>{t("Reset credits")}</h2>
          {snapshot.resetCredits.map((credit) => (
            <article className="reset-credit" key={`${credit.kind}-${credit.expiresAt}`}>
              <span><strong>{t("Full usage reset")}</strong><small>{t("Expiration")}</small></span>
              <span><strong>{formatDate(credit.expiresAt)}</strong><small>{t("{count} available", {count: credit.count})}</small></span>
            </article>
          ))}
        </section>
      ) : null}

      {snapshot.localActivity ? (
        <section className="detail-section">
          <h2>{t("Last {days} days · This PC", {days: snapshot.localActivity.periodDays})}</h2>
          <div className="activity-grid">
            <Stat label={t("Tokens")} value={compact(snapshot.localActivity.tokens)} />
            <Stat label={t("Sessions")} value={String(snapshot.localActivity.sessions)} />
            <Stat label={t("Active days")} value={String(snapshot.localActivity.activeDays)} />
          </div>
        </section>
      ) : null}

      {snapshot.providerId === "deepseek" ? (
        <section className="detail-section detail-section--history">
          <h2>{t("Last 30 days · Official website")}</h2>
          <DeepSeekHistory
            onSync={onDeepSeekHistorySync}
            snapshot={snapshot}
            statusPathAvailable={deepseekHistoryStatusPathAvailable}
            syncStatus={deepseekHistoryStatus}
          />
        </section>
      ) : null}

      {snapshot.providerId === "gemini" && !snapshot.primaryMetric && <p>{t(snapshot.statusMessage ?? "Gemini CLI quota is currently unavailable. Installation and sign-in status have not been checked.")}</p>}
      {snapshot.providerId === "gemini" && <div className="service-actions">
        {onCheckGeminiStatus && <button type="button" onClick={onCheckGeminiStatus}>{t("Check Gemini status")}</button>}
        {onOpenGeminiDocumentation && <button type="button" onClick={onOpenGeminiDocumentation}>{t("Gemini CLI documentation")}</button>}
      </div>}
      <footer>{t(freshness(snapshot))} · {t("Updated")} {formatTime(snapshot.fetchedAt)}</footer>
    </section>
  )
}

function freshness(snapshot: UsageSnapshot) {
  const age = Math.max(0, (Date.now() - Date.parse(snapshot.fetchedAt)) / 1000)
  if (snapshot.status === "cached" || (snapshot.status === "fresh" && age >= snapshot.staleAfterSeconds)) {
    return t("Cached · {minutes} min ago", {minutes: Math.floor(age / 60)})
  }
  if (snapshot.status === "fresh") return "Fresh"
  if (snapshot.status === "refreshing") return "Refreshing"
  if (snapshot.status === "authenticationRequired") return "Needs sign-in"
  if (["setupRequired", "notInstalled"].includes(snapshot.status)) return "Needs setup"
  return "Unavailable"
}

function MetricCard({ metric }: { metric: UsageMetric }) {
  const fraction = metric.limit ? Math.min(metric.current / metric.limit, 1) : 0
  return (
    <article className="metric-card">
      <span><small>{t(metric.label)}</small><strong>{formatMetric(metric)}</strong></span>
      {metric.limit ? <span aria-hidden="true" className="metric-bar"><i style={{ width: `${fraction * 100}%` }} /></span> : null}
      <small>{metric.resetAt ? t("Resets {date}", {date: formatDate(metric.resetAt)}) : t(metric.resetDescription ?? "Official value")}</small>
    </article>
  )
}

function UnavailableCard({ status }: { status: UsageSnapshot["status"] }) {
  return <article className="metric-card metric-card--empty">{t(statusLabel(status))}</article>
}

function Stat({ label, value }: { label: string; value: string }) {
  return <article><strong>{value}</strong><small>{label}</small></article>
}

function subtitle(provider: UsageSnapshot["providerId"]) {
  if (provider === "gemini") return "Gemini CLI quota"
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
  return new Intl.NumberFormat(getLocale(), { notation: "compact", maximumFractionDigits: 1 }).format(value)
}

function formatDate(value: string) {
  const date = new Date(value)
  return Number.isNaN(date.valueOf()) ? value : new Intl.DateTimeFormat(getLocale(), { dateStyle: "medium", timeStyle: "short" }).format(date)
}

function formatTime(value: string) {
  const date = new Date(value)
  return Number.isNaN(date.valueOf()) ? t("recently") : new Intl.DateTimeFormat(getLocale(), { hour: "2-digit", minute: "2-digit" }).format(date)
}
