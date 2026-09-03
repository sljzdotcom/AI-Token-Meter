const argumentsAfterScript = process.argv.slice(2);

if (argumentsAfterScript[0] === "auth" && argumentsAfterScript[1] === "status") {
  process.stdout.write("Logged in\n");
  process.exit(0);
}

if (argumentsAfterScript.includes("--ax-screen-reader")) {
  process.stdin.setEncoding("utf8");
  process.stdout.write("Claude Code ready\r\n");
  process.stdin.once("data", (input) => {
    if (input.includes("/usage")) {
      process.stdout.write([
        "Current session",
        "23% used",
        "Resets in 3 hr 42 min",
        "Current week (all models)",
        "5% used",
        "Resets Sep 6 at 8:00am",
        "",
      ].join("\r\n"));
      setTimeout(() => process.exit(0), 100);
    }
  });
  return;
}

process.stderr.write("unsupported fixture arguments");
process.exit(2);
