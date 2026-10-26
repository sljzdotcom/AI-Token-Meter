import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import vm from "node:vm";

const source = readFileSync(new URL("./code0-bridge.js", import.meta.url), "utf8");
const origin = "https://console.code0.ai";
const nonce = "33333333-4444-4555-8666-777777777777";
const document = "88888888-9999-4000-8111-222222222222";
const status = { success: true, data: { quota_per_unit: 500000 } };
const self = { success: true, data: { quota: 1250000, used_quota: 750000 } };
async function settle() {
  // Drain Response.clone's stream-transfer tasks as well as Promise microtasks.
  for (let turn = 0; turn < 12; turn++) await new Promise((resolve) => setImmediate(resolve));
}

function harness({ subframe = false } = {}) {
  const messages = [];
  const calls = [];
  const xhrs = [];
  let throwNext = null;
  const originalFetch = function (...args) {
    if (throwNext) { const error = throwNext; throwNext = null; throw error; }
    let resolve, reject;
    const promise = new Promise((yes, no) => { resolve = yes; reject = no; });
    calls.push({ args, promise, resolve, reject });
    return promise;
  };
  class FakeXHR extends EventTarget {
    constructor() {
      super();
      this.responseType = "";
      this.readyState = 0;
      this.status = 0;
      this.responseURL = "";
      this.responseText = "";
      this.response = null;
      this.headers = new Headers();
      xhrs.push(this);
    }
    open(method, url, ...rest) {
      this.openArgs = [method, url, ...rest];
      this.readyState = 1;
    }
    send(body) { this.sentBody = body; }
    getResponseHeader(key) { return this.headers.get(key); }
    abort() {
      this.status = 0;
      this.dispatchEvent(new Event("abort"));
      this.dispatchEvent(new Event("loadend"));
    }
    reply(body, { url = new URL(this.openArgs[1], origin).href, code = 200, contentType = "application/json" } = {}) {
      this.responseURL = url;
      this.status = code;
      this.headers.set("content-type", contentType);
      this.responseText = typeof body === "string" ? body : JSON.stringify(body);
      this.response = this.responseType === "json" ? JSON.parse(this.responseText) : this.responseText;
      this.readyState = 4;
      this.dispatchEvent(new Event("readystatechange"));
      this.dispatchEvent(new Event("load"));
      this.dispatchEvent(new Event("loadend"));
    }
  }
  const scope = {
    URL, Request, Response, Headers, TextDecoder, TextEncoder, Uint8Array,
    ArrayBuffer, AbortController, setTimeout, clearTimeout,
    fetch: originalFetch, XMLHttpRequest: FakeXHR,
    location: { href: `${origin}/console`, origin },
  };
  scope.window = scope;
  scope.self = scope;
  scope.top = subframe ? {} : scope;
  const install = vm.runInNewContext(source, scope, { timeout: 1000 });
  const controller = install({ version: 1, nonce, document, send(message) {
    // Native transport is JSON. Keep test assertions independent of VM prototypes.
    messages.push(JSON.parse(JSON.stringify(message)));
  } });
  function request(input, init) {
    const result = scope.fetch(input, init);
    return { result, call: calls.at(-1) };
  }
  function reply(call, body, { url, code = 200, redirected = false, type = "basic", contentType = "application/json" } = {}) {
    const input = call.args[0];
    const requested = input instanceof Request ? input.url : String(input);
    const response = new Response(typeof body === "string" ? body : JSON.stringify(body), {
      status: code, headers: { "content-type": contentType },
    });
    Object.defineProperty(response, "url", { value: url ?? new URL(requested, origin).href });
    Object.defineProperty(response, "redirected", { value: redirected });
    Object.defineProperty(response, "type", { value: type });
    const clone = response.clone.bind(response);
    call.clones = 0;
    response.clone = () => { call.clones++; return clone(); };
    call.resolve(response);
    return response;
  }
  async function complete(input, body, init, options) {
    const { call, result } = request(input, init);
    reply(call, body, options);
    await result;
    await settle();
    return call;
  }
  return {
    scope, calls, xhrs, messages, controller, request, reply, complete,
    captures: () => messages.filter((message) => message.kind === "capture"),
    throwOnNextFetch(error) { throwNext = error; },
  };
}

