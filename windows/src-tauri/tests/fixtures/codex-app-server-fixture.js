import readline from "node:readline";

const lines = readline.createInterface({ input: process.stdin, crlfDelay: Infinity });
lines.on("line", (line) => {
  const message = JSON.parse(line);
  if (message.method === "initialize") {
    process.stdout.write(`${JSON.stringify({ id: message.id, result: { userAgent: "fixture" } })}\n`);
    return;
  }
  if (message.method === "account/read") {
    process.stdout.write(`${JSON.stringify({ method: "unrelated/notification", params: {} })}\n`);
    process.stdout.write(`${JSON.stringify({
      id: message.id,
      result: {
        account: { type: "chatgpt", email: "private@example.com", planType: "pro" },
        requiresOpenaiAuth: true,
      },
    })}\n`);
    return;
  }
  if (message.method === "account/rateLimits/read") {
    process.stdout.write(`${JSON.stringify({
      id: message.id,
      result: {
        rateLimits: {
          primary: { usedPercent: 17, windowDurationMins: 300, resetsAt: 1788422400 },
          secondary: { usedPercent: 4, windowDurationMins: 10080, resetsAt: 1788652800 },
        },
        rateLimitResetCredits: { availableCount: 0, credits: [] },
      },
    })}\n`);
  }
});
