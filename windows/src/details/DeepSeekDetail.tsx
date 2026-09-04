import type { UsageSnapshot } from "../state/usage"

export type DeepSeekHistoryStatus = "idle" | "opening" | "active" | "completed" | "cancelled" | "failed"

export function DeepSeekHistory({
  snapshot,
  onSync,
  syncStatus = "idle",
  statusPathAvailable = true,
}: {
  snapshot: UsageSnapshot
  onSync?: () => void
  syncStatus?: DeepSeekHistoryStatus
  statusPathAvailable?: boolean
}) {
  const history = snapshot.dailyHistory ?? []
  if (!history.length) {
    const syncing = syncStatus === "opening" || syncStatus === "active"
    const message = !statusPathAvailable
      ? "Official history status is temporarily unavailable."
      : syncStatus === "opening"
      ? "Opening official page…"
      : syncStatus === "active"
        ? "Sync in progress"
        : syncStatus === "failed"
          ? "Official history sync could not be started. Try again."
          : "Usage history will appear here after official-page sync."
    return (
      <div className="history-placeholder">
        <span role={!statusPathAvailable || syncStatus === "failed" ? "alert" : syncing ? "status" : undefined}>{message}</span>
        {onSync ? (
          <button disabled={syncing || !statusPathAvailable} onClick={onSync} type="button">
            {syncStatus === "failed" ? "Try again" : "Sync official history"}
          </button>
        ) : null}
      </div>
    )
  }

  const totalCost = history.reduce((total, day) => total + day.costCny, 0)
  const totalRequests = history.reduce((total, day) => total + day.requests, 0)
  const totalTokens = history.reduce((total, day) => total + day.tokens, 0)
  const maximumCost = Math.max(...history.map((day) => day.costCny), 0.01)

  return (
    <div className="deepseek-history">
      <div className="history-summary">
        <HistoryStat label="Cost" value={`¥${totalCost.toFixed(2)}`} />
        <HistoryStat label="Requests" value={new Intl.NumberFormat("en").format(totalRequests)} />
        <HistoryStat label="Tokens" value={compact(totalTokens)} />
      </div>
      <div aria-label="DeepSeek cost for the last 30 days" className="history-chart" role="img">
        {history.map((day) => (
          <span
            aria-label={`${day.date}: ¥${day.costCny.toFixed(2)}`}
            className="history-chart__day"
            key={day.date}
            title={`${day.date} · ¥${day.costCny.toFixed(2)}`}
          >
            <i style={{ height: `${Math.max((day.costCny / maximumCost) * 100, 3)}%` }} />
          </span>
        ))}
      </div>
      <small className="history-source">
        Official website · Updated {formatTime(snapshot.historyFetchedAt ?? snapshot.fetchedAt)}
      </small>
    </div>
  )
}

function HistoryStat({ label, value }: { label: string; value: string }) {
  return <article><strong>{value}</strong><small>{label}</small></article>
}

function compact(value: number) {
  return new Intl.NumberFormat("en", { notation: "compact", maximumFractionDigits: 1 }).format(value)
}

function formatTime(value: string) {
  const date = new Date(value)
  return Number.isNaN(date.valueOf())
    ? "recently"
    : new Intl.DateTimeFormat("en", { dateStyle: "medium", timeStyle: "short" }).format(date)
}
