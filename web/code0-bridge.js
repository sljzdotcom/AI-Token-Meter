/* Shared Code0 document-start bridge. Native callers provide the transport. */
(() => {
  "use strict";
  // Deny-only test-first contract: installation does not request or publish data.
  return function installCode0Bridge(_config) {
    return Object.freeze({ dispose() {} });
  };
})()
