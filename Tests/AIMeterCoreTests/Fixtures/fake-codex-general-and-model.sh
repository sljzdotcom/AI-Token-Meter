#!/bin/sh

if [ "$1" != "app-server" ]; then
  exit 2
fi

IFS= read -r ai_meter_initialize
printf '{"id":1,"result":{"userAgent":"fake-codex"}}\n'
IFS= read -r ai_meter_initialized
IFS= read -r ai_meter_rate_request
printf '%s\n' '{"id":2,"result":{"rateLimits":{"limitId":"codex","primary":{"usedPercent":5,"windowDurationMins":10080,"resetsAt":1788652811},"secondary":null},"rateLimitsByLimitId":{"codex_bengalfox":{"limitId":"codex_bengalfox","primary":{"usedPercent":0,"windowDurationMins":300,"resetsAt":1788093543},"secondary":{"usedPercent":0,"windowDurationMins":10080,"resetsAt":1788680343}}},"rateLimitResetCredits":{"availableCount":2,"credits":[{"id":"discard-me-1","status":"available","resetType":"codexRateLimits","grantedAt":1890000000,"title":"Bonus reset","description":null,"expiresAt":1900000000},{"id":"discard-me-2","status":"available","resetType":"codexRateLimits","grantedAt":1890000001,"title":null,"description":null,"expiresAt":1900100000},{"id":"discard-me-3","status":"redeemed","resetType":"codexRateLimits","grantedAt":1890000002,"title":"Used","description":null,"expiresAt":1900200000}]}}}'
