#!/usr/bin/env node
/** Narrow upstream characterization; see docs/development/2026-09-08-gemini-cli-capability.md.
 * No CLI launch, account, network, or credential implementation is loaded.
 * Requires --experimental-vm-modules and isolated dependencies documented there.
 */
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import path from 'node:path';
import vm from 'node:vm';
import { createHash } from 'node:crypto';
import { pathToFileURL } from 'node:url';
const source = process.env.GEMINI_PROBE_SOURCE || '/private/tmp/req012-gemini-source';
const deps = process.env.GEMINI_PROBE_DEPS || '/private/tmp/req012-gemini-probe/node_modules';
const dependency = async (name, entry) => import(pathToFileURL(path.join(deps, name, entry)).href);
const { transform } = await dependency('esbuild', 'lib/main.js');
const React = await dependency('react', 'index.js');
const Ink = await dependency('ink', 'build/index.js');
const { render, cleanup } = await dependency('ink-testing-library', 'build/index.js');
const root = path.join(source, 'packages/cli/src');
const files = [
  'ui/commands/statsCommand.ts', 'ui/commands/modelCommand.ts', 'ui/commands/types.ts', 'ui/types.ts',
  'ui/components/StatsDisplay.tsx', 'ui/components/ModelStatsDisplay.tsx', 'ui/components/ModelDialog.tsx',
  'ui/components/ModelQuotaDisplay.tsx', 'ui/components/ProgressBar.tsx', 'ui/components/Table.tsx',
  'ui/components/QuotaStatsInfo.tsx', 'ui/utils/formatters.ts', 'ui/utils/computeStats.ts', 'ui/utils/displayUtils.ts',
  'services/BuiltinCommandLoader.ts', 'services/CommandService.ts', 'services/SlashCommandResolver.ts',
  'utils/commands.ts', 'ui/utils/commandUtils.ts', 'nonInteractiveCliCommands.ts',
  'ui/noninteractive/nonInteractiveUi.ts', 'nonInteractiveCli.ts',
];
const counters = { modelGenerationCalls: 0, modelBoundaryAttempts: 0, secretReads: 0, networkAttempts: 0, quotaRefreshes: 0 };
const refuse = label => (..._args) => { throw new Error(`Unconfigured adapter called: ${label}`); };
const output = [];
const fakeStream = { write: s => { output.push(String(s)); return true; }, on() {}, off() {} };
const fakeProcess = { env: { HOME: '/private/tmp/req012-gemini-empty-home' }, cwd: () => '/private/tmp/req012-gemini-probe', stdout: fakeStream, stderr: fakeStream, stdin: { isTTY: false, removeAllListeners() {} }, platform: 'darwin', exit: refuse('process.exit') };
const context = vm.createContext({ console, process: fakeProcess, AbortController, setTimeout, clearTimeout, Date, Buffer,
  fetch: () => { counters.networkAttempts++; throw new Error('Network forbidden'); } });
