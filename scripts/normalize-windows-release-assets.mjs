#!/usr/bin/env node

import { createHash } from "node:crypto";
import {
  copyFile,
  mkdir,
  readFile,
  readdir,
  stat,
  writeFile,
} from "node:fs/promises";
import path from "node:path";

function fail(message) {
  throw new Error(message);
}

const [sourceArgument, stageArgument, version] = process.argv.slice(2);

if (!sourceArgument || !stageArgument || !version) {
  fail("usage: normalize-windows-release-assets.mjs SOURCE STAGE VERSION");
}
if (!/^\d+\.\d+\.\d+(?:-preview\.\d+)?$/.test(version)) {
  fail(`Invalid release version: ${version}`);
}

const source = path.resolve(sourceArgument);
const stage = path.resolve(stageArgument);
const entries = await readdir(source, { withFileTypes: true });
const installers = entries.filter(
  (entry) => entry.isFile() && entry.name.toLowerCase().endsWith("-setup.exe"),
);

if (installers.length !== 1) {
  fail(`Expected exactly one NSIS setup executable, found ${installers.length}`);
}

const sourceInstaller = path.join(source, installers[0].name);
const sourceSignature = `${sourceInstaller}.sig`;
let signatureMetadata;
try {
  signatureMetadata = await stat(sourceSignature);
} catch {
  fail(`Expected updater signature beside installer: ${path.basename(sourceSignature)}`);
}
if (!signatureMetadata.isFile() || signatureMetadata.size === 0) {
  fail(`Updater signature is empty or not a file: ${path.basename(sourceSignature)}`);
}

const installerBytes = await readFile(sourceInstaller);
if (installerBytes.length === 0) {
  fail(`NSIS installer is empty: ${path.basename(sourceInstaller)}`);
}

const installerName = `AI-Token-Meter-${version}-windows-x64-setup.exe`;
const stagedInstaller = path.join(stage, installerName);
await mkdir(stage, { recursive: true });
await copyFile(sourceInstaller, stagedInstaller);
await copyFile(sourceSignature, `${stagedInstaller}.sig`);

const hash = createHash("sha256").update(installerBytes).digest("hex");
await writeFile(
  `${stagedInstaller}.sha256`,
  `${hash}  ${installerName}\n`,
  "ascii",
);

console.log(`Normalized signed Windows updater: ${installerName}`);
