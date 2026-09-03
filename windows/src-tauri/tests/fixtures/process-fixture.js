import fs from "node:fs";
import { spawn } from "node:child_process";

const mode = process.argv[2];

switch (mode) {
  case "arguments":
    process.stdout.write(JSON.stringify(process.argv.slice(3)));
    break;
  case "flood":
    process.stdout.write("x".repeat(256 * 1024));
    break;
  case "sleep":
    setTimeout(() => process.stdout.write("late"), 10_000);
    break;
  case "encoded": {
    const text = "\u001b[31m安全输出\u001b[0m\r\n";
    process.stdout.write(Buffer.concat([Buffer.from([0xff, 0xfe]), Buffer.from(text, "utf16le")]));
    break;
  }
  case "stdin":
    process.stdin.setEncoding("utf8");
    process.stdout.write("ready\r\n");
    process.stdin.once("data", (input) => {
      process.stdout.write(`received:${input.trim()}\r\n`);
      process.exit(0);
    });
    break;
  case "spawn-child": {
    const sentinel = process.argv[3];
    setTimeout(() => {
      spawn(process.execPath, [__filename, "child-write", sentinel], {
        detached: false,
        stdio: "ignore",
      });
      setTimeout(() => process.exit(0), 50);
    }, 100);
    break;
  }
  case "child-write":
    setTimeout(() => fs.writeFileSync(process.argv[3], "survived"), 600);
    break;
  case "environment":
    process.stdout.write(JSON.stringify({
      cwd: process.cwd(),
      allowed: process.env.AI_METER_ALLOWED ?? null,
      path: process.env.PATH ?? null,
    }));
    break;
  default:
    process.stderr.write("unknown fixture mode");
    process.exitCode = 2;
}