test("installation and unrelated responses do not request data or read their bodies", async () => {
  const h = harness();
  assert.equal(h.calls.length, 0);
  const call = await h.complete("/api/user/token", { data: "synthetic-unrelated-body" });
  assert.equal(call.clones, 0);
  assert.equal(h.captures().length, 0);
  h.controller.dispose();
});

test("same-document status and complete self produce only the filtered numeric envelope", async () => {
  const h = harness();
  await h.complete("/api/status", status);
  await h.complete("/api/user/self", {
    ...self, data: { ...self.data, email: "synthetic-invalid", id: 42, accessToken: "synthetic-do-not-export" },
  });
  assert.equal(h.captures().length, 1);
  const capture = h.captures()[0];
  assert.deepEqual(Object.keys(capture).sort(), [
    "version", "nonce", "document", "sequence", "kind", "request", "revision", "quota", "usedQuota", "divisor",
  ].sort());
  assert.deepEqual([capture.quota, capture.usedQuota, capture.divisor], [1250000, 750000, 500000]);
  assert.equal(capture.version, 1);
  assert.equal(capture.nonce, nonce);
  assert.equal(capture.document, document);
  assert.ok(Number.isSafeInteger(capture.sequence) && capture.sequence > 0);
  assert.ok(Number.isSafeInteger(capture.request) && capture.request > 0);
  assert.equal(capture.revision, 0);
  assert.ok(Buffer.byteLength(JSON.stringify(capture)) <= 2048);
  assert.ok(!JSON.stringify(h.messages).includes("synthetic-do-not-export"));
  await h.complete("/api/user/self", self);
  assert.equal(h.captures().length, 1, "an attempt publishes once");
});

test("self may arrive before status but cannot publish with a missing divisor", async () => {
  const h = harness();
  await h.complete("/api/user/self", self);
  assert.equal(h.captures().length, 0);
  await h.complete("/api/status", status);
  assert.equal(h.captures().length, 1);
});

test("newer self request wins even when an older complete response arrives last", async () => {
  const h = harness();
  await h.complete("/api/status", status);
  const old = h.request("/api/user/self");
  const current = h.request("/api/user/self");
  h.reply(old.call, { success: true, data: { quota: 9, used_quota: 8 } });
  await old.result; await settle();
  assert.equal(h.captures().length, 0);
  h.reply(current.call, self);
  await current.result; await settle();
  assert.equal(h.captures().length, 1);
  assert.equal(h.captures()[0].quota, 1250000);
});

test("overlapping refreshes require a self started after every refresh has finished", async () => {
  const h = harness();
  await h.complete("/api/status", status);
  const old = h.request("/api/user/self");
  const a = h.request("/api/user/auth/refresh", { method: "POST" });
  const b = h.request("/api/user/auth/refresh", { method: "POST" });
  h.reply(a.call, { success: true, data: { accessToken: "synthetic-never-read" } });
  await a.result; await settle();
  const premature = h.request("/api/user/self");
  h.reply(b.call, { success: true });
  await b.result; await settle();
  h.reply(old.call, self); h.reply(premature.call, self);
  await Promise.all([old.result, premature.result]); await settle();
  assert.equal(h.captures().length, 0);
  assert.equal(a.call.clones, 0, "auth bundle must not be read");
  assert.equal(b.call.clones, 0);
  await h.complete("/api/user/self", self);
  assert.equal(h.captures().length, 1);
  assert.equal(h.captures()[0].revision, 2);
  assert.ok(!JSON.stringify(h.messages).includes("synthetic-never-read"));
});

