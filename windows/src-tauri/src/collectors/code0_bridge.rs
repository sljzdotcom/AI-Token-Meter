use super::code0_capture::Code0WebsiteCapture;

/// Supplied by the owned native callback, never decoded from page JSON.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Code0BridgeContext {
    pub connection_epoch: String,
    pub attempt_id: String,
    pub source_url: String,
    pub is_main_frame: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Code0BridgeReason {
    AuthenticationChanged,
    InvalidResponse,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Code0BridgeRejection {
    ValidationRequired,
    InvalidContext,
    InvalidPayload,
    InvalidSequence,
    InvalidState,
    Disposed,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Code0BridgeEvent {
    Control,
    Capture(Code0WebsiteCapture),
    Invalidated(Code0BridgeReason),
    Failure(Code0BridgeReason),
    Rejected(Code0BridgeRejection),
}

/// Phase-one deny-only contract. No page message can authorize a capture yet.
pub struct Code0BridgeValidator;

impl Code0BridgeValidator {
    pub fn new(
        _connection_epoch: &str,
        _attempt_id: &str,
        _nonce: &str,
        _document_id: &str,
    ) -> Self {
        Self
    }

    pub fn receive(&mut self, _payload: &[u8], _context: &Code0BridgeContext) -> Code0BridgeEvent {
        Code0BridgeEvent::Rejected(Code0BridgeRejection::ValidationRequired)
    }

    pub fn dispose(&mut self) {}
}

/// The canonical expression is embedded unchanged; no network or runtime here.
pub const CODE0_BRIDGE_SOURCE: &str = include_str!("../../../../web/code0-bridge.js");