const settings = { merged: { ui: { showUserIdentity: true }, security: { auth: { selectedType: 'oauth-personal' } } } };
const stats = { sessionId: 'synthetic-empty-session', sessionStartTime: new Date(), metrics: {
  models: {}, tools: { totalCalls: 0, totalSuccess: 0, totalFail: 0, totalDurationMs: 0, totalDecisions: { accept: 0, reject: 0, modify: 0, auto_accept: 0 } }, files: { totalLinesAdded: 0, totalLinesRemoved: 0 },
} };
let quota = { buckets: [
  { modelId: 'gemini-2.5-pro', remainingFraction: 0.75, resetTime: new Date(Date.now() + 3600000).toISOString() },
  { modelId: 'gemini-2.5-flash', remainingFraction: 0.4, resetTime: new Date(Date.now() + 7200000).toISOString() },
] };
const config = {
  get config() { return this; },
  getModel: () => 'auto', getActiveModel: () => 'gemini-2.5-pro', getUserTierName: () => 'Synthetic Tier', getUserPaidTier: () => undefined,
  refreshUserQuota: async () => { counters.quotaRefreshes++; return quota; }, refreshAvailableCredits: async () => undefined,
  getLastRetrievedQuota: () => quota, getQuotaRemaining: () => 750, getQuotaLimit: () => 1000, getQuotaResetTime: () => quota?.buckets[0]?.resetTime,
  getProModelNoAccessSync: () => false, getProModelNoAccess: async () => false,
  getHasAccessToPreviewModel: () => false, getGemini31LaunchedSync: () => false, getExperimentalDynamicModelConfiguration: () => false,
  getExperimentalGemma: () => false,
  modelConfigService: { getModelDefinition: id => ({ tier: id.includes('pro') ? 'pro' : 'flash' }) },
  isAgentsEnabled: () => false, getExtensionsEnabled: () => false, getEnableHooksUI: () => false, getMcpEnabled: () => false,
  getFolderTrust: () => false, isPlanEnabled: () => false, isSkillsSupportEnabled: () => false, isVoiceModeEnabled: () => false,
  getContentGeneratorConfig: () => ({ authType: 'oauth-personal' }), getSessionId: () => 'synthetic-empty-session',
  getAgentSessionNoninteractiveEnabled: () => false, getDebugMode: () => false, getOutputFormat: () => 'text',
  getRawOutput: () => false, getMessageBus: () => ({}), getMaxSessionTurns: () => 1,
  getGeminiClient: () => ({ sendMessageStream: (...args) => { counters.modelBoundaryAttempts++; throw new Error(`MODEL_BOUNDARY:${JSON.stringify(args[0])}`); } }),
};
const ConfigContext = React.createContext(config);
const core = {
  UserAccountManager: class { getCachedGoogleAccount() { return 'fixture@example.invalid'; } },
  getG1CreditBalance: () => undefined, getDisplayString: x => x,
  isAutoModel: x => x === 'auto', LlmRole: { MAIN: 'main' }, AuthType: { USE_GEMINI: 'gemini-api-key', LOGIN_WITH_GOOGLE: 'oauth-personal' },
  GEMINI_MODEL_ALIAS_AUTO: 'auto', DEFAULT_GEMINI_MODEL: 'gemini-2.5-pro', DEFAULT_GEMINI_FLASH_MODEL: 'gemini-2.5-flash', DEFAULT_GEMINI_FLASH_LITE_MODEL: 'gemini-2.5-flash-lite',
  getAutoModelDescription: () => 'Synthetic auto model description', isProModel: x => x.includes('pro'),
  isNightly: async () => false, startupProfiler: { start: () => ({ end() {} }) }, getAdminErrorMessage: () => 'disabled',
  debugLogger: { debug() {}, warn() {}, error() {} }, coreEvents: { emitSlashCommandConflicts() {}, emitConsoleLog() {}, on() {}, off() {}, drainBacklogs() {} },
  uiTelemetryService: { getMetrics: () => stats.metrics }, Logger: class {}, FatalInputError: class extends Error {},
  promptIdContext: { run: (_id, fn) => fn() }, OutputFormat: { TEXT: 'text', JSON: 'json', STREAM_JSON: 'stream-json' },
  createWorkingStdio: () => ({ stdout: fakeStream }), Scheduler: class { dispose() {} }, CoreEvent: { UserFeedback: 'feedback' },
};
const adapters = {
  'react': React, 'react/jsx-runtime': await dependency('react', 'jsx-runtime.js'), 'ink': Ink,
  '@google/gemini-cli-core': core,
  'ui/semantic-colors.js': { theme: { text: { primary: 'white', secondary: 'gray', link: 'cyan', accent: 'green' }, status: { success: 'green', warning: 'yellow', error: 'red' }, border: { default: 'gray' } } },
  'ui/contexts/SessionContext.js': { useSessionStats: () => ({ stats }) },
  'ui/contexts/SettingsContext.js': { useSettings: () => settings },
  'ui/contexts/ConfigContext.js': { ConfigContext, useConfig: () => config },
  'ui/contexts/UIStateContext.js': { useUIState: () => ({ terminalWidth: 100 }) },
  'ui/hooks/useKeypress.js': { useKeypress() {} },
  'ui/components/shared/DescriptiveRadioButtonSelect.js': { DescriptiveRadioButtonSelect: ({ items }) => React.createElement(Ink.Text, {}, items.map(x => x.title).join(' | ')) },
  'ui/components/ThemedGradient.js': { ThemedGradient: Ink.Text },
  'utils/installationInfo.js': { isDevelopment: false },
  'services/FileCommandLoader.js': { FileCommandLoader: class { async loadCommands() { return []; } } },
  'services/McpPromptLoader.js': { McpPromptLoader: class { async loadCommands() { return []; } } },
  'ui/utils/ConsolePatcher.js': { ConsolePatcher: class { patch() {} cleanup() {} } },
  'ui/hooks/atCommandProcessor.js': { AT_COMMAND_PATH_REGEX_SOURCE: '\\S+', handleAtCommand: async ({ query }) => ({ processedQuery: [{ text: query }] }) },
  'utils/errors.js': { handleError: error => { throw error; } },
  'ui/utils/textOutput.js': { TextOutput: class { write(s) { fakeStream.write(s); } ensureTrailingNewline() {} } },
  'node:readline': { default: {} }, 'strip-ansi': { default: x => x }, 'clipboardy': { default: { write: refuse('clipboard') } },
  'node:fs': { default: new Proxy({}, { get: () => () => { counters.secretReads++; throw new Error('Filesystem forbidden'); } }) },
  'nonInteractiveCliAgentSession.js': { runNonInteractive: refuse('ADK branch out of scope') },
};
const transformed = new Map();
const imports = new Map();
const hashes = {};
function keyFor(spec, importer) { return spec.startsWith('.') ? path.posix.normalize(path.posix.join(path.posix.dirname(importer), spec)) : spec; }
for (const file of files) {
  const content = await fs.readFile(path.join(root, file), 'utf8');
  hashes[file] = createHash('sha256').update(content).digest('hex');
  const key = file.replace(/\.tsx?$/, '.js');
  const { code } = await transform(content, { loader: file.endsWith('tsx') ? 'tsx' : 'ts', format: 'esm', jsx: 'automatic', target: 'es2022' });
  transformed.set(key, code);
  for (const match of code.matchAll(/import\s+([\s\S]*?)\s+from\s+["']([^"']+)["'];/g)) {
    const target = keyFor(match[2], key);
    const names = imports.get(target) || new Set();
    let clause = match[1].trim();
    if (clause.startsWith('{')) clause.slice(1, -1).split(',').forEach(x => { const n = x.trim().split(/\s+as\s+/)[0]; if (n) names.add(n); });
    else names.add('default');
    imports.set(target, names);
  }
}
// The digest pins every executable upstream file to the reviewed v0.58.0 source.
assert.equal(createHash('sha256').update(JSON.stringify(hashes)).digest('hex'), 'af6262dc728d48b9a9cf273afd6014f1c58b7534abc0f2f9be88be2be4e9c395', 'Upstream baseline differs; review source before updating this digest');
// Unrelated built-ins are inert fixtures; the REAL loader still constructs and returns stats/model.
for (const [key, names] of imports) {
  if (transformed.has(key) || adapters[key]) continue;
  if (/^ui\/commands\/.*Command\.js$/.test(key)) {
    adapters[key] = Object.fromEntries([...names].map(name => [name,
      ['extensionsCommand', 'ideCommand', 'memoryCommand', 'restoreCommand'].includes(name) ? () => null
      : name === 'chatCommand' || name === 'resumeCommand' ? { name: `fixture-${name}`, kind: 'built-in', subCommands: [] } : null]));
  } else throw new Error(`Missing explicit adapter: ${key}`);
}
const modules = new Map();
for (const [key, code] of transformed) modules.set(key, new vm.SourceTextModule(code, { context, identifier: key }));
for (const [key, names] of imports) {
  if (modules.has(key)) continue;
  const values = adapters[key];
  modules.set(key, new vm.SyntheticModule([...names], function () {
    for (const name of names) this.setExport(name, name in values ? values[name] : refuse(`${key}:${name}`));
  }, { context, identifier: key }));
}
const entry = new vm.SourceTextModule(files.filter(x => /Command\.ts$|Display\.tsx$|ModelDialog\.tsx$|nonInteractiveCli\.ts$/.test(x)).map((x, i) => `import * as m${i} from './${x.replace(/\.tsx?$/, '.js')}'; export { m${i} };`).join('\n'), { context, identifier: 'entry.js' });
await entry.link((specifier, importer) => {
  const key = keyFor(specifier, importer.identifier);
  if (!modules.has(key)) throw new Error(`Forbidden module: ${key}`);
  return modules.get(key);
});
await entry.evaluate();
const api = file => modules.get(file).namespace;
const items = [];
const commandContext = { services: { agentContext: { config }, settings }, session: { stats }, ui: { addItem: x => items.push(x) } };
const frames = {};
async function frame(name, Component, props) {
  const result = render(React.createElement(Component, props));
  await new Promise(resolve => setTimeout(resolve, 30));
  frames[name] = result.lastFrame();
  result.unmount();
  return frames[name];
}
const statsCommand = api('ui/commands/statsCommand.js').statsCommand;
await statsCommand.action(commandContext);
assert.equal(counters.quotaRefreshes, 1);
assert.equal(items[0].pooledRemaining, 750);
const sessionFrame = await frame('stats-empty', api('ui/components/StatsDisplay.js').StatsDisplay, { ...items[0], quotaStats: { remaining: 750, limit: 1000 } });
assert.match(sessionFrame, /Session Stats/);
assert.doesNotMatch(sessionFrame, /25%|750|1,000|Model usage/);
await statsCommand.subCommands.find(x => x.name === 'model').action(commandContext);
const modelStatsFrame = await frame('stats-model-empty', api('ui/components/ModelStatsDisplay.js').ModelStatsDisplay, { currentModel: 'auto', quotaStats: { remaining: 750, limit: 1000 } });
assert.match(modelStatsFrame, /No API calls have been made/);
assert.doesNotMatch(modelStatsFrame, /25%|750|1,000/);
if (process.argv.includes('--expect-empty-stats-quota')) assert.match(modelStatsFrame, /25%/, 'Rejected assumption: empty-session model stats must expose quota');
// Negative capability control: a collector requiring quota here MUST fail against the real renderer.
assert.throws(() => assert.match(modelStatsFrame, /25%/), { name: 'AssertionError' });
const result = await api('ui/commands/modelCommand.js').modelCommand.action(commandContext, '');
assert.equal(result.dialog, 'model');
const dialog = await frame('model-dialog', api('ui/components/ModelDialog.js').ModelDialog, { onClose() {} });
assert.match(dialog, /Model usage/);
assert.match(dialog, /Pro[^\n]*25%/);
assert.match(dialog, /Flash[^\n]*60%/);
assert.match(dialog, /Resets:/);
assert.equal(counters.modelBoundaryAttempts, 0);
const savedQuota = quota;
quota = undefined;
const noQuota = await frame('model-dialog-no-quota', api('ui/components/ModelDialog.js').ModelDialog, { onClose() {} });
assert.doesNotMatch(noQuota, /Model usage|25%|60%|Resets:/);
quota = savedQuota;
const headless = [];
for (const input of ['/stats', '/stats model', '/model', '/not-a-command']) {
  const before = counters.modelBoundaryAttempts;
  let error;
  output.length = 0;
  try { await api('nonInteractiveCli.js').runNonInteractive({ config, settings, input, prompt_id: 'synthetic-prompt' }); }
  catch (e) { error = e.message; }
  headless.push({ input, modelBoundaryAttempts: counters.modelBoundaryAttempts - before, error, output: output.join('') });
}
assert.equal(headless[0].modelBoundaryAttempts, 1);
assert.match(headless[0].error, /MODEL_BOUNDARY/);
assert.equal(headless[1].modelBoundaryAttempts, 1);
assert.equal(headless[2].modelBoundaryAttempts, 0);
assert.match(headless[2].error, /not supported in non-interactive mode/);
assert.equal(headless[3].modelBoundaryAttempts, 1);
assert.equal(counters.modelGenerationCalls, 0);
assert.equal(counters.secretReads, 0);
assert.equal(counters.networkAttempts, 0);
cleanup();
const report = { sourceBaseline: 'v0.58.0 / ac9431c9e2290d68af31a77614ff2fddb2391ca3', counters, frames, headless, sourceSha256: hashes,
  boundary: 'Real pinned command/renderer/loader/headless loop; synthetic config/account/quota and I/O. No CLI startup, OAuth, PTY, Windows, or ADK verification.' };
const destination = process.env.GEMINI_PROBE_OUTPUT || '/private/tmp/req012-gemini-probe/result.json';
await fs.writeFile(destination, JSON.stringify(report, null, 2) + '\n');
console.log('PASS: real /model renders 25% and 60%; empty stats hides quota; headless stats falls through to model boundary; headless model rejects; generation/credential/network = 0.');
console.log(`Evidence: ${destination}`);
