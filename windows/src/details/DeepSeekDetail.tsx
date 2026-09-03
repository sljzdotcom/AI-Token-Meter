import type { UsageSnapshot } from "../state/usage"

export function DeepSeekHistory({ snapshot, onSync }: { snapshot: UsageSnapshot; onSync?: () => void }) {
  const history = snapshot.dailyHistory ?? []
  if (!history.length) {
    return (
      <div className="history-placeholder">
        <span>Usage history will appear here after official-page sync.</span>
        {onSync ? <button onClick={onSync} type="button">Sync official history</button> : null}
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