test("a 2xx refresh by itself never counts as successful authentication or quota", async () => {
  const h = harness();
  await h.complete("/api/status", status);
  await h.complete("/api/user/auth/refresh", { success: false, accessToken: "synthetic-not-read" }, { method: "POST" });
  assert.equal(h.captures().length, 0);
  assert.equal(h.calls[1].clones, 0);
  await h.complete("/api/user/self", { success: false, data: self.data });
  assert.equal(h.captures().length, 0);
});

test("login and logout request starts invalidate pending values without reading their bodies", async () => {
  for (const path of ["/api/user/login", "/api/user/login/2fa", "/api/user/auth/logout", "/api/user/logout"]) {
    const h = harness();
    await h.complete("/api/status", status);
    const pending = h.request("/api/user/self");
    await h.complete(path, { success: true }, { method: path === "/api/user/logout" ? "GET" : "POST", body: path === "/api/user/logout" ? undefined : "synthetic-body" });
    h.reply(pending.call, self); await pending.result; await settle();
    assert.equal(h.captures().length, 0, path);
    assert.ok(h.messages.some((message) => message.kind === "invalidate"), path);
    assert.ok(!JSON.stringify(h.messages).includes("synthetic-body"));
  }
});

test("exact request and response allowlists reject method, query, origin and redirect variants", async () => {
  const cases = [
    ["/api/user/self", { method: "POST" }, {}],
    [new Request(`${origin}/api/user/self`), { method: "POST" }, {}],
    ["/api/user/self?debug=1", {}, {}],
    ["/api/user/self#fragment", {}, {}],
    ["/api/user/self/extra", {}, {}],
    ["https://console.code0.ai.invalid/api/user/self", {}, {}],
    ["https://sub.console.code0.ai/api/user/self", {}, {}],
    ["https://console.code0.ai:444/api/user/self", {}, {}],
    ["https://user:password@console.code0.ai/api/user/self", {}, {}],
    ["http://console.code0.ai/api/user/self", {}, {}],
    ["/api/user/self", {}, { url: "https://other.invalid/api/user/self" }],
    ["/api/user/self", {}, { redirected: true }],
    ["/api/user/self", {}, { type: "opaque" }],
    ["/api/user/self", {}, { type: "opaqueredirect" }],
    ["/api/user/self", {}, { url: `${origin}/api/status` }],
    ["/api/user/self", {}, { url: "" }],
    ["/api/user/self", {}, { url: "/api/user/self" }],
    ["/api/user/self", {}, { code: 401 }],
    ["/api/user/self", {}, { contentType: "text/html" }],
  ];
  for (const [input, init, options] of cases) {
    const h = harness();
    await h.complete("/api/status", status);
    await h.complete(input, self, init, options);
    assert.equal(h.captures().length, 0, String(input));
  }
});

test("GET Request and URL input work, preserving fetch identity and readable original body", async () => {
  const h = harness();
  await h.complete(new URL(`${origin}/api/status`), status);
  const originalRequest = new Request(`${origin}/api/user/self`, { method: "POST" });
  const result = h.request(originalRequest, { method: "GET" });
  assert.equal(result.result, result.call.promise);
  const response = h.reply(result.call, self);
  assert.equal(await result.result, response);
  await settle();
  assert.deepEqual(await response.json(), self);
  assert.equal(h.captures().length, 1);
  const error = new Error("synthetic original fetch error");
  h.throwOnNextFetch(error);
  assert.throws(() => h.scope.fetch("/unrelated"), (actual) => actual === error);
});

test("XHR supports a complete pair and preserves caller load events", async () => {
  const h = harness();
  const a = new h.scope.XMLHttpRequest(); a.open("GET", "/api/status"); a.send();
  a.reply(status);
  const b = new h.scope.XMLHttpRequest(); b.open("GET", "/api/user/self"); b.send();
  let loads = 0; b.addEventListener("load", () => loads++);
  b.reply(self); await settle();
  assert.equal(loads, 1);
  assert.deepEqual(JSON.parse(b.responseText), self);
  assert.equal(h.captures().length, 1);
});

