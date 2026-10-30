//! Pure, fixed-storage protocol for the selected owned-loopback HTTP401 fixture.
//!
//! This module performs no I/O and produces no native observations. Its caller
//! owns the port provenance, actual receive/send results, authority, clocks,
//! poll and I/O budgets, socket inventories, and all cleanup confirmations.
//! Compiler, tests and native behavior are UNRUN in this source-only slice.

const REQUEST_CAPACITY: usize = 4096;
const RECEIVE_STEP_CAPACITY: usize = 512;
const RESPONSE: &[u8; 139] = b"HTTP/1.1 401 Unauthorized\r\nWWW-Authenticate: Basic realm=\"code0-fixture\"\r\nContent-Length: 0\r\nConnection: close\r\nCache-Control: no-store\r\n\r\n";
const REQUEST_LINE: &[u8] = b"GET /code0-http-auth HTTP/1.1";

/// Fixed discriminators only: never include request bytes or native errors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ProtocolError {
    InvalidPort,
    ReceiveStepTooLarge,
    RequestTooLarge,
    InvalidFraming,
    LineTooLong,
    InvalidRequestLine,
    InvalidHeader,
    TooManyHeaders,
    InvalidHost,
    ForbiddenHeader,
    InvalidContentLength,
    ExtraBytes,
    PrematureEof,
    UnexpectedInput,
    UnexpectedSend,
    InvalidSendLength,
    SendFailed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct RequestReceipt {
    pub(super) request_bytes: u16,
    pub(super) header_count: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ReceiveStatus {
    Incomplete,
    Accepted(RequestReceipt),
}

/// The caller supplies an actual send result only after its native postchecks.
/// Non-progress results yield to a later poll; this module does not retry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SendOutcome {
    Sent(usize),
    WouldBlock,
    Interrupted,
    Failed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ResponseProgress {
    pub(super) sent: u16,
    pub(super) complete: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Receiving,
    Responding,
    Complete,
    Failed,
}

/// A single request and response cursor. No Debug/Clone implementation can
/// expose the private Host or the temporary receive buffer. Construct only from
/// the caller's validated owned-listener port; the type cannot establish that
/// provenance itself. It accepts no caller target, address or response bytes.
pub(super) struct Http401Protocol {
    buffer: [u8; REQUEST_CAPACITY],
    received: usize,
    scanned: usize,
    line_start: usize,
    first_line: bool,
    expected_host: [u8; 15],
    expected_host_len: usize,
    receipt: Option<RequestReceipt>,
    sent: usize,
    phase: Phase,
}

impl Http401Protocol {
    pub(super) fn new(owned_port: u16) -> Result<Self, ProtocolError> {
        if owned_port == 0 {
            return Err(ProtocolError::InvalidPort);
        }
        let mut expected_host = [0; 15];
        expected_host[..10].copy_from_slice(b"127.0.0.1:");
        let mut digits = [0; 5];
        let mut cursor = digits.len();
        let mut remaining = owned_port;
        while remaining != 0 {
            cursor -= 1;
            digits[cursor] = b'0' + (remaining % 10) as u8;
            remaining /= 10;
        }
        let digit_count = digits.len() - cursor;
        expected_host[10..10 + digit_count].copy_from_slice(&digits[cursor..]);
        Ok(Self {
            buffer: [0; REQUEST_CAPACITY],
            received: 0,
            scanned: 0,
            line_start: 0,
            first_line: true,
            expected_host,
            expected_host_len: 10 + digit_count,
            receipt: None,
            sent: 0,
            phase: Phase::Receiving,
        })
    }

    /// Feed exactly one actual receive result (at most 512 bytes). An empty
    /// result represents EOF. The caller must also reject any later buffered
    /// input; this module cannot observe bytes it was never given.
    pub(super) fn receive(&mut self, bytes: &[u8]) -> Result<ReceiveStatus, ProtocolError> {
        if self.phase != Phase::Receiving {
            return self.reject(ProtocolError::UnexpectedInput);
        }
        if bytes.is_empty() {
            return self.reject(ProtocolError::PrematureEof);
        }
        if bytes.len() > RECEIVE_STEP_CAPACITY {
            return self.reject(ProtocolError::ReceiveStepTooLarge);
        }
        if bytes.len() > REQUEST_CAPACITY - self.received {
            return self.reject(ProtocolError::RequestTooLarge);
        }
        self.buffer[self.received..self.received + bytes.len()].copy_from_slice(bytes);
        self.received += bytes.len();

        // Incremental framing visits each byte once, except a split trailing CR
        // is revisited once. Field parsing occurs only after the final CRLF.
        while self.scanned < self.received {
            let byte = self.buffer[self.scanned];
            if byte == b'\r' {
                if self.scanned + 1 == self.received {
                    break;
                }
                if self.buffer[self.scanned + 1] != b'\n' {
                    return self.reject(ProtocolError::InvalidFraming);
                }
                let content_len = self.scanned - self.line_start;
                self.scanned += 2;
                if content_len == 0 {
                    if self.first_line {
                        return self.reject(ProtocolError::InvalidRequestLine);
                    }
                    if self.scanned != self.received {
                        return self.reject(ProtocolError::ExtraBytes);
                    }
                    let parsed = parse_request(
                        &self.buffer[..self.received],
                        &self.expected_host[..self.expected_host_len],
                    );
                    self.buffer.fill(0);
                    return match parsed {
                        Ok(receipt) => {
                            self.receipt = Some(receipt);
                            self.phase = Phase::Responding;
                            Ok(ReceiveStatus::Accepted(receipt))
                        }
                        Err(error) => self.reject(error),
                    };
                }
                self.first_line = false;
                self.line_start = self.scanned;
            } else {
                if byte == b'\n'
                    || byte == 0x7f
                    || !byte.is_ascii()
                    || (byte < b' ' && byte != b'\t')
                {
                    return self.reject(ProtocolError::InvalidFraming);
                }
                // Root-ratified boundary: content bytes exclude CRLF, while
                // the overall 4096-byte budget includes every delimiter.
                let limit = if self.first_line { 64 } else { 512 };
                if self.scanned + 1 - self.line_start > limit {
                    return self.reject(ProtocolError::LineTooLong);
                }
                self.scanned += 1;
            }
        }
        if self.received == REQUEST_CAPACITY {
            return self.reject(ProtocolError::RequestTooLarge);
        }
        Ok(ReceiveStatus::Incomplete)
    }

    pub(super) fn receipt(&self) -> Option<RequestReceipt> {
        self.receipt
    }

    /// Fixed unsent bytes, available only after a valid complete request. A
    /// borrowed suffix grants no socket/native authority and proves no send.
    pub(super) fn response_suffix(&self) -> Option<&'static [u8]> {
        (self.phase == Phase::Responding).then(|| &RESPONSE[self.sent..])
    }

    /// Sum successful send lengths; completion proves only caller-reported
    /// provider acceptance of 139 bytes, never peer read or native callback.
    pub(super) fn record_send(
        &mut self,
        outcome: SendOutcome,
    ) -> Result<ResponseProgress, ProtocolError> {
        if self.phase != Phase::Responding {
            return self.reject(ProtocolError::UnexpectedSend);
        }
        match outcome {
            SendOutcome::Sent(length) => {
                if length == 0 || length > RESPONSE.len() - self.sent {
                    return self.reject(ProtocolError::InvalidSendLength);
                }
                self.sent += length;
                if self.sent == RESPONSE.len() {
                    self.phase = Phase::Complete;
                }
            }
            SendOutcome::WouldBlock | SendOutcome::Interrupted => {}
            SendOutcome::Failed => return self.reject(ProtocolError::SendFailed),
        }
        Ok(ResponseProgress {
            sent: self.sent as u16,
            complete: self.phase == Phase::Complete,
        })
    }

    fn reject<T>(&mut self, error: ProtocolError) -> Result<T, ProtocolError> {
        self.phase = Phase::Failed;
        self.receipt = None;
        self.buffer.fill(0);
        Err(error)
    }
}

