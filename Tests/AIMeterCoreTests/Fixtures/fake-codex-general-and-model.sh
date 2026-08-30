#!/bin/sh

if [ "$1" != "app-server" ]; then
  exit 2
fi

IFS= read -r ai_meter_initialize
printf '{"id":1,"result":{"userAgent":"fake-codex"}}\n'
IFS= read -r ai_meter_initialized
IFS= read -r ai_meter_rate_request
printf '%s\n' '{"id":2,"result":{"rateLimits":{"limitId":"codex","primary":{"usedPercent":5,"windowDurationMins":10080,"resetsAt":1788652811},"secondary":null},"rateLimitsByLimitId":{"codex_bengalfox":{"limitId":"codex_bengalfox","primary":{"usedPercent":0,"windowDurationMins":300,"resetsAt":1788093543},"secondary":{"usedPercent":0,"windowDurationMins":10080,"resetsAt":1788680343}}}}}'