test("XHR final URL must match the original exact endpoint, without claiming visibility of intermediate hops", async () => {
  for (const url of ["", "/api/user/self", `${origin}/api/status`, `${origin}/api/user/self?extra=1`, "https://other.invalid/api/user/self"]) {
    const h = harness();
    await h.complete("/api/status", status);
    const xhr = new h.scope.XMLHttpRequest(); xhr.open("GET", "/api/user/self"); xhr.send();
    xhr.reply(self, { url }); await settle();
    assert.equal(h.captures().length, 0, url);
  }
});

test("invalid quota and missing or invalid divisors never emit numeric values", async () => {
  for (const data of [
    { quota: -1, used_quota: 0 }, { quota: 1.5, used_quota: 0 },
    { quota: "1", used_quota: 0 }, { quota: true, used_quota: 0 },
    { quota: 1 }, { quota: 9007199254740992, used_quota: 0 },
  ]) {
    const h = harness();
    await h.complete("/api/status", status);
    await h.complete("/api/user/self", { success: true, data });
    assert.equal(h.captures().length, 0);
  }
  for (const divisor of [0, -1, "500000", null, 9007199254740992]) {
    const h = harness();
    await h.complete("/api/status", { success: true, data: { quota_per_unit: divisor } });
    await h.complete("/api/user/self", self);
    assert.equal(h.captures().length, 0);
  }
});

test("oversize response fails the attempt without exporting the body", async () => {
  const h = harness();
  await h.complete("/api/status", status);
  await h.complete("/api/user/self", { ...self, padding: "s".repeat(65536) });
  assert.equal(h.captures().length, 0);
  assert.ok(h.messages.some((message) => message.kind === "failure"));
  assert.ok(h.messages.every((message) => Buffer.byteLength(JSON.stringify(message)) <= 2048));
});

test("raw JSON numbers cannot round a fractional or oversized quota into an accepted integer", async () => {
  for (const raw of ["9007199254740991.1", "9007199254740990.9", "1.00000000000000001", "-1e-999"]) {
    const h = harness();
    await h.complete("/api/status", status);
    await h.complete("/api/user/self", `{"success":true,"data":{"quota":${raw},"used_quota":0}}`);
    assert.equal(h.captures().length, 0, raw);
  }
});

test("zero quota and a fractional positive divisor remain valid site-credit values", async () => {
  const h = harness();
  await h.complete("/api/status", { success: true, data: { quota_per_unit: 0.5 } });
  await h.complete("/api/user/self", { success: true, data: { quota: 0, used_quota: 1 } });
  assert.equal(h.captures().length, 1);
  assert.deepEqual([h.captures()[0].quota, h.captures()[0].usedQuota, h.captures()[0].divisor], [0, 1, 0.5]);
});

test("UTF-8 byte limits apply even when the response has fewer than 64KiB characters", async () => {
  const h = harness();
  await h.complete("/api/status", status);
  await h.complete("/api/user/self", { ...self, padding: "额".repeat(24000) });
  assert.equal(h.captures().length, 0);
  assert.ok(h.messages.some((message) => message.kind === "failure"));
});

test("refresh transport failure and rejected fetch pause without changing caller errors", async () => {
  const rejected = harness();
  const error = new Error("synthetic-network-failure-do-not-export");
  const pending = rejected.request("/api/user/auth/refresh", { method: "POST" });
  pending.call.reject(error);
  await assert.rejects(pending.result, (actual) => actual === error);
  await settle();
  assert.ok(rejected.messages.some((message) => message.kind === "failure"));
  assert.ok(!JSON.stringify(rejected.messages).includes(error.message));

  const conflict = harness();
  await conflict.complete("/api/status", status);
  await conflict.complete("/api/user/auth/refresh", { success: false }, { method: "POST" }, { code: 409 });
  await conflict.complete("/api/user/self", self);
  assert.equal(conflict.captures().length, 0);
  assert.ok(conflict.messages.some((message) => message.kind === "failure"));
});

