/* Canonical passive Code0 bridge. Native owns configuration and transport. */
(() => {
  "use strict";
  const ORIGIN = "https://console.code0.ai";
  const MAX = "9007199254740991";
  const LIMIT = 65536;
  return function installCode0Bridge(config) {
    let active = window === top && location.origin === ORIGIN && config.version === 1 &&
      typeof config.nonce === "string" && typeof config.document === "string" && typeof config.send === "function";
    let sequence = 0, requestID = 0, revision = 0, pending = null, divisor = null, statusRequest = 0;
    const refreshing = new Set();
    const readers = new Set(), cleanups = new Set();
    const encoder = new TextEncoder();
    function emit(kind, fields = {}) {
      if (!active) return;
      if (!Number.isSafeInteger(++sequence)) { halt(); return; }
      const message = { version: 1, nonce: config.nonce, document: config.document, sequence, kind, ...fields };
      if (encoder.encode(JSON.stringify(message)).byteLength > 2048) { halt(); return; }
      try { config.send(message); } catch { halt(); }
    }
    function halt() {
      active = false; pending = null; divisor = null; refreshing.clear();
      for (const cleanup of [...cleanups]) cleanup();
      for (const reader of readers) { try { void reader.cancel().catch(() => {}); } catch {} }
    }
    function end(kind, reason) { emit(kind, { reason }); halt(); }
    function fail() { end("failure", "invalidResponse"); }
    function url(value, relative = true) {
      try {
        const parsed = relative ? new URL(value, location.href) : new URL(value);
        return parsed.origin === ORIGIN && !parsed.username && !parsed.password ? parsed : null;
      } catch { return null; }
    }
    function metadata(input, init) {
      try {
        const isRequest = typeof Request !== "undefined" && input instanceof Request;
        const parsed = url(isRequest ? input.url : String(input));
        const method = String(init?.method ?? (isRequest ? input.method : "GET")).toUpperCase();
        if (!parsed) return null;
        const path = parsed.pathname;
        if ((method === "POST" && ["/api/user/login", "/api/user/login/2fa", "/api/user/auth/logout"].includes(path)) ||
            (method === "GET" && path === "/api/user/logout")) return { kind: "invalidate" };
        if (method === "POST" && path === "/api/user/auth/refresh") return { kind: "refresh" };
        if (method !== "GET" || parsed.search || parsed.hash || !["/api/status", "/api/user/self"].includes(path)) return null;
        return { kind: path === "/api/status" ? "status" : "self", endpoint: parsed.href };
      } catch { return null; }
    }
    function start(meta) {
      if (!active || !meta) return null;
      if (meta.kind === "invalidate") { end("invalidate", "authenticationChanged"); return null; }
      if (!Number.isSafeInteger(++requestID)) { halt(); return null; }
      const record = { ...meta, request: requestID, revision, eligible: refreshing.size === 0 };
      if (meta.kind === "refresh") {
        if (!Number.isSafeInteger(++revision)) { halt(); return null; }
        record.revision = revision; refreshing.add(record.request); pending = null;
        emit("refreshStart", { request: record.request, revision });
      } else if (meta.kind === "self") {
        pending = record;
        emit("selfStart", { request: record.request, revision });
      } else statusRequest = record.request;
      return record;
    }
    function publish() {
      if (!active || !pending?.values || divisor === null || refreshing.size || !pending.eligible || pending.revision !== revision) return;
      if (!Number.isFinite(pending.values.quota / divisor) || !Number.isFinite(pending.values.usedQuota / divisor)) { fail(); return; }
      emit("capture", { request: pending.request, revision, quota: pending.values.quota, usedQuota: pending.values.usedQuota, divisor });
      halt();
    }
    // Compare decimal tokens before Number can round fractional or unsafe quotas.
    function numeric(raw, integer) {
      if (raw.startsWith("-")) throw 0;
      const match = /^(\d+)(?:\.(\d+))?(?:[eE]([+-]?\d+))?$/.exec(raw);
      if (!match) throw 0;
      let digits = (match[1] + (match[2] ?? "")).replace(/^0+/, "");
      if (!digits) { if (!integer) throw 0; return 0; }
      const exponent = Number(match[3] ?? 0);
      if (!Number.isSafeInteger(exponent) || Math.abs(exponent) > LIMIT) throw 0;
      let scale = exponent - (match[2]?.length ?? 0);
      const trailing = /0+$/.exec(digits)?.[0].length ?? 0;
      if (trailing) { digits = digits.slice(0, -trailing); scale += trailing; }
      if (integer && scale < 0) throw 0;
      const integralLength = digits.length + scale;
      if (integralLength > MAX.length) throw 0;
      if (integralLength === MAX.length) {
        const integral = (digits + "0".repeat(Math.max(0, scale))).slice(0, MAX.length);
        if (integral > MAX || (integral === MAX && scale < 0)) throw 0;
      }
      const value = Number(raw);
      if (!Number.isFinite(value) || value < 0 || value > Number(MAX) || (!integer && value === 0)) throw 0;
      return value;
    }
    // Validate JSON syntax without constructing unrelated account values. Retain only
    // business success and the three specified numeric tokens; duplicate keys fail.
    function project(text, kind) {
      let at = 0, tokens = 0;
      const literalPattern = /true|false|null/y;
      const numberPattern = /-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?/y;
      const result = {};
      const ws = () => { while (/\s/.test(text[at] ?? "") && at < text.length) { if (!" \t\r\n".includes(text[at])) throw 0; at++; } };
      function string(key) {
        const begin = at++;
        while (at < text.length) {
          const c = text[at++];
          if (c === '"') return key ? JSON.parse(text.slice(begin, at)) : undefined;
          if (c.charCodeAt(0) < 32) throw 0;
          if (c === "\\") {
            const escape = text[at++];
            if (escape === "u") { if (!/^[0-9a-fA-F]{4}$/.test(text.slice(at, at + 4))) throw 0; at += 4; }
            else if (!'"\\/bfnrt'.includes(escape ?? "!") || escape === undefined) throw 0;
          }
        }
        throw 0;
      }
      function value(path, depth) {
        if (++tokens > LIMIT || depth > 64) throw 0;
        ws(); const c = text[at];
        if (c === "{") {
          at++; ws(); const keys = new Set();
          if (text[at] === "}") { at++; return; }
          while (true) {
            ws(); if (text[at] !== '"') throw 0;
            const key = string(true); if (keys.has(key)) throw 0; keys.add(key);
            ws(); if (text[at++] !== ":") throw 0;
            const next = path === "" && ["success", "data"].includes(key) ? key :
              path === "data" && ["quota", "used_quota", "quota_per_unit"].includes(key) ? `data.${key}` : "ignored";
            value(next, depth + 1);
            ws(); const separator = text[at++]; if (separator === "}") return; if (separator !== ",") throw 0;
          }
        }
        if (c === "[") {
          at++; ws(); if (text[at] === "]") { at++; return; }
          while (true) { value("ignored", depth + 1); ws(); const separator = text[at++]; if (separator === "]") return; if (separator !== ",") throw 0; }
        }
        const selected = kind === "status" ? path === "data.quota_per_unit" :
          path === "data.quota" || path === "data.used_quota";
        if (c === '"') { string(false); if (selected || path === "success") throw 0; return; }
        literalPattern.lastIndex = at;
        const literal = literalPattern.exec(text);
        if (literal) { at += literal[0].length; if (path === "success") result.success = literal[0] === "true"; else if (selected) throw 0; return; }
        numberPattern.lastIndex = at;
        const number = numberPattern.exec(text);
        if (!number) throw 0; at += number[0].length;
        if (selected) result[path] = numeric(number[0], path !== "data.quota_per_unit");
        if (path === "success") throw 0;
      }
      ws(); if (text[at] !== "{") throw 0; value("", 0); ws(); if (at !== text.length || result.success !== true) throw 0;
      if (kind === "status") { if (result["data.quota_per_unit"] === undefined) throw 0; return result["data.quota_per_unit"]; }
      if (result["data.quota"] === undefined || result["data.used_quota"] === undefined) throw 0;
      return { quota: result["data.quota"], usedQuota: result["data.used_quota"] };
    }
    function current(record) {
      return active && (record.kind === "status" ? record.request === statusRequest :
        pending === record && record.revision === revision && record.eligible && refreshing.size === 0);
    }
    function finish(record, text) {
      if (!current(record)) return;
      try {
        if (encoder.encode(text).byteLength > LIMIT) throw 0;
        const values = project(text, record.kind);
        if (!active) return;
        if (record.kind === "status") { if (record.request !== statusRequest) return; divisor = values; }
        else { if (pending !== record || record.revision !== revision || !record.eligible || refreshing.size) return; record.values = values; }
        publish();
      } catch { fail(); }
    }
    function responseOK(record, finalURL, status, contentType) {
      const final = typeof finalURL === "string" ? url(finalURL, false) : null;
      return status >= 200 && status < 300 && finalURL && final && !final.search && !final.hash &&
        final.href === record.endpoint && /^application\/(?:[\w.-]+\+)?json(?:\s*;|\s*$)/i.test(contentType ?? "");
    }
    function refreshEnd(record, status) {
      if (!active) return;
      if (!(status >= 200 && status < 300)) { fail(); return; }
      refreshing.delete(record.request); pending = null;
      emit("refreshEnd", { request: record.request, revision });
    }
    async function observeFetch(record, response) {
      if (!active) return;
      if (record.kind === "refresh") { refreshEnd(record, response.status); return; }
      if (!current(record)) return;
      if (response.redirected !== false || ["opaque", "opaqueredirect"].includes(response.type) ||
          !responseOK(record, response.url, response.status, response.headers.get("content-type"))) { fail(); return; }
      const reader = response.clone().body?.getReader();
      if (!reader) { fail(); return; }
      const decoder = new TextDecoder("utf-8", { fatal: true }); let bytes = 0, text = "", completed = false;
      readers.add(reader);
      try {
        while (current(record)) {
          const chunk = await reader.read(); if (!current(record)) return;
          if (chunk.done) { completed = true; text += decoder.decode(); finish(record, text); return; }
          bytes += chunk.value.byteLength; if (bytes > LIMIT) { fail(); return; }
          text += decoder.decode(chunk.value, { stream: true });
        }
      } catch { if (active) fail(); }
      finally { readers.delete(reader); if (!completed) { try { void reader.cancel().catch(() => {}); } catch {} } reader.releaseLock(); }
    }
    const originalFetch = window.fetch;
    function wrappedFetch(...args) {
      const meta = metadata(args[0], args[1]); const record = start(meta);
      let promise;
      try { promise = Reflect.apply(originalFetch, this, args); }
      catch (error) { if (record) fail(); throw error; }
      if (record) void promise.then(response => observeFetch(record, response), () => { if (active) fail(); }).catch(() => { if (active) fail(); });
      return promise;
    }
    const prototype = window.XMLHttpRequest?.prototype;
    const originalOpen = prototype?.open, originalSend = prototype?.send;
    const xhrMeta = new WeakMap();
    function wrappedOpen(method, input, ...rest) {
      const result = Reflect.apply(originalOpen, this, [method, input, ...rest]);
      xhrMeta.set(this, metadata(input, { method })); return result;
    }
    function wrappedSend(...args) {
      const record = start(xhrMeta.get(this));
      if (!record) return Reflect.apply(originalSend, this, args);
      const xhr = this;
      const cleanup = () => { for (const event of ["load", "error", "abort", "timeout"]) xhr.removeEventListener(event, handlers[event]); cleanups.delete(cleanup); };
      const failed = () => { cleanup(); if (active) fail(); };
      const handlers = { error: failed, abort: failed, timeout: failed, load: () => {
        cleanup(); if (!active) return;
        try {
          if (record.kind === "refresh") { refreshEnd(record, xhr.status); return; }
          if (!current(record)) return;
          if (!["", "text"].includes(xhr.responseType) || !responseOK(record, xhr.responseURL, xhr.status, xhr.getResponseHeader("content-type"))) { fail(); return; }
          const text = xhr.responseText; if (typeof text !== "string" || text.length > LIMIT) { fail(); return; } finish(record, text);
        } catch { fail(); }
      } };
      cleanups.add(cleanup);
      for (const event of Object.keys(handlers)) xhr.addEventListener(event, handlers[event]);
      try { return Reflect.apply(originalSend, this, args); } catch (error) { failed(); throw error; }
    }
    if (active) {
      window.fetch = wrappedFetch;
      if (prototype) { prototype.open = wrappedOpen; prototype.send = wrappedSend; }
      emit("ready");
    }
    return Object.freeze({ dispose() {
      halt();
      if (window.fetch === wrappedFetch) window.fetch = originalFetch;
      if (prototype?.open === wrappedOpen) prototype.open = originalOpen;
      if (prototype?.send === wrappedSend) prototype.send = originalSend;
    } });
  };
})()
