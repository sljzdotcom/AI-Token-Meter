const assert = require('node:assert/strict');
const test = require('node:test');

const { normalizeTerminalInput } = require('./gemini-terminal-input.cjs');

test('removes the primary device-attributes reply observed from Windows ConPTY', () => {
 const response = '\x1b[?61;6;7;21;22;23;24;28;32;42c';
 assert.equal(
  normalizeTerminalInput(`${response}/model\r${response}\x1b${response}/quit\r`),
  '/model\r\x1b/quit\r',
 );
});

test('removes the fixed cursor-position reply without changing command order', () => {
 assert.equal(
  normalizeTerminalInput('\x1b[1;1R/model\r\x1b\x1b[1;1R/quit\r'),
  '/model\r\x1b/quit\r',
 );
});

test('preserves incomplete, malformed, and unsupported control input', () => {
 for (const input of [
  '\x1b[?61;6',
  '\x1b[?61:6c/model\r',
  '\x1b[>61;6c/model\r',
  '\x1b[2;3R/model\r',
 ]) {
  assert.equal(normalizeTerminalInput(input), input);
 }
});

test('does not join malformed device-attributes fragments across a removed reply', () => {
 for (const input of [
  '\x1b[?61\x1b[1;1R;6c/model\r',
  '\x1b[?61;6\x1b[1;1Rc/model\r',
 ]) {
  assert.equal(
   normalizeTerminalInput(input),
   input.replace('\x1b[1;1R', ''),
  );
 }
});

test('removes only complete observed protocol replies across accumulated input', () => {
 const partial = '\x1b[?61;6;7';
 assert.equal(normalizeTerminalInput(partial), partial);
 assert.equal(
  normalizeTerminalInput(`${partial};21;22;23;24;28;32;42c/model\r`),
  '/model\r',
 );
});