test("XHR abort ends an auth refresh instead of allowing later data", async () => {
  const h = harness();
  await h.complete("/api/status", status);
  const refresh = new h.scope.XMLHttpRequest();
  refresh.open("POST", "/api/user/auth/refresh"); refresh.send(); refresh.abort();
  await h.complete("/api/user/self", self);
  assert.equal(h.captures().length, 0);
  assert.ok(h.messages.some((message) => message.kind === "failure"));
});

test("subframes and disposed documents cannot publish", async () => {
  const frame = harness({ subframe: true });
  await frame.complete("/api/status", status); await frame.complete("/api/user/self", self);
  assert.equal(frame.messages.length, 0);
  const h = harness();
  await h.complete("/api/status", status);
  const pending = h.request("/api/user/self");
  h.controller.dispose();
  h.reply(pending.call, self); await pending.result; await settle();
  assert.equal(h.captures().length, 0);
});

test("cross-transport pairing and newer XHR self win over an older fetch response", async () => {
  const h = harness();
  const statusXHR = new h.scope.XMLHttpRequest(); statusXHR.open("GET", "/api/status"); statusXHR.send(); statusXHR.reply(status);
  const old = h.request("/api/user/self");
  const current = new h.scope.XMLHttpRequest(); current.open("GET", "/api/user/self"); current.send();
  h.reply(old.call, self); await old.result; await settle();
  assert.equal(h.captures().length, 0);
  current.reply({ success: true, data: { quota: 7, used_quota: 3 } }); await settle();
  assert.equal(h.captures().length, 1);
  assert.deepEqual([h.captures()[0].quota, h.captures()[0].usedQuota], [7, 3]);
});

test("refresh while a self clone is being read prevents the pending parse from publishing", async () => {
  const h = harness(); await h.complete("/api/status", status);
  const pending = h.request("/api/user/self");
  const response = h.reply(pending.call, self);
  let release;
  const delayed = new Promise((resolve) => { release = resolve; });
  let reads = 0;
  response.clone = () => ({ body: { getReader: () => ({
    read() { return reads++ === 0 ? delayed : Promise.resolve({ done: true }); },
    cancel() { return Promise.resolve(); }, releaseLock() {},
  }) } });
  await pending.result; await settle();
  await h.complete("/api/user/auth/refresh", {}, { method: "POST" });
  release({ value: new TextEncoder().encode(JSON.stringify(self)), done: false }); await settle();
  assert.equal(h.captures().length, 0);
  await h.complete("/api/user/self", self);
  assert.equal(h.captures().length, 1);
});

test("XHR parsed JSON mode is refused because original numeric lexemes are unavailable", async () => {
  const h = harness(); await h.complete("/api/status", status);
  const xhr = new h.scope.XMLHttpRequest(); xhr.open("GET", "/api/user/self"); xhr.responseType = "json"; xhr.send();
  xhr.reply(self); await settle();
  assert.equal(h.captures().length, 0);
  assert.ok(h.messages.some((message) => message.kind === "failure"));
});

test("duplicate and dotted JSON keys cannot spoof the selected field paths", async () => {
  for (const body of [
    '{"success":true,"data":{"quota":1,"quota":2,"used_quota":0}}',
    '{"success":true,"data":{"quota":1,"qu\\u006fta":2,"used_quota":0}}',
    '{"success":true,"data.quota":1,"data.used_quota":0}',
  ]) {
    const h = harness(); await h.complete("/api/status", status); await h.complete("/api/user/self", body);
    assert.equal(h.captures().length, 0);
  }
});

test("dispose restores only owned hooks and late XHR completion cannot send", async () => {
  const h = harness(); await h.complete("/api/status", status);
  const xhr = new h.scope.XMLHttpRequest(); xhr.open("GET", "/api/user/self"); xhr.send();
  const replacement = () => "page replacement";
  h.scope.fetch = replacement;
  const before = h.messages.length; h.controller.dispose();
  assert.equal(h.scope.fetch, replacement);
  xhr.reply(self); await settle();
  assert.equal(h.messages.length, before);
});