impl Drop for Http401Protocol {
    fn drop(&mut self) {
        self.buffer.fill(0);
        self.expected_host.fill(0);
    }
}

// Called once, only after the bounded incremental framer found exactly one
// complete message. The parser retains no field slice or value in its result.
fn parse_request(bytes: &[u8], expected_host: &[u8]) -> Result<RequestReceipt, ProtocolError> {
    let mut lines = bytes.split(|byte| *byte == b'\n');
    if lines.next().and_then(|line| line.strip_suffix(b"\r")) != Some(REQUEST_LINE) {
        return Err(ProtocolError::InvalidRequestLine);
    }
    let mut header_count = 0;
    let mut host_seen = false;
    let mut length_seen = false;
    for raw in lines {
        let line = raw
            .strip_suffix(b"\r")
            .ok_or(ProtocolError::InvalidFraming)?;
        if line.is_empty() {
            break;
        }
        header_count += 1;
        if header_count > 32 {
            return Err(ProtocolError::TooManyHeaders);
        }
        let colon = line
            .iter()
            .position(|byte| *byte == b':')
            .ok_or(ProtocolError::InvalidHeader)?;
        let name = &line[..colon];
        if name.is_empty()
            || !name
                .iter()
                .all(|byte| byte.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(byte))
        {
            return Err(ProtocolError::InvalidHeader);
        }
        let value = trim_field_edges(&line[colon + 1..]);
        if [
            b"authorization".as_slice(),
            b"proxy-authorization",
            b"cookie",
            b"transfer-encoding",
            b"expect",
            b"upgrade",
        ]
        .iter()
        .any(|forbidden| name.eq_ignore_ascii_case(forbidden))
        {
            return Err(ProtocolError::ForbiddenHeader);
        }
        if name.eq_ignore_ascii_case(b"host") {
            if host_seen || value != expected_host {
                return Err(ProtocolError::InvalidHost);
            }
            host_seen = true;
        } else if name.eq_ignore_ascii_case(b"content-length") {
            if length_seen || value != b"0" {
                return Err(ProtocolError::InvalidContentLength);
            }
            length_seen = true;
        }
    }
    if !host_seen {
        return Err(ProtocolError::InvalidHost);
    }
    Ok(RequestReceipt {
        request_bytes: bytes.len() as u16,
        header_count,
    })
}

