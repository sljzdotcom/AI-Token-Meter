#!/usr/bin/env node
// Bounded official-CLI startup characterization. See the Task4a appendix in the capability report.
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
const root = '/private/tmp/req012-gemini-startup';
const cli = path.join(root, 'node_modules/@google/gemini-cli/bundle/gemini.js');
assert.equal(JSON.parse(await fs.readFile(path.join(root, 'node_modules/@google/gemini-cli/package.json'), 'utf8')).version, '0.58.0');
await fs.writeFile('/private/tmp/req012-gemini-boundary-denied.txt', 'synthetic permission probe');
const lockfile = await fs.readFile(path.join(root, 'package-lock.json'), 'utf8');
const packageIntegrity = JSON.parse(lockfile).packages['node_modules/@google/gemini-cli'].integrity;
assert.equal(packageIntegrity, 'sha512-++LtUYMcLE8dVxMcuwv6kIp8+h6z+std/7iVE+vSunkrwNDaWMFkWw/psv2RSySWjr2A1SsEEIGCK0xULWY2sA==');
await fs.copyFile(process.execPath, path.join(root, 'node'));
await fs.chmod(path.join(root, 'node'), 0o755);
const scenario = process.argv.slice(2).find(x => !x.startsWith('--')) || 'authenticated';
const untrustedWorkspace = process.argv.includes('--untrusted-workspace');
const sentinelControl = process.argv.includes('--sentinel-control');
const useAllowlist = !sentinelControl && !process.argv.includes('--without-mcp-allowlist');
assert.ok(['authenticated', 'missing-auth', 'invalid-auth', 'version', 'quota-failure'].includes(scenario));
assert.ok(!untrustedWorkspace || (scenario === 'authenticated' && useAllowlist && !sentinelControl), 'Untrusted scenario requires the default authenticated isolation combination');
const runRoot = path.join(root, scenario + (untrustedWorkspace ? '-untrusted' : '') + (useAllowlist ? '-allowlist' : '') + (sentinelControl ? '-control' : ''));
await fs.mkdir(path.join(runRoot, 'home/.gemini/extensions/sentinel'), { recursive: true });
await fs.mkdir(path.join(runRoot, 'workspace'), { recursive: true });
await fs.writeFile(path.join(runRoot, 'workspace/.env'), '');
const sentinel = path.join(runRoot, 'sentinel.sh');
await fs.writeFile(sentinel, '#!/bin/sh\necho sentinel >> "' + path.join(runRoot, 'sentinel-fired') + '"\n');
await fs.chmod(sentinel, 0o700);
const hookSentinel = path.join(runRoot, 'hook-sentinel.sh');
const mcpSentinel = path.join(runRoot, 'mcp-sentinel.sh');
const extensionSentinel = path.join(runRoot, 'extension-sentinel.sh');
for (const target of [hookSentinel, mcpSentinel, extensionSentinel]) { await fs.copyFile(sentinel, target); await fs.chmod(target, 0o700); }
const settings = {
  security: { auth: { selectedType: 'oauth-personal' }, folderTrust: { enabled: untrustedWorkspace } },
  general: { previewFeatures: false, enableAutoUpdate: false },
  advanced: { autoConfigureMemory: false }, telemetry: { enabled: false },
  hooksConfig: { enabled: true }, hooks: { SessionStart: [{ hooks: [{ type: 'command', command: hookSentinel }] }] },
  mcpServers: { startup_sentinel: { command: mcpSentinel } },
};
await fs.writeFile(path.join(runRoot, 'home/.gemini/settings.json'), JSON.stringify(settings));
await fs.writeFile(path.join(runRoot, 'home/.gemini/extensions/sentinel/gemini-extension.json'), JSON.stringify({ name: 'startup-sentinel', version: '1.0.0', mcpServers: { extension_sentinel: { command: extensionSentinel } } }));
await fs.writeFile(path.join(runRoot, 'home/.gemini/GEMINI.md'), 'SYNTHETIC_GLOBAL_MEMORY_SENTINEL; fixture text, no instructions.');
await fs.writeFile(path.join(runRoot, 'system.json'), JSON.stringify({
  hooksConfig: { enabled: sentinelControl }, admin: { mcp: { enabled: sentinelControl }, extensions: { enabled: sentinelControl }, skills: { enabled: false } },
  context: { memoryBoundaryMarkers: [], includeDirectories: [], fileName: [] },
  privacy: { usageStatisticsEnabled: false }, telemetry: { enabled: false }, ide: { enabled: false },
  general: { enableAutoUpdate: false }, advanced: { autoConfigureMemory: false },
}));
await fs.writeFile(path.join(runRoot, 'system-defaults.json'), '{}');
const trustedFoldersPath = path.join(runRoot, 'home/.gemini/trustedFolders.json');
if (untrustedWorkspace) assert.equal(await fs.access(trustedFoldersPath).then(() => true, () => false), false, 'Start without a trust rule');
const authPath = path.join(runRoot, 'home/.gemini/oauth_creds.json');
await fs.rm(authPath, { force: true });
if (scenario === 'authenticated' || scenario === 'invalid-auth' || scenario === 'quota-failure') {
  await fs.writeFile(authPath, JSON.stringify({ access_token: 'SYNTHETIC_ACCESS_TOKEN', refresh_token: 'SYNTHETIC_REFRESH_TOKEN', expiry_date: Date.now() + 86400000, token_type: 'Bearer', scope: 'https://www.googleapis.com/auth/cloud-platform' }));
  await fs.writeFile(path.join(runRoot, 'home/.gemini/google_accounts.json'), JSON.stringify({ active: 'fixture@example.invalid', old: [] }));
}
const preload = String.raw`
const fs = require('node:fs');
const cp = require('node:child_process');
// Equivalent native realpath avoids Node's JS ancestor-lstat permission issue.
const originalRealpath = fs.realpath;
fs.realpath = Object.assign(function (p, ...args) { return String(p).startsWith('/private/tmp/req012-gemini-startup/') ? originalRealpath.native(p, ...args) : originalRealpath(p, ...args); }, { native: originalRealpath.native });
const { syncBuiltinESMExports } = require('node:module');
const path = require('node:path');
const eventsFile = process.env.PROBE_EVENTS;
const append = fs.appendFileSync.bind(fs);
function log(event) { append(eventsFile, JSON.stringify(event) + '\n'); }
const assert = require('node:assert/strict');
assert.throws(() => fs.readFileSync('/private/tmp/req012-gemini-boundary-denied.txt'), {code:'ERR_ACCESS_DENIED'});
assert.throws(() => cp.spawn('/usr/bin/true', []), {code:'ERR_ACCESS_DENIED'});
assert.equal(process.permission.has('addons'), false);
assert.equal(process.permission.has('ffi'), false);
log({type:'guard-check', filesystemBlocked:true, subprocessBlocked:true, addonsBlocked:true, ffiBlocked:true});
require('node:net').connect({host:'127.0.0.1',port:9}).on('error', error => { assert.equal(error.code,'ERR_ACCESS_DENIED'); log({type:'guard-network-check', blocked:true}); }).on('connect', () => { throw new Error('Network permission guard failed'); });
for (const target of [fs, fs.promises]) for (const name of target === fs ? ['writeFile','writeFileSync','appendFile','appendFileSync','rename','renameSync'] : ['writeFile','appendFile','rename']) {
  const original = target[name];
  target[name] = function(p,...args) { if (/trustedFolders/.test(String(p)) || (name.startsWith('rename') && /trustedFolders/.test(String(args[0])))) log({type:'trust-write-attempt', path:String(p)}); return original.call(this,p,...args); };
}
for (const target of [fs, fs.promises]) for (const name of target === fs ? ['readFile','readFileSync'] : ['readFile']) {
  const original = target[name];
  target[name] = function(p,...args) { if (typeof p === 'string' && /settings|\.env|GEMINI\.md|oauth_creds|google_accounts/.test(p)) log({type:'file-read', path:p}); return original.call(this,p,...args); };
}
const originalDlopen = process.dlopen;
process.dlopen = function(...args) { log({type:'addon-attempt', path:String(args[1])}); return originalDlopen.apply(this,args); };

process.on('uncaughtExceptionMonitor', error => log({ type: 'uncaught', message: error.message, stack: error.stack }));
const originalExit = process.exit;
process.exit = function(code) { log({ type: 'exit', code, stack: new Error().stack }); return originalExit.call(process, code); };
const stdinEmit = process.stdin.emit;
process.stdin.emit = function(event, ...args) { if(event === 'data') log({type:'terminal-input', data:String(args[0])}); return stdinEmit.call(this,event,...args); };
log({ type: 'boundary', home: process.env.HOME, permissions: ['filesystem-only-temp', 'network-denied', 'child-process-denied', 'addons-denied', 'ffi-denied'] });
for (const name of ['spawn', 'spawnSync', 'exec', 'execSync', 'execFile', 'execFileSync', 'fork']) {
  const original = cp[name];
  cp[name] = function (...args) { log({ type: 'child-attempt', api: name, command: String(args[0]), stack: String(args[0]).includes('sentinel') ? new Error().stack : undefined, args: Array.isArray(args[1]) ? args[1] : undefined }); return original.apply(this, args); };
}
syncBuiltinESMExports();
// Permission enforcement lives below the HTTP interceptor; unrecognized networking cannot escape.
const nativeFetch = globalThis.fetch;
const nock = require(${JSON.stringify(path.join(root, 'node_modules/nock'))});
const interceptedFetch = globalThis.fetch;
globalThis.fetch = (input, ...args) => String(input).startsWith('data:') ? nativeFetch(input, ...args) : interceptedFetch(input, ...args);
nock.disableNetConnect();
nock.emitter.on('no match', req => log({ type: 'network-denied', method: req.method, path: String(req.path).slice(0, 200), host: req.hostname || req.host }));
nock('https://cloudcode-pa.googleapis.com').persist().post('/v1internal:listExperiments').reply(function(uri) { log({ type: 'fixture-request', name: 'listExperiments', uri }); return [200, { experiments: [] }]; });
const reset = new Date(Date.now() + 3600000).toISOString();
function reply(name, data) { return function (uri, body) { log({ type: 'fixture-request', name, method: this.req.method, uri }); return [name === 'tokeninfo' && process.env.PROBE_SCENARIO === 'invalid-auth' ? 401 : name === 'retrieveUserQuota' && process.env.PROBE_SCENARIO === 'quota-failure' ? 403 : 200, data]; }; }
nock('https://oauth2.googleapis.com').persist().post('/tokeninfo').reply(reply('tokeninfo', { audience: 'synthetic', scope: 'https://www.googleapis.com/auth/cloud-platform', expires_in: 86400, email: 'fixture@example.invalid' }));
nock('https://cloudcode-pa.googleapis.com').persist().post('/v1internal:loadCodeAssist').reply(reply('loadCodeAssist', { currentTier: { id: 'free-tier', name: 'Synthetic Free Tier' }, cloudaicompanionProject: 'synthetic-project' }));
nock('https://cloudcode-pa.googleapis.com').persist().post('/v1internal:retrieveUserQuota').reply(reply('retrieveUserQuota', { buckets: [{ modelId: 'gemini-2.5-pro', remainingFraction: 0.75, resetTime: reset, tokenType: 'REQUESTS' }, { modelId: 'gemini-2.5-flash', remainingFraction: 0.4, resetTime: reset, tokenType: 'REQUESTS' }] }));
`;
await fs.writeFile(path.join(runRoot, 'preload.cjs'), preload);
const events = path.join(runRoot, 'events.jsonl');
await fs.writeFile(events, '');
const options = {
  scenario, cwd: path.join(runRoot, 'workspace'),
  argv: [path.join(root, 'node'), '--permission', `--allow-fs-read=${root}`, '--allow-fs-read=/.dockerenv', `--allow-fs-write=${runRoot}`, '--require', path.join(runRoot, 'preload.cjs'), cli, ...(scenario === 'version' ? ['--version'] : []), ...(sentinelControl ? [] : ['-e', 'none']), ...(useAllowlist ? ['--allowed-mcp-server-names', 'probe-no-such-server'] : [])],
  env: { ...(untrustedWorkspace ? { GEMINI_CLI_TRUST_WORKSPACE: 'false' } : {}), HOME: path.join(runRoot, 'home'), USERPROFILE: path.join(runRoot, 'home'), PATH: root + ':/usr/bin:/bin', TERM: 'xterm-256color', LANG: 'en_US.UTF-8', SHELL: '/bin/sh', NO_BROWSER: 'true', GEMINI_CLI_NO_RELAUNCH: 'true', GEMINI_CLI_SYSTEM_SETTINGS_PATH: path.join(runRoot, 'system.json'), GEMINI_CLI_SYSTEM_DEFAULTS_PATH: path.join(runRoot, 'system-defaults.json'), PROBE_EVENTS: events, PROBE_SCENARIO: scenario, TMPDIR: runRoot },
  transcript: path.join(runRoot, 'terminal.txt'), outcome: path.join(runRoot, 'outcome.json'),
};
await fs.writeFile(path.join(runRoot, 'options.json'), JSON.stringify(options));
const python = String.raw`
import os, pty, json, sys, select, time, signal, fcntl, termios, struct
p=json.load(open(sys.argv[1])); pid, fd=pty.fork()
if pid == 0:
    os.chdir(p['cwd']); os.execve(p['argv'][0], p['argv'], p['env'])
fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack('HHHH', 50, 140, 0, 0))
start=time.monotonic(); chunks=[]; sent=False; exiting=False; status=None; timeout=False; cancelled=False; pending=[]; cleanupError=None
def type_command(value, delay=0):
    now=time.monotonic()+delay
    for i,c in enumerate(value): pending.append((now+i*0.1,bytes([c])))
    pending.append((now+len(value)*0.1+0.5,b'\r'))
while time.monotonic()-start < 25:
    ready,_,_=select.select([fd],[],[],0.02)
    if ready:
        try:
            chunk=os.read(fd,65536)
            if not chunk: break
            chunks.append(chunk)
        except OSError: break
    now=time.monotonic()
    while pending and pending[0][0] <= now:
        _,data=pending.pop(0); os.write(fd,data)
    if p['scenario'] in ('authenticated','quota-failure') and not sent and now-start > 5:
        type_command(b'/model'); sent=True
    text=b''.join(chunks).decode('utf8','replace')
    if p['scenario'] in ('authenticated','quota-failure') and not exiting and (('25%' in text and '60%' in text) or (p['scenario']=='quota-failure' and 'Select Model' in text) or now-start > 15):
        os.write(fd,b'\x1b'); type_command(b'/quit',0.8); exiting=True
    if not cancelled and ((exiting and now-start > 18) or (p['scenario'] not in ('authenticated','quota-failure') and now-start > 8)):
        os.write(fd,b'\x03'); pending.append((now+0.5,b'\x03')); cancelled=True
    done, s=os.waitpid(pid,os.WNOHANG)
    if done: status=s; break
open(p['transcript'],'w').write(b''.join(chunks).decode('utf8','replace'))
if status is None:
    done,s=os.waitpid(pid,os.WNOHANG)
    if done: status=s
    else:
        timeout=True
        os.close(fd); time.sleep(0.5)
        try: os.kill(pid,signal.SIGKILL)
        except (ProcessLookupError, PermissionError) as e: cleanupError=str(e)
        reapDeadline=time.monotonic()+2
        while time.monotonic()<reapDeadline:
            done,status=os.waitpid(pid,os.WNOHANG)
            if done: break
            time.sleep(0.05)
        if not done: status=None
open(p['transcript'],'w').write(b''.join(chunks).decode('utf8','replace'))
json.dump({'pid':pid,'status':status,'exitCode':os.waitstatus_to_exitcode(status) if status is not None else None,'timedOut':timeout,'sentCancellation':cancelled,'sentModelCommand':sent,'cleanupError':cleanupError,'elapsed':time.monotonic()-start},open(p['outcome'],'w'))
`;
await fs.writeFile(path.join(root, 'pty-driver.py'), python);
const run = spawnSync('/usr/bin/python3', [path.join(root, 'pty-driver.py'), path.join(runRoot, 'options.json')], { timeout: 30000, encoding: 'utf8', env: { PATH: '/usr/bin:/bin', HOME: path.join(runRoot, 'home') } });
assert.equal(run.status, 0, run.stderr || String(run.error));
const terminal = await fs.readFile(options.transcript, 'utf8');
const outcome = JSON.parse(await fs.readFile(options.outcome, 'utf8'));
const calls = (await fs.readFile(events, 'utf8')).trim().split('\n').filter(Boolean).map(x => JSON.parse(x));
const plainTerminal = terminal.replace(/\x1b\[[0-?]*[ -/]*[@-~]/g, '');
const sentinelAttempts = calls.filter(x => x.type === 'child-attempt' && /(?:hook|mcp|extension)-sentinel/.test(JSON.stringify([x.command,x.args])));
const report = { nodeVersion: process.version, nockVersion: JSON.parse(await fs.readFile(path.join(root,'node_modules/nock/package.json'),'utf8')).version, lockSha256:createHash('sha256').update(lockfile).digest('hex'), sentinelAttempts, packageIntegrity, scenario, untrustedWorkspace, useAllowlist, sentinelControl, outcome, calls, terminalPath: options.transcript, visibleQuota: terminal.includes('25%') && terminal.includes('60%') };
await fs.writeFile(path.join(runRoot, 'result.json'), JSON.stringify(report,null,2));
// These assertions reject a real upstream behavior change, not fixture membership.
assert.equal(outcome.timedOut, false, 'CLI must exit before the deadline');
assert.equal(outcome.exitCode, 0, 'CLI exit/cancellation must be observed, never inferred');
assert.ok(calls.some(x => x.type === 'guard-check' && x.filesystemBlocked && x.subprocessBlocked));
assert.ok(calls.some(x => x.type === 'guard-network-check' && x.blocked));
assert.equal(calls.filter(x => x.type === 'network-denied').length, 0, 'Unexpected/model network attempt');
assert.equal(calls.filter(x => x.type === 'child-attempt' && /(^|[ /])(open|security|osascript)( |$)/.test(x.command)).length, 0, 'Browser/keychain helper attempt');
assert.equal(calls.filter(x => x.type === 'addon-attempt' && /keytar/i.test(x.path)).length, 0, 'Keychain addon attempt');
assert.equal(await fs.access(path.join(runRoot,'sentinel-fired')).then(() => true, () => false), false, 'No sentinel may actually execute');
const finalSettings=JSON.parse(await fs.readFile(path.join(runRoot,'home/.gemini/settings.json'),'utf8'));
assert.deepEqual(finalSettings.model,settings.model,'Opening and escaping quota dialog must not persist a model selection');
if (untrustedWorkspace) {
  assert.equal(calls.filter(x => x.type === 'trust-write-attempt').length, 0, 'No trust file write attempt');
  assert.equal(await fs.access(trustedFoldersPath).then(() => true, () => false), false, 'No automatic trust approval or trust file write');
  assert.deepEqual(finalSettings.security, settings.security, 'Trust settings must remain enabled');
  assert.doesNotMatch(plainTerminal, /Do you trust the files in this folder\?|A folder trust level must be selected/);
  assert.match(plainTerminal, /This folder is untrusted|Untrusted/);
}
const fixtureRequests=calls.filter(x=>x.type==='fixture-request').map(x=>x.name);
if (scenario === 'authenticated') {
  assert.match(plainTerminal,/Model usage/);
  assert.match(plainTerminal,/Pro[^\n]*25%/);
  assert.match(plainTerminal,/Flash[^\n]*60%/);
  assert.match(plainTerminal,/Resets:/);
  assert.equal(outcome.sentCancellation,false);
  assert.ok(fixtureRequests.includes('tokeninfo') && fixtureRequests.includes('loadCodeAssist') && fixtureRequests.includes('retrieveUserQuota'));
  if (sentinelControl) {
    for(const name of ['hook','mcp','extension']) assert.ok(sentinelAttempts.some(x=>JSON.stringify([x.command,x.args]).includes(name+'-sentinel')),name+' control must be detected');
  } else if (useAllowlist) assert.equal(sentinelAttempts.length,0,'Official isolation settings must prevent attempts, not just OS executions');
  else assert.ok(sentinelAttempts.some(x=>JSON.stringify([x.command,x.args]).includes('mcp-sentinel')),'File-based admin alone does not disable MCP');
} else if (scenario === 'quota-failure') {
  assert.match(plainTerminal,/Select Model/);
  assert.equal(report.visibleQuota,false);
  assert.doesNotMatch(plainTerminal,/Model usage/);
  assert.equal(outcome.sentCancellation,false);
  assert.equal(sentinelAttempts.length,0);
} else if (scenario === 'version') {
  assert.match(plainTerminal,/0\.58\.0/);
  assert.equal(fixtureRequests.length,0);
  assert.equal(sentinelAttempts.length,0);
} else {
  assert.match(plainTerminal,/Enter the authorization code/);
  assert.equal(report.visibleQuota,false);
  assert.equal(outcome.sentModelCommand,false);
  assert.equal(outcome.sentCancellation,true);
  assert.deepEqual(fixtureRequests,scenario==='invalid-auth'?['tokeninfo']:[]);
}
console.log(`PASS ${scenario}${sentinelControl?' control':''}: exit=${outcome.exitCode}, visibleQuota=${report.visibleQuota}, sentinelAttempts=${sentinelAttempts.length}, deniedExternalNetwork=0; ${path.join(runRoot,'result.json')}`);