fn trim_field_edges(mut value: &[u8]) -> &[u8] {
    while matches!(value.first(), Some(b' ' | b'\t')) {
        value = &value[1..];
    }
    while matches!(value.last(), Some(b' ' | b'\t')) {
        value = &value[..value.len() - 1];
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID: &[u8] = b"GET /code0-http-auth HTTP/1.1\r\nHost: 127.0.0.1:43127\r\n\r\n";

    fn receive_all(
        protocol: &mut Http401Protocol,
        request: &[u8],
    ) -> Result<ReceiveStatus, ProtocolError> {
        let mut result = Ok(ReceiveStatus::Incomplete);
        for chunk in request.chunks(512) {
            result = protocol.receive(chunk);
            if result.is_err() {
                break;
            }
        }
        result
    }

    fn rejected(request: &[u8]) {
        let mut protocol = Http401Protocol::new(43127).unwrap();
        assert!(receive_all(&mut protocol, request).is_err());
        assert!(protocol.response_suffix().is_none());
        assert!(protocol.receipt().is_none());
        assert!(protocol.buffer.iter().all(|byte| *byte == 0));
    }

    // Catches premature acceptance, delimiter loss between reads, or body bytes
    // becoming eligible for the fixed challenge response.
    #[test]
    fn every_two_part_split_requires_the_complete_request() {
        for split in 1..VALID.len() {
            let mut protocol = Http401Protocol::new(43127).unwrap();
            assert_eq!(
                protocol.receive(&VALID[..split]),
                Ok(ReceiveStatus::Incomplete)
            );
            assert!(protocol.response_suffix().is_none());
            assert_eq!(
                protocol.receive(&VALID[split..]),
                Ok(ReceiveStatus::Accepted(RequestReceipt {
                    request_bytes: 56,
                    header_count: 1
                }))
            );
            assert!(protocol.buffer.iter().all(|byte| *byte == 0));
        }
    }

    // Catches a framer requiring both CR and LF in the same input chunk.
    #[test]
    fn single_byte_chunks_preserve_split_crlf() {
        let mut protocol = Http401Protocol::new(43127).unwrap();
        for byte in &VALID[..VALID.len() - 1] {
            assert_eq!(protocol.receive(&[*byte]), Ok(ReceiveStatus::Incomplete));
        }
        assert!(matches!(
            protocol.receive(b"\n"),
            Ok(ReceiveStatus::Accepted(_))
        ));
    }

    // Catches Host normalization, an externally supplied target, or an
    // accidental general-purpose HTTP request-line implementation.
    #[test]
    fn request_line_and_private_owned_host_are_exact() {
        for request in [
            b"POST /code0-http-auth HTTP/1.1\r\nHost: 127.0.0.1:43127\r\n\r\n".as_slice(),
            b"GET /code0-http-auth HTTP/1.0\r\nHost: 127.0.0.1:43127\r\n\r\n",
            b"GET /code0-http-auth?x HTTP/1.1\r\nHost: 127.0.0.1:43127\r\n\r\n",
            b"GET /code0-http-auth/ HTTP/1.1\r\nHost: 127.0.0.1:43127\r\n\r\n",
            b"GET  /code0-http-auth HTTP/1.1\r\nHost: 127.0.0.1:43127\r\n\r\n",
            b"GET http://127.0.0.1:43127/code0-http-auth HTTP/1.1\r\nHost: 127.0.0.1:43127\r\n\r\n",
            b"GET /code0-http-auth HTTP/1.1\r\nHost: localhost:43127\r\n\r\n",
            b"GET /code0-http-auth HTTP/1.1\r\nHost: 127.0.0.1:043127\r\n\r\n",
            b"GET /code0-http-auth HTTP/1.1\r\nHost: 127.0.0.1:43128\r\n\r\n",
            b"GET /code0-http-auth HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
        ] {
            rejected(request);
        }
        assert!(matches!(
            Http401Protocol::new(0),
            Err(ProtocolError::InvalidPort)
        ));
        for (port, host) in [(1, "127.0.0.1:1"), (65535, "127.0.0.1:65535")] {
            let mut protocol = Http401Protocol::new(port).unwrap();
            let request = format!("GET /code0-http-auth HTTP/1.1\r\nHost: {host}\r\n\r\n");
            assert!(matches!(
                receive_all(&mut protocol, request.as_bytes()),
                Ok(ReceiveStatus::Accepted(_))
            ));
        }
    }

    // Catches case-sensitive header handling or trimming interior whitespace.
    #[test]
    fn header_names_ignore_ascii_case_and_values_trim_only_edges() {
        let mut protocol = Http401Protocol::new(43127).unwrap();
        let request = b"GET /code0-http-auth HTTP/1.1\r\nhOsT:\t 127.0.0.1:43127 \t\r\nCoNtEnT-LeNgTh: \t0\t \r\nX-Extra: a:b\tc\r\n\r\n";
        assert!(matches!(
            receive_all(&mut protocol, request),
            Ok(ReceiveStatus::Accepted(RequestReceipt {
                header_count: 3,
                ..
            }))
        ));
        rejected(b"GET /code0-http-auth HTTP/1.1\r\nHost: 127.0.0.1: 43127\r\n\r\n");
    }

    // Catches silently allowing credential-bearing or ambiguous framing fields,
    // including names whose case differs from conventional serialization.
    #[test]
    fn forbidden_headers_and_duplicate_or_nonzero_lengths_fail_closed() {
        for header in [
            "Authorization: example",
            "pRoXy-AuThOrIzAtIoN: example",
            "Cookie: example",
            "Transfer-Encoding: chunked",
            "Expect: 100-continue",
            "Upgrade: example",
            "Host: 127.0.0.1:43127",
            "Content-Length: 1",
            "Content-Length: 00",
            "Content-Length: +0",
            "Content-Length:",
            "Content-Length: 0, 0",
            "Content-Length: 0\r\ncontent-length: 0",
        ] {
            let request = format!(
                "GET /code0-http-auth HTTP/1.1\r\nHost: 127.0.0.1:43127\r\n{header}\r\n\r\n"
            );
            rejected(request.as_bytes());
        }
        rejected(b"GET /code0-http-auth HTTP/1.1\r\n\r\n");
        rejected(b"GET /code0-http-auth HTTP/1.1\r\nX-Extra: yes\r\n\r\n");
    }

    // Catches accepting invalid token names, obs-fold, controls or loose line endings.
    #[test]
    fn malformed_ascii_headers_and_framing_are_rejected() {
        for header in [
            b" Host: 127.0.0.1:43127".as_slice(),
            b"\tX: continuation",
            b"X : value",
            b": value",
            b"X(value): value",
            b"X-Value",
            b"X: one\nY: two",
            b"X: one\rY: two",
            b"X: \x00",
            b"X: \x01",
            b"X: \x0b",
            b"X: \x7f",
            b"X: \x80",
        ] {
            let mut request =
                b"GET /code0-http-auth HTTP/1.1\r\nHost: 127.0.0.1:43127\r\n".to_vec();
            request.extend_from_slice(header);
            request.extend_from_slice(b"\r\n\r\n");
            rejected(&request);
        }
        rejected(b"GET /code0-http-auth HTTP/1.1\nHost: 127.0.0.1:43127\n\n");
    }

    // Catches rejecting valid token punctuation or treating an interior colon
    // as a second delimiter rather than part of an ignored header value.
    #[test]
    fn all_token_punctuation_and_visible_ascii_values_are_allowed() {
        let mut protocol = Http401Protocol::new(43127).unwrap();
        let request = b"GET /code0-http-auth HTTP/1.1\r\nHost: 127.0.0.1:43127\r\n!#$%&'*+-.^_`|~AZaz09: !~:ok\r\n\r\n";
        assert!(matches!(
            receive_all(&mut protocol, request),
            Ok(ReceiveStatus::Accepted(_))
        ));
    }

    // Catches off-by-two line limits: content excludes CRLF; the total includes it.
    #[test]
    fn header_content_512_is_accepted_and_513_is_rejected() {
        for (value_len, accepted) in [(510, true), (511, false)] {
            let mut request =
                b"GET /code0-http-auth HTTP/1.1\r\nHost: 127.0.0.1:43127\r\nX:".to_vec();
            request.extend(std::iter::repeat_n(b'a', value_len));
            request.extend_from_slice(b"\r\n\r\n");
            let mut protocol = Http401Protocol::new(43127).unwrap();
            assert_eq!(
                matches!(
                    receive_all(&mut protocol, &request),
                    Ok(ReceiveStatus::Accepted(_))
                ),
                accepted
            );
        }
    }

    // Catches reading beyond the request-line content cap before any CRLF.
    // A 64-byte partial line is still incomplete, never a valid alternate GET.
    #[test]
    fn request_line_64_is_incomplete_and_65_is_rejected() {
        let mut protocol = Http401Protocol::new(43127).unwrap();
        assert_eq!(protocol.receive(&[b'a'; 64]), Ok(ReceiveStatus::Incomplete));
        assert_eq!(protocol.receive(b"a"), Err(ProtocolError::LineTooLong));
        assert!(protocol.response_suffix().is_none());
        assert!(protocol.buffer.iter().all(|byte| *byte == 0));
    }

    // Catches header-count accounting that includes the request or blank line,
    // or silently permits a thirty-third nonempty header.
    #[test]
    fn exactly_32_headers_are_accepted_and_33_are_rejected() {
        for (extra, accepted) in [(31, true), (32, false)] {
            let mut request =
                b"GET /code0-http-auth HTTP/1.1\r\nHost: 127.0.0.1:43127\r\n".to_vec();
            for _ in 0..extra {
                request.extend_from_slice(b"X: a\r\n");
            }
            request.extend_from_slice(b"\r\n");
            let mut protocol = Http401Protocol::new(43127).unwrap();
            assert_eq!(
                matches!(
                    receive_all(&mut protocol, &request),
                    Ok(ReceiveStatus::Accepted(_))
                ),
                accepted
            );
            if accepted {
                assert_eq!(protocol.receipt().unwrap().header_count, 32);
            }
        }
    }

    // Catches total-byte overflow even when individual lines fit; padding uses
    // seven 512-byte content lines and one 440/441-byte content line.
    #[test]
    fn total_4096_is_accepted_and_4097_is_rejected() {
        for (tail_value_len, accepted) in [(438, true), (439, false)] {
            let mut request =
                b"GET /code0-http-auth HTTP/1.1\r\nHost: 127.0.0.1:43127\r\n".to_vec();
            for _ in 0..7 {
                request.extend_from_slice(b"X:");
                request.extend(std::iter::repeat_n(b'a', 510));
                request.extend_from_slice(b"\r\n");
            }
            request.extend_from_slice(b"X:");
            request.extend(std::iter::repeat_n(b'a', tail_value_len));
            request.extend_from_slice(b"\r\n\r\n");
            assert_eq!(request.len(), if accepted { 4096 } else { 4097 });
            let mut protocol = Http401Protocol::new(43127).unwrap();
            assert_eq!(
                matches!(
                    receive_all(&mut protocol, &request),
                    Ok(ReceiveStatus::Accepted(_))
                ),
                accepted
            );
            if accepted {
                assert_eq!(protocol.receipt().unwrap().request_bytes, 4096);
            }
        }
    }

    // Catches body/pipelining acceptance both in the terminating read and after
    // acceptance; revocation cannot undo bytes a caller already sent.
    #[test]
    fn any_buffered_suffix_or_later_input_revokes_response_eligibility() {
        for suffix in [b"x".as_slice(), b"\r\n", VALID] {
            let mut request = VALID.to_vec();
            request.extend_from_slice(suffix);
            rejected(&request);
        }
        let mut protocol = Http401Protocol::new(43127).unwrap();
        assert!(protocol.receive(VALID).is_ok());
        assert_eq!(protocol.receive(b"x"), Err(ProtocolError::UnexpectedInput));
        assert!(protocol.response_suffix().is_none());
    }

    // Catches a silent preconnect or premature EOF accidentally authorizing a
    // challenge, and a caller violating the maximum actual receive step.
    #[test]
    fn eof_and_oversized_receive_step_are_terminal() {
        for prefix in [b"".as_slice(), &VALID[..VALID.len() - 1]] {
            let mut protocol = Http401Protocol::new(43127).unwrap();
            if !prefix.is_empty() {
                assert!(protocol.receive(prefix).is_ok());
            }
            assert_eq!(protocol.receive(b""), Err(ProtocolError::PrematureEof));
            assert!(protocol.response_suffix().is_none());
            assert!(protocol.buffer.iter().all(|byte| *byte == 0));
            assert!(protocol.receive(VALID).is_err());
        }
        let mut protocol = Http401Protocol::new(43127).unwrap();
        assert_eq!(
            protocol.receive(&[b'a'; 513]),
            Err(ProtocolError::ReceiveStepTooLarge)
        );
        assert!(protocol.response_suffix().is_none());
    }

    // Catches response formatting/body drift and resending already acknowledged
    // bytes after partial sends or non-progress outcomes.
    #[test]
    fn response_is_exact_139_bytes_and_partial_sends_advance_only_the_suffix() {
        let mut protocol = Http401Protocol::new(43127).unwrap();
        assert!(protocol.response_suffix().is_none());
        protocol.receive(VALID).unwrap();
        assert!(protocol.response_suffix() == Some(b"HTTP/1.1 401 Unauthorized\r\nWWW-Authenticate: Basic realm=\"code0-fixture\"\r\nContent-Length: 0\r\nConnection: close\r\nCache-Control: no-store\r\n\r\n".as_slice()));
        assert_eq!(
            protocol.record_send(SendOutcome::Sent(27)),
            Ok(ResponseProgress {
                sent: 27,
                complete: false
            })
        );
        assert!(
            protocol
                .response_suffix()
                .unwrap()
                .starts_with(b"WWW-Authenticate:")
        );
        for outcome in [SendOutcome::WouldBlock, SendOutcome::Interrupted] {
            assert_eq!(
                protocol.record_send(outcome),
                Ok(ResponseProgress {
                    sent: 27,
                    complete: false
                })
            );
        }
        assert_eq!(
            protocol.record_send(SendOutcome::Sent(111)),
            Ok(ResponseProgress {
                sent: 138,
                complete: false
            })
        );
        assert_eq!(protocol.response_suffix(), Some(b"\n".as_slice()));
        assert_eq!(
            protocol.record_send(SendOutcome::Sent(1)),
            Ok(ResponseProgress {
                sent: 139,
                complete: true
            })
        );
        assert!(protocol.response_suffix().is_none());
        assert!(protocol.record_send(SendOutcome::Sent(1)).is_err());
    }

    // Catches zero/error/overflow send results creating completion, or reuse of
    // a failed protocol object to issue another challenge.
    #[test]
    fn zero_failed_or_oversized_send_is_sticky_failure() {
        for outcome in [
            SendOutcome::Sent(0),
            SendOutcome::Sent(140),
            SendOutcome::Failed,
        ] {
            let mut protocol = Http401Protocol::new(43127).unwrap();
            protocol.receive(VALID).unwrap();
            assert!(protocol.record_send(outcome).is_err());
            assert!(protocol.response_suffix().is_none());
            assert!(protocol.receipt().is_none());
            assert!(protocol.record_send(SendOutcome::Sent(139)).is_err());
            assert!(protocol.receive(VALID).is_err());
        }
        let mut protocol = Http401Protocol::new(43127).unwrap();
        assert_eq!(
            protocol.record_send(SendOutcome::Sent(1)),
            Err(ProtocolError::UnexpectedSend)
        );
        assert!(protocol.receive(VALID).is_err());

        let mut protocol = Http401Protocol::new(43127).unwrap();
        protocol.receive(VALID).unwrap();
        protocol.record_send(SendOutcome::Sent(138)).unwrap();
        assert_eq!(
            protocol.record_send(SendOutcome::Sent(2)),
            Err(ProtocolError::InvalidSendLength)
        );
        assert!(protocol.response_suffix().is_none());
        assert!(protocol.receipt().is_none());
    }
}
