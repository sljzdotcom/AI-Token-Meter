// Same production-facing legacy single-accept core. No bounded-two-accept policy.
// Pure state borrowing keeps native Owner/current/Active wrappers in N unchanged.
use super::*;

record!(TransportSlot {ordinal:u8,accepted:bool,chosen:bool,owned_at_final:bool,accepted_local_matched:Option<bool>,accepted_peer_matched:Option<bool>,linger_off:Option<bool>,nonblocking:Option<bool>,setup_calls:u16,recv_attempts:u16,send_attempts:u16,nonprogress_count:u16,seal_attempt_count:u16,recv_bytes:u16,sent_bytes:u16,eof_count:u16,parsed_requests:u16,seal_final_count:u16,close_attempts:u16,close_successes:u16,seal_class:Option<&'static str>});
record!(TransportObservation {version:u8,phase:&'static str,accepted_total:u16,chosen_slot:Option<u8>,winner_bindings:u16,auxiliary_count:u16,listener_retirement:Option<&'static str>,listener_retired:Option<bool>,seal_complete:Option<bool>,current:Option<bool>,original_timely:Option<bool>,accept_attempts:u16,recv_attempts:u16,send_attempts:u16,received_bytes:u16,first_chunk_bytes:u16,socket_peak:u16,pending_accept:bool,slots:[TransportSlot;2]});
impl TransportObservation {
    pub(super) fn new() -> Self {
        Self {
            version: 1,
            phase: "Preparing",
            slots: [
                TransportSlot {
                    ordinal: 1,
                    ..TransportSlot::default()
                },
                TransportSlot {
                    ordinal: 2,
                    ..TransportSlot::default()
                },
            ],
            ..Self::default()
        }
    }
}

pub(super) struct TransportParts<'a> {
    listener: &'a mut Option<SocketLease>,
    stream: &'a mut Option<SocketLease>,
    startup: &'a mut bool,
    startup_cleanup_attempted: &'a mut bool,
    setup: &'a mut u8,
    accepted_setup: &'a mut u8,
    budget: &'a mut ServiceBudget,
    in_action: &'a mut bool,
    parser: &'a mut Option<Http401Protocol>,
    received: &'a mut usize,
    port: &'a mut Option<u16>,
    url: &'a mut Option<String>,
    server: &'a mut Server,
    phase: &'a mut &'static str,
    observation: &'a mut TransportObservation,
}
impl State {
    pub(super) fn transport_parts(&mut self) -> TransportParts<'_> {
        TransportParts {
            listener: &mut self.listener,
            stream: &mut self.stream,
            startup: &mut self.startup,
            startup_cleanup_attempted: &mut self.startup_cleanup_attempted,
            setup: &mut self.setup,
            accepted_setup: &mut self.accepted_setup,
            budget: &mut self.budget,
            in_action: &mut self.in_action,
            parser: &mut self.parser,
            received: &mut self.received,
            port: &mut self.port,
            url: &mut self.url,
            server: &mut self.facts.server,
            phase: &mut self.facts.phase,
            observation: &mut self.transport_observation,
        }
    }
}
pub(super) enum CoreNext {
    Return,
    Idle,
    Operation(Operation),
}
pub(super) fn core_next(
    navigate_reserved: bool,
    committed: bool,
    s: &TransportParts<'_>,
) -> CoreNext {
    let operation = if *s.setup < 9 {
        Some(Operation::Setup(
            *s.setup,
            s.listener.as_ref().map(|v| v.handle),
        ))
    } else if !navigate_reserved {
        None
    } else if !committed {
        return CoreNext::Return;
    } else if s.stream.is_none() {
        s.listener.as_ref().map(|v| Operation::Accept(v.handle))
    } else if *s.accepted_setup < 4 {
        s.stream
            .as_ref()
            .map(|v| Operation::Accepted(*s.accepted_setup, v.handle))
    } else if s.server.response_complete == Some(true) {
        return CoreNext::Return;
    } else if let Some(bytes) = s.parser.as_ref().and_then(Http401Protocol::response_suffix) {
        s.stream.as_ref().map(|v| Operation::Send(v.handle, bytes))
    } else if *s.received < 4096 {
        s.stream
            .as_ref()
            .map(|v| Operation::Receive(v.handle, 512.min(4096 - *s.received)))
    } else {
        None
    };
    operation.map_or(CoreNext::Idle, CoreNext::Operation)
}
pub(super) fn core_reserve(
    s: &mut TransportParts<'_>,
    operation: Operation,
    now: Instant,
) -> Result<bool, ()> {
    let io = matches!(
        operation,
        Operation::Accept(_) | Operation::Receive(..) | Operation::Send(..)
    );
    if !s.budget.reserve(now, io)? {
        return Ok(false);
    }
    *s.in_action = true;
    if io {
        s.server.io_attempts = s.budget.io;
    } else {
        match operation {
            Operation::Setup(index, _) => {
                s.server.setup_calls += 1;
                match index {
                    0 => s.server.startup_attempts += 1,
                    1 => s.server.socket_creates += 1,
                    6 => s.server.binds += 1,
                    8 => s.server.listens += 1,
                    _ => {}
                }
            }
            Operation::Accepted(..) => s.server.accepted_setup_calls += 1,
            _ => {}
        }
    }

    match operation {
        Operation::Accept(_) => {
            s.observation.accept_attempts += 1;
            s.observation.pending_accept = true;
        }
        Operation::Receive(..) => {
            s.observation.recv_attempts += 1;
            s.observation.slots[0].recv_attempts += 1;
        }
        Operation::Send(..) => {
            s.observation.send_attempts += 1;
            s.observation.slots[0].send_attempts += 1;
        }
        Operation::Accepted(..) => s.observation.slots[0].setup_calls += 1,
        _ => {}
    }
    Ok(true)
}
pub(super) fn core_inventory(s: &mut TransportParts<'_>) {
    s.server.sockets_remaining = s.listener.as_ref().is_some_and(SocketLease::owned) as u16
        + s.stream.as_ref().is_some_and(SocketLease::owned) as u16;
    s.server.startup_remaining = u16::from(*s.startup);
    s.observation.socket_peak = s.observation.socket_peak.max(s.server.sockets_remaining);
    s.observation.slots[0].owned_at_final = s.stream.as_ref().is_some_and(SocketLease::owned);
    s.observation.phase = match *s.phase {
        "Armed" => "Seeking",
        "Writing" => "Responding",
        "AwaitAuth" => "AwaitAuth",
        "Revoked" => "Revoked",
        "Closed" => "Closed",
        _ => "Preparing",
    };
}
pub(super) fn core_inventory_return(
    s: &mut TransportParts<'_>,
    operation: Operation,
    outcome: &Outcome,
) {
    if let Operation::Setup(0, _) = operation {
        s.server.startup_succeeded = Some(outcome.ok);
        if outcome.ok {
            *s.startup = true;
        }
        s.server.version22 = outcome.version.map(|v| v == 0x0202);
    }
    match operation {
        Operation::Setup(1, _) => {
            if let Some(handle) = outcome.socket {
                *s.listener = Some(SocketLease::new(handle, 1));
            }
        }
        Operation::Accept(_) => {
            if let Some(handle) = outcome.socket {
                *s.stream = Some(SocketLease::new(handle, 2));
                s.server.accepts += 1;
            }
        }
        _ => {}
    }
    match operation {
        Operation::Setup(2, _) => s.server.exclusive_ack = Some(outcome.ok),
        Operation::Setup(4, _) => s.server.listener_linger_off = outcome.linger_off,
        Operation::Setup(5, _) => s.server.listener_nonblocking = Some(outcome.ok),
        Operation::Setup(7, _) => {
            s.server.bound_endpoint_matched = outcome
                .address
                .as_ref()
                .map(|a| address_matches(a, None) && a.sin_port != 0)
        }
        Operation::Setup(8, _) => s.server.listen_succeeded = Some(outcome.ok),
        Operation::Accepted(1, _) => s.server.stream_linger_off = outcome.linger_off,
        Operation::Accepted(2, _) => s.server.stream_nonblocking = Some(outcome.ok),
        Operation::Accepted(3, _) => {
            s.server.accepted_local_matched = outcome
                .address
                .as_ref()
                .map(|a| address_matches(a, *s.port))
        }
        Operation::Accept(_) => {
            if outcome.socket.is_some() {
                s.server.accepted_peer_matched = Some(
                    outcome
                        .address
                        .as_ref()
                        .is_some_and(|a| address_matches(a, None)),
                );
            }
        }
        _ => {}
    }

    if matches!(operation, Operation::Accept(_)) {
        s.observation.pending_accept = false;
        if outcome.socket.is_some() {
            s.observation.accepted_total += 1;
            s.observation.slots[0].accepted = true;
        }
    }
    let slot = &mut s.observation.slots[0];
    slot.accepted_local_matched = s.server.accepted_local_matched;
    slot.accepted_peer_matched = s.server.accepted_peer_matched;
    slot.linger_off = s.server.stream_linger_off;
    slot.nonblocking = s.server.stream_nonblocking;
    core_inventory(s);
}
pub(super) fn core_matches(s: &TransportParts<'_>, operation: Operation) -> bool {
    match operation {
        Operation::Setup(index, handle) => {
            *s.setup == index
                && (index <= 1
                    || handle == s.listener.as_ref().filter(|v| v._id == 1).map(|v| v.handle))
        }
        Operation::Accepted(index, handle) => {
            *s.accepted_setup == index
                && s.stream
                    .as_ref()
                    .is_some_and(|v| v._id == 2 && v.handle == handle)
        }
        Operation::Accept(handle) => s
            .listener
            .as_ref()
            .is_some_and(|v| v._id == 1 && v.handle == handle),
        Operation::Receive(handle, _) | Operation::Send(handle, _) => s
            .stream
            .as_ref()
            .is_some_and(|v| v._id == 2 && v.handle == handle),
    }
}
pub(super) fn core_settle(
    s: &mut TransportParts<'_>,
    operation: Operation,
    outcome: &Outcome,
    current: bool,
    timely: bool,
) -> Option<&'static str> {
    let nonprogress = matches!(outcome.error, Some(ws::WSAEWOULDBLOCK | ws::WSAEINTR))
        && matches!(
            operation,
            Operation::Accept(_) | Operation::Receive(..) | Operation::Send(..)
        );
    s.observation.current = Some(s.observation.current.unwrap_or(true) && current);
    // Clock and ownership observations have distinct real producers.
    s.observation.original_timely = Some(s.observation.original_timely.unwrap_or(true) && timely);
    if matches!(operation, Operation::Receive(..) | Operation::Send(..)) && nonprogress {
        s.observation.slots[0].nonprogress_count += 1;
    }
    if matches!(operation, Operation::Receive(..)) && outcome.ok {
        s.observation.slots[0].recv_bytes = s.observation.slots[0]
            .recv_bytes
            .saturating_add(outcome.length as u16)
            .min(4097);
        s.observation.received_bytes = s.observation.slots[0].recv_bytes;
        if outcome.eof {
            s.observation.slots[0].eof_count = 1;
        }
        if current && outcome.length > 0 && s.observation.winner_bindings == 0 {
            s.observation.winner_bindings = 1;
            s.observation.chosen_slot = Some(1);
            s.observation.first_chunk_bytes = outcome.length as u16;
            s.observation.slots[0].chosen = true;
            s.observation.auxiliary_count = 0;
            s.observation.seal_complete = Some(true);
        }
    }
    let mut failed = None;
    // Actual successful provider lengths remain honest even if authority died.
    if matches!(operation, Operation::Receive(..)) && outcome.ok {
        *s.received = (*s.received).saturating_add(outcome.length);
        s.server.request_bytes = (*s.received).min(4097) as u16;
    }
    if matches!(operation, Operation::Send(..)) && outcome.ok {
        s.server.sent_bytes = s
            .server
            .sent_bytes
            .saturating_add(outcome.length as u16)
            .min(140);
    }
    if !current {
        failed = Some("http401Binding");
    } else if !outcome.ok && !nonprogress {
        if matches!(operation, Operation::Send(..)) {
            let _ = s.parser.as_mut().unwrap().record_send(SendOutcome::Failed);
        }
        failed = Some("http401Socket");
    } else if nonprogress {
        if matches!(operation, Operation::Send(..)) {
            let outcome = if outcome.error == Some(ws::WSAEWOULDBLOCK) {
                SendOutcome::WouldBlock
            } else {
                SendOutcome::Interrupted
            };
            if s.parser.as_mut().unwrap().record_send(outcome).is_err() {
                failed = Some("http401Wire");
            }
        }
    } else {
        match operation {
            Operation::Setup(index, _) => {
                let valid = match index {
                    0 => s.server.version22 == Some(true),
                    2 => {
                        s.server.exclusive_ack = Some(outcome.ok);
                        outcome.ok
                    }
                    4 => {
                        s.server.listener_linger_off = outcome.linger_off;
                        outcome.linger_off == Some(true)
                    }
                    5 => {
                        s.server.listener_nonblocking = Some(outcome.ok);
                        outcome.ok
                    }
                    7 => {
                        let valid = outcome
                            .address
                            .as_ref()
                            .is_some_and(|a| address_matches(a, None) && a.sin_port != 0);
                        s.server.bound_endpoint_matched = Some(valid);
                        if valid {
                            let port = u16::from_be(outcome.address.as_ref().unwrap().sin_port);
                            *s.port = Some(port);
                            *s.url = Some(format!("http://127.0.0.1:{port}/code0-http-auth"));
                            *s.parser = Http401Protocol::new(port).ok();
                        }
                        valid && s.parser.is_some()
                    }
                    8 => {
                        s.server.listen_succeeded = Some(outcome.ok);
                        outcome.ok
                    }
                    _ => outcome.ok,
                };
                if valid {
                    *s.setup += 1;
                } else {
                    failed = Some("http401Socket");
                }
            }
            Operation::Accept(_) => {
                let valid = outcome
                    .address
                    .as_ref()
                    .is_some_and(|a| address_matches(a, None));
                s.server.accepted_peer_matched = Some(valid);
                if !valid {
                    failed = Some("http401Socket");
                }
            }
            Operation::Accepted(index, _) => {
                let valid = match index {
                    1 => {
                        s.server.stream_linger_off = outcome.linger_off;
                        outcome.linger_off == Some(true)
                    }
                    2 => {
                        s.server.stream_nonblocking = Some(outcome.ok);
                        outcome.ok
                    }
                    3 => {
                        let good = outcome
                            .address
                            .as_ref()
                            .is_some_and(|a| address_matches(a, *s.port));
                        s.server.accepted_local_matched = Some(good);
                        good
                    }
                    _ => outcome.ok,
                };
                if valid {
                    *s.accepted_setup += 1;
                } else {
                    failed = Some("http401Socket");
                }
            }
            Operation::Receive(..) => {
                if outcome.eof {
                    let _ = s.parser.as_mut().unwrap().receive(&[]);
                    s.server.request_valid = Some(false);
                    failed = Some("http401Request");
                } else {
                    match s
                        .parser
                        .as_mut()
                        .unwrap()
                        .receive(&outcome.bytes[..outcome.length])
                    {
                        Ok(ReceiveStatus::Incomplete) => {}
                        Ok(ReceiveStatus::Accepted(receipt)) => {
                            s.server.parsed_requests += 1;
                            s.server.header_count = u16::from(receipt.header_count);
                            s.server.request_valid =
                                Some(s.parser.as_ref().unwrap().receipt() == Some(receipt));
                            *s.phase = "Writing";
                        }
                        Err(_) => {
                            s.server.request_valid = Some(false);
                            failed = Some("http401Request");
                        }
                    }
                }
            }
            Operation::Send(..) => {
                let result = s
                    .parser
                    .as_mut()
                    .unwrap()
                    .record_send(SendOutcome::Sent(outcome.length));
                match result {
                    Ok(progress) => {
                        if progress.complete {
                            s.server.response_complete = Some(true);
                            *s.phase = "AwaitAuth";
                        }
                    }
                    Err(_) => {
                        s.server.response_complete = Some(false);
                        failed = Some("http401Wire");
                    }
                }
            }
        }
    }
    *s.in_action = false;
    s.observation.slots[0].parsed_requests = s.server.parsed_requests;
    s.observation.slots[0].sent_bytes = s.server.sent_bytes;
    core_inventory(s);
    failed
}
pub(super) fn core_retire_listener(operation: Operation, outcome: &Outcome) -> bool {
    matches!(operation, Operation::Accept(_)) && outcome.socket.is_some()
}
pub(super) fn core_close_reserve(s: &mut TransportParts<'_>, listener: bool) -> Option<ws::SOCKET> {
    let lease = if listener {
        &mut s.listener
    } else {
        &mut s.stream
    };
    let handle = lease.as_mut().and_then(SocketLease::reserve_close);
    if handle.is_some() {
        if listener {
            s.server.listener_close_attempts += 1;
            if s.observation.listener_retirement.is_none() {
                s.observation.listener_retirement = Some(if s.server.accepts == 1 {
                    "legacyFirstAccept"
                } else {
                    "revoked"
                });
            }
        } else {
            s.server.stream_close_attempts += 1;
            s.observation.slots[0].close_attempts += 1;
        }
        *s.in_action = true;
    }
    handle
}
pub(super) fn core_close_returned(s: &mut TransportParts<'_>, listener: bool, ok: bool) {
    let lease = if listener {
        s.listener.as_mut()
    } else {
        s.stream.as_mut()
    };
    if let Some(lease) = lease {
        lease.close_returned(ok);
    }
    if ok {
        if listener {
            s.server.listener_close_successes += 1;
            s.observation.listener_retired = Some(true);
        } else {
            s.server.stream_close_successes += 1;
            s.observation.slots[0].close_successes += 1;
        }
    }
    core_inventory(s);
}
pub(super) fn core_cleanup_reserve(s: &mut TransportParts<'_>) -> bool {
    core_inventory(s);
    if *s.startup
        && !*s.startup_cleanup_attempted
        && s.server.sockets_remaining == 0
        && !*s.in_action
    {
        *s.startup_cleanup_attempted = true;
        *s.in_action = true;
        s.server.cleanup_attempts += 1;
        true
    } else {
        false
    }
}
pub(super) fn core_cleanup_returned(s: &mut TransportParts<'_>, ok: bool) {
    if ok {
        *s.startup = false;
        s.server.cleanup_successes += 1;
    }
    core_inventory(s);
}

#[cfg(test)]
pub(super) mod controlled {
    use super::*;
    use std::rc::Weak;

    const RESPONSE: &[u8; 139] = b"HTTP/1.1 401 Unauthorized\r\nWWW-Authenticate: Basic realm=\"code0-fixture\"\r\nContent-Length: 0\r\nConnection: close\r\nCache-Control: no-store\r\n\r\n";
    const TURN_LIMIT: usize = 8192;
    const SLEEP_MAX: Duration = Duration::from_millis(25);

    record!(ClientQ {same_record:Option<bool>,same_attempt:Option<bool>,endpoint_argument_matched:Option<bool>,nonblocking_ack:Option<bool>,current_before:Option<bool>,current_after:Option<bool>,time_before:Option<bool>,time_after:Option<bool>,post_drop_current:Option<bool>});
    record!(ClientStartup {calls:u16,returned:Option<i32>,version:Option<u16>,version_matched:Option<bool>,succeeded:Option<bool>,reference_owned:bool});
    record!(ClientBudgets {setup_calls:u16,setup_error_reads:u16,io_calls:u16,io_error_reads:u16,primary_operations:u16,admission_denials:u16,spacing_qualified:Option<bool>,original_clock_matched:Option<bool>,overflow:bool});
    record!(Ownership {server_peak:u16,client_peak:u16,combined_peak:u16,remaining_server_sockets:u16,remaining_client_sockets:u16,in_flight_client_actions:u16,completion_obligations:u16,harness_reference_held_at_server_cleanup:Option<bool>,no_ambient_startup_reliance:Option<bool>});
    record!(ClientConnect {calls:u16,returned:Option<i32>,last_error:Option<i32>,disposition:&'static str,entered_seq:Option<u16>,returned_seq:Option<u16>,at_ms:Option<u64>,q:ClientQ});
    record!(ClientSelect {calls:u16,no_ready:u16,error_reads:u16,last_return:Option<i32>,last_error:Option<i32>,write_count:Option<u32>,except_count:Option<u32>,write_current_member:Option<bool>,except_current_member:Option<bool>,ready_class:&'static str,ready_seq:Option<u16>,ready_at_ms:Option<u64>,q:ClientQ});
    record!(ClientSoError {calls:u16,returned:Option<i32>,last_error:Option<i32>,length:Option<i32>,buffer_value:Option<i32>,value_available:Option<bool>,from_ready_seq:Option<u16>,entered_seq:Option<u16>,returned_seq:Option<u16>,at_ms:Option<u64>,query_consumed:bool,obligation_cancelled:bool,completion_count:u16,completion:&'static str,q:ClientQ});
    record!(ClientData {send_calls:u16,recv_calls:u16,send_error_reads:u16,recv_error_reads:u16,sent_bytes:u16,received_bytes:u16,request_matched:Option<bool>,response_matched:Option<bool>,original_timely:Option<bool>});
    record!(ClientClose {calls:u16,returned:Option<i32>,last_error:Option<i32>,qualified:Option<bool>,owned_at_final:bool,in_flight:bool});
    record!(ClientReceipt {ordinal:u8,socket_calls:u16,socket_created:Option<bool>,linger_set_calls:u16,linger_set_ack:Option<bool>,linger_read_calls:u16,linger_read_ack:Option<bool>,nonblocking_calls:u16,nonblocking_ack:Option<bool>,setup_error_reads:u16,connect:ClientConnect,select:ClientSelect,so_error:ClientSoError,data:ClientData,close:ClientClose});
    record!(ClientCleanup {calls:u16,returned:Option<i32>,last_error:Option<i32>,qualified:Option<bool>,reference_remaining:u16,close_error_reads:u16,cleanup_error_reads:u16,eligible_before:Option<bool>,no_client_use_after_attempt:Option<bool>});
    record!(ClientHarness {version:u8,tier:&'static str,startup:ClientStartup,budgets:ClientBudgets,ownership:Ownership,slots:[ClientReceipt;2],cleanup:ClientCleanup});
    record!(ControlledTiming {target_at_ms:Option<u64>,first_close_at_ms:Option<u64>,final_at_ms:Option<u64>,cleanup_elapsed_ms:Option<u64>,original_clock_matched:Option<bool>,close_immediately_after_target:Option<bool>,first_close_adopted:Option<bool>,within_close_deadline:Option<bool>});
    record!(ControlledRaw {schema:u8,run_id:String,case_id:&'static str,variant:&'static str,result:&'static str,failure:&'static str,complete:bool,unknown:bool,overflow:bool,server:Server,transport:TransportObservation,client_harness:ClientHarness,timing:ControlledTiming});

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub(in super::super) enum Connection {
        Unobserved,
        Pending,
        Connected,
        Refused,
        Failed,
    }

    struct Client {
        id: u8,
        lease: Option<SocketLease>,
        setup: u8,
        connection: Connection,
        in_flight: bool,
        action_seq: Option<u16>,
    }
    impl Client {
        fn new(id: u8) -> Self {
            Self {
                id,
                lease: None,
                setup: 0,
                connection: Connection::Unobserved,
                in_flight: false,
                action_seq: None,
            }
        }
    }

    // Storage only. Policy selection, accounting, result settlement and owned
    // server disposition all use the same functions called by native advance.
    struct ServerStorage {
        listener: Option<SocketLease>,
        stream: Option<SocketLease>,
        startup: bool,
        startup_cleanup_attempted: bool,
        setup: u8,
        accepted_setup: u8,
        budget: ServiceBudget,
        in_action: bool,
        parser: Option<Http401Protocol>,
        received: usize,
        port: Option<u16>,
        url: Option<String>,
        facts: Server,
        phase: &'static str,
        observation: TransportObservation,
    }
    impl ServerStorage {
        fn new() -> Self {
            Self {
                listener: None,
                stream: None,
                startup: false,
                startup_cleanup_attempted: false,
                setup: 0,
                accepted_setup: 0,
                budget: ServiceBudget::default(),
                in_action: false,
                parser: None,
                received: 0,
                port: None,
                url: None,
                facts: Server::default(),
                phase: "Preparing",
                observation: TransportObservation::new(),
            }
        }
        fn parts(&mut self) -> TransportParts<'_> {
            TransportParts {
                listener: &mut self.listener,
                stream: &mut self.stream,
                startup: &mut self.startup,
                startup_cleanup_attempted: &mut self.startup_cleanup_attempted,
                setup: &mut self.setup,
                accepted_setup: &mut self.accepted_setup,
                budget: &mut self.budget,
                in_action: &mut self.in_action,
                parser: &mut self.parser,
                received: &mut self.received,
                port: &mut self.port,
                url: &mut self.url,
                server: &mut self.facts,
                phase: &mut self.phase,
                observation: &mut self.observation,
            }
        }
    }

    enum ClientOp {
        Setup(u8),
        Connect(ws::SOCKADDR_IN),
        Select,
        SoError,
        Send { bytes: [u8; 128], len: usize },
        Receive(usize),
    }
    struct ClientAction {
        ordinal: usize,
        record: u8,
        handle: Option<ws::SOCKET>,
        kind: ClientOp,
        attempt: Weak<()>,
        entered: u16,
        before: Instant,
        before_current: bool,
        endpoint: bool,
        endpoint_port: Option<u16>,
        nonblocking: bool,
        from_ready: Option<u16>,
    }
    struct NativeClientReturn {
        returned: i32,
        error: Option<i32>,
        socket: Option<ws::SOCKET>,
        ok: bool,
        write_count: Option<u32>,
        except_count: Option<u32>,
        write_member: Option<bool>,
        except_member: Option<bool>,
        so_length: Option<i32>,
        so_value: Option<i32>,
        bytes: [u8; 139],
        length: usize,
    }
    impl NativeClientReturn {
        fn empty() -> Self {
            Self {
                returned: 0,
                error: None,
                socket: None,
                ok: false,
                write_count: None,
                except_count: None,
                write_member: None,
                except_member: None,
                so_length: None,
                so_value: None,
                bytes: [0; 139],
                length: 0,
            }
        }
    }
    impl Drop for NativeClientReturn {
        fn drop(&mut self) {
            self.bytes.fill(0);
        }
    }

    // Native execution owns only a reserved snapshot; it never holds an Owner or
    // driver state borrow. Capture errors immediately, before clocks or Drops.
    fn perform_client(a: &ClientAction) -> NativeClientReturn {
        let mut out = NativeClientReturn::empty();
        match &a.kind {
            ClientOp::Setup(step) => {
                let operation = match (*step, a.handle) {
                    (0, _) => Operation::Setup(1, None),
                    (1, Some(h)) => Operation::Accepted(0, h),
                    (2, Some(h)) => Operation::Accepted(1, h),
                    (3, Some(h)) => Operation::Accepted(2, h),
                    _ => return out,
                };
                let native = perform(operation);
                out.ok = native.ok && (*step != 2 || native.linger_off == Some(true));
                out.error = native.error;
                out.socket = native.socket;
                out.returned = if native.ok { 0 } else { ws::SOCKET_ERROR };
                drop(native);
            }
            ClientOp::Connect(address) => {
                if let Some(h) = a.handle {
                    let r = unsafe {
                        ws::connect(
                            h,
                            (address as *const ws::SOCKADDR_IN).cast(),
                            std::mem::size_of::<ws::SOCKADDR_IN>() as i32,
                        )
                    };
                    let error = socket_error(r);
                    out.returned = r;
                    out.error = error;
                    out.ok = r == 0;
                }
            }
            ClientOp::Select => {
                if let Some(h) = a.handle {
                    let mut write = ws::FD_SET {
                        fd_count: 1,
                        fd_array: [0; 64],
                    };
                    write.fd_array[0] = h;
                    let mut except = ws::FD_SET {
                        fd_count: 1,
                        fd_array: [0; 64],
                    };
                    except.fd_array[0] = h;
                    let zero = ws::TIMEVAL {
                        tv_sec: 0,
                        tv_usec: 0,
                    };
                    let r = unsafe {
                        ws::select(0, std::ptr::null_mut(), &mut write, &mut except, &zero)
                    };
                    let error = socket_error(r);
                    out.returned = r;
                    out.error = error;
                    out.ok = r >= 0;
                    out.write_count = Some(write.fd_count);
                    out.except_count = Some(except.fd_count);
                    out.write_member = match write.fd_count {
                        0 => Some(false),
                        1 => Some(write.fd_array[0] == h),
                        _ => None,
                    };
                    out.except_member = match except.fd_count {
                        0 => Some(false),
                        1 => Some(except.fd_array[0] == h),
                        _ => None,
                    };
                }
            }
            ClientOp::SoError => {
                if let Some(h) = a.handle {
                    let mut value = i32::MIN;
                    let mut length = std::mem::size_of::<i32>() as i32;
                    let r = unsafe {
                        ws::getsockopt(
                            h,
                            ws::SOL_SOCKET,
                            ws::SO_ERROR,
                            (&mut value as *mut i32).cast(),
                            &mut length,
                        )
                    };
                    let error = socket_error(r);
                    out.returned = r;
                    out.error = error;
                    out.ok = r == 0;
                    out.so_length = Some(length);
                    out.so_value = Some(value);
                }
            }
            ClientOp::Send { bytes, len } => {
                if let Some(h) = a.handle {
                    let r = unsafe { ws::send(h, bytes.as_ptr(), *len as i32, 0) };
                    let error = socket_error(r);
                    out.returned = r;
                    out.error = error;
                    out.ok = r >= 0;
                    if r >= 0 {
                        out.length = r as usize;
                    }
                }
            }
            ClientOp::Receive(length) => {
                if let Some(h) = a.handle {
                    let r = unsafe { ws::recv(h, out.bytes.as_mut_ptr(), *length as i32, 0) };
                    let error = socket_error(r);
                    out.returned = r;
                    out.error = error;
                    out.ok = r >= 0;
                    if r >= 0 {
                        out.length = r as usize;
                    }
                }
            }
        }
        out
    }

    struct Driver {
        started: Instant,
        identity: Rc<()>,
        timing: Timing,
        progress: bool,
        server: ServerStorage,
        clients: [Client; 2],
        harness: ClientHarness,
        sequence: u16,
        client_last: Option<Instant>,
        cursor: usize,
        first_accept_returned: bool,
        legacy_listener_closed_before_b: bool,
        target: Option<Instant>,
        close_origin: Option<Instant>,
        immediate: Option<bool>,
        first_failure: Option<&'static str>,
        unknown: bool,
        request: [u8; 128],
        request_len: usize,
        response: [u8; 139],
    }
    impl Driver {
        fn new(started: Instant) -> Self {
            let mut receipts = [ClientReceipt::default(), ClientReceipt::default()];
            for (n, r) in receipts.iter_mut().enumerate() {
                r.ordinal = (n + 1) as u8;
                r.connect.disposition = "Unobserved";
                r.select.ready_class = "Unobserved";
                r.so_error.completion = "Unobserved";
            }
            Self {
                started,
                identity: Rc::new(()),
                timing: Timing::new(started),
                progress: true,
                server: ServerStorage::new(),
                clients: [Client::new(1), Client::new(2)],
                harness: ClientHarness {
                    version: 1,
                    tier: "transportOnly",
                    slots: receipts,
                    ..ClientHarness::default()
                },
                sequence: 0,
                client_last: None,
                cursor: 0,
                first_accept_returned: false,
                legacy_listener_closed_before_b: false,
                target: None,
                close_origin: None,
                immediate: None,
                first_failure: None,
                unknown: false,
                request: [0; 128],
                request_len: 0,
                response: [0; 139],
            }
        }
        fn live(&self, now: Instant) -> bool {
            self.progress
                && self.timing.matches_start(self.started)
                && self.timing.before(now)
                && now
                    .checked_duration_since(self.started)
                    .is_some_and(|v| v < Duration::from_secs(75))
        }
        fn close_live(&self, now: Instant) -> bool {
            self.timing.first_close.is_some_and(|at| {
                now >= at
                    && now
                        .checked_duration_since(at)
                        .is_some_and(|v| v < Duration::from_secs(10))
            }) && now
                .checked_duration_since(self.started)
                .is_some_and(|v| v < Duration::from_secs(75))
        }
        fn seq(&mut self) -> Option<u16> {
            let next = self.sequence.checked_add(1)?;
            self.sequence = next;
            Some(next)
        }
        fn fail(&mut self, reason: &'static str, now: Instant) {
            if self.first_failure.is_none() {
                self.first_failure = Some(reason);
            }
            self.progress = false;
            if self.timing.first_close.is_none() {
                self.close_origin = Some(now);
            }
            self.timing.close(now);
            self.server.phase = "Revoked";
            drop(self.server.parser.take());
        }
        fn target(&mut self, reason: Option<&'static str>, now: Instant) {
            if self.first_failure.is_some() || !self.live(now) {
                self.fail("deadline", now);
                return;
            }
            self.target = Some(now);
            if let Some(reason) = reason {
                self.first_failure = Some(reason);
            }
            self.progress = false;
            let close = Instant::now();
            let first = self.timing.first_close.is_none();
            if first {
                self.close_origin = Some(close);
            }
            self.timing.close(close);
            self.immediate =
                Some(first && !self.progress && self.timing.first_close == Some(close));
            self.server.phase = "Revoked";
            drop(self.server.parser.take());
        }
        fn inventory(&mut self) {
            core_inventory(&mut self.server.parts());
            let server = self.server.facts.sockets_remaining;
            let clients = self
                .clients
                .iter()
                .filter(|c| c.lease.as_ref().is_some_and(SocketLease::owned))
                .count() as u16;
            let in_flight = self.clients.iter().filter(|c| c.in_flight).count() as u16;
            let obligations = self
                .harness
                .slots
                .iter()
                .filter(|r| {
                    r.select.ready_seq.is_some()
                        && !r.so_error.query_consumed
                        && !r.so_error.obligation_cancelled
                })
                .count() as u16;
            let o = &mut self.harness.ownership;
            o.server_peak = o.server_peak.max(server);
            o.client_peak = o.client_peak.max(clients);
            o.combined_peak = o.combined_peak.max(server + clients);
            o.remaining_server_sockets = server;
            o.remaining_client_sockets = clients;
            o.in_flight_client_actions = in_flight;
            o.completion_obligations = obligations;
            self.harness.cleanup.reference_remaining =
                u16::from(self.harness.startup.reference_owned);
        }
        fn server_close(&mut self, listener: bool) {
            let now = Instant::now();
            let allowed = if self.progress {
                self.live(now)
            } else {
                self.close_live(now)
            };
            if !allowed && self.server.facts.sockets_remaining != 0 {
                self.unknown = true;
                self.fail("deadline", now);
                return;
            }
            let handle = core_close_reserve(&mut self.server.parts(), listener);
            if let Some(h) = handle {
                let before = Instant::now();
                let r = unsafe { ws::closesocket(h) };
                let error = socket_error(r);
                let now = Instant::now();
                // Ordinary listener retirement is still within original45;
                // closing dispositions use the already adopted original10.
                let timely = if self.progress {
                    self.live(before) && self.live(now)
                } else {
                    self.close_live(before) && self.close_live(now)
                };
                let qualified = r == 0 && error.is_none() && timely;
                core_close_returned(&mut self.server.parts(), listener, qualified);
                self.server.in_action = false;
                if !qualified {
                    self.unknown = true;
                    self.fail("closeFailure", now);
                }
                self.inventory();
            }
            self.server_balance();
        }
        fn server_balance(&mut self) {
            let now = Instant::now();
            let allowed = if self.progress {
                self.live(now)
            } else {
                self.close_live(now)
            };
            if !allowed && self.server.startup {
                self.unknown = true;
                self.fail("deadline", now);
                return;
            }
            if core_cleanup_reserve(&mut self.server.parts()) {
                self.harness
                    .ownership
                    .harness_reference_held_at_server_cleanup =
                    Some(self.harness.startup.reference_owned);
                let before = Instant::now();
                let r = unsafe { ws::WSACleanup() };
                let error = socket_error(r);
                let now = Instant::now();
                let timely = if self.progress {
                    self.live(before) && self.live(now)
                } else {
                    self.close_live(before) && self.close_live(now)
                };
                let ok = r == 0 && error.is_none() && timely;
                core_cleanup_returned(&mut self.server.parts(), ok);
                self.server.in_action = false;
                if !ok {
                    self.unknown = true;
                    self.fail("cleanupFailure", now);
                }
                self.inventory();
            }
        }
        fn server_op(&mut self, now: Instant) -> bool {
            if !self.live(now) || self.server.in_action {
                return false;
            }
            if self.server.setup == 9
                && self.server.facts.accepts == 0
                && self.clients[0].connection != Connection::Connected
            {
                return false;
            }
            // Explicit cfg(test) transport-only admission. No fabricated native
            // top/resource/core/Basic/Session facts exist in this tier.
            let op = match core_next(true, true, &self.server.parts()) {
                CoreNext::Operation(op) => op,
                _ => return false,
            };
            match core_reserve(&mut self.server.parts(), op, now) {
                Ok(false) => return false,
                Err(()) => {
                    self.fail("budget", now);
                    return false;
                }
                Ok(true) => {}
            }
            let outcome = perform(op);
            core_inventory_return(&mut self.server.parts(), op, &outcome);
            self.inventory();
            let current = self.live(Instant::now()) && core_matches(&self.server.parts(), op);
            let timely = self.timing.before(Instant::now());
            let reason = core_settle(&mut self.server.parts(), op, &outcome, current, timely);
            let retire = core_retire_listener(op, &outcome);
            let accepted = matches!(op, Operation::Accept(_)) && outcome.socket.is_some();
            drop(outcome);
            let post = Instant::now();
            if !self.live(post) {
                self.fail("deadline", post);
            }
            if let Some(reason) = reason {
                self.fail(reason, post);
            }
            if retire {
                self.server_close(true);
            }
            // Marker is published only after the real accept, Outcome Drop and
            // the same actual immediate-listener-close disposition have returned.
            if accepted && self.progress {
                self.first_accept_returned = true;
            }
            if self.server.facts.response_complete == Some(true) {
                self.server_close(false);
            }
            if self.server.setup == 9 && self.server.phase == "Preparing" {
                self.server.phase = "Armed";
            }
            self.inventory();
            true
        }
        fn client_kind(&self, n: usize) -> Option<ClientOp> {
            let c = &self.clients[n];
            let r = &self.harness.slots[n];
            if !self.harness.startup.reference_owned
                || self.harness.startup.version_matched != Some(true)
                || self.harness.cleanup.calls != 0
                || c.in_flight
            {
                return None;
            }
            if n == 1
                && (!self.first_accept_returned
                    || self.clients[0].connection != Connection::Connected)
            {
                return None;
            }
            if c.setup < 4 {
                return Some(ClientOp::Setup(c.setup));
            }
            if c.connection == Connection::Unobserved && self.server.setup == 9 {
                let port = self.server.port?;
                let mut address = loopback();
                address.sin_port = port.to_be();
                return Some(ClientOp::Connect(address));
            }
            if c.connection == Connection::Pending {
                if r.select.ready_seq.is_some()
                    && !r.so_error.query_consumed
                    && !r.so_error.obligation_cancelled
                {
                    return Some(ClientOp::SoError);
                }
                if r.select.ready_seq.is_none()
                    && matches!(r.select.ready_class, "Unobserved" | "NoReady")
                {
                    return Some(ClientOp::Select);
                }
            }
            if n == 1 && c.connection == Connection::Connected {
                if usize::from(r.data.sent_bytes) < self.request_len {
                    let mut bytes = [0; 128];
                    let start = usize::from(r.data.sent_bytes);
                    let len = self.request_len - start;
                    bytes[..len].copy_from_slice(&self.request[start..self.request_len]);
                    return Some(ClientOp::Send { bytes, len });
                }
                if r.data.received_bytes < 139 {
                    return Some(ClientOp::Receive(139 - usize::from(r.data.received_bytes)));
                }
            }
            None
        }
        fn reserve_client(&mut self, n: usize, now: Instant) -> Option<ClientAction> {
            if !self.live(now) {
                return None;
            }
            let kind = self.client_kind(n)?;
            if self.client_last.is_some_and(|last| {
                now.checked_duration_since(last)
                    .is_none_or(|elapsed| elapsed < Duration::from_millis(25))
            }) {
                if self.harness.budgets.admission_denials == 8192 {
                    self.harness.budgets.overflow = true;
                    self.fail("overflow", now);
                } else {
                    self.harness.budgets.admission_denials += 1;
                }
                return None;
            }
            let setup = matches!(kind, ClientOp::Setup(_));
            let count = if setup {
                self.harness.budgets.setup_calls
            } else {
                self.harness.budgets.io_calls
            };
            let cap = if setup { 64 } else { 2048 };
            if count > cap - 2 {
                self.fail("budget", now);
                return None;
            }
            let reserved_handle = self.clients[n]
                .lease
                .as_ref()
                .filter(|lease| lease.owned() && !lease.close_attempted)
                .map(|lease| lease.handle);
            let record_current = self.clients[n].id == (n + 1) as u8
                && if matches!(kind, ClientOp::Setup(0)) {
                    self.clients[n].lease.is_none()
                } else {
                    reserved_handle.is_some()
                };
            if !record_current {
                self.fail("staleIdentity", now);
                return None;
            }
            let entered = match self.seq() {
                Some(v) => v,
                None => {
                    self.harness.budgets.overflow = true;
                    self.fail("overflow", now);
                    return None;
                }
            };
            let endpoint = match &kind {
                ClientOp::Connect(address) => {
                    self.server.port.is_some() && address_matches(address, self.server.port)
                }
                _ => self.server.port.is_some(),
            };
            if matches!(kind, ClientOp::Connect(_)) && n == 1 {
                self.legacy_listener_closed_before_b = self.server.facts.accepts == 1
                    && self.server.facts.listener_close_attempts == 1
                    && self.server.facts.listener_close_successes == 1
                    && self.server.observation.listener_retirement == Some("legacyFirstAccept")
                    && self.server.observation.listener_retired == Some(true)
                    && self.first_accept_returned;
            }
            let c = &mut self.clients[n];
            let r = &mut self.harness.slots[n];
            let handle = c
                .lease
                .as_ref()
                .filter(|lease| lease.owned() && !lease.close_attempted)
                .map(|lease| lease.handle);
            let current = c.id == (n + 1) as u8
                && if matches!(kind, ClientOp::Setup(0)) {
                    c.lease.is_none()
                } else {
                    handle.is_some()
                };
            let nonblocking = r.nonblocking_ack == Some(true);
            match &kind {
                ClientOp::Setup(0) => r.socket_calls += 1,
                ClientOp::Setup(1) => r.linger_set_calls += 1,
                ClientOp::Setup(2) => r.linger_read_calls += 1,
                ClientOp::Setup(3) => r.nonblocking_calls += 1,
                ClientOp::Connect(_) => {
                    r.connect.calls += 1;
                    r.connect.entered_seq = Some(entered);
                }
                ClientOp::Select => r.select.calls += 1,
                ClientOp::SoError => {
                    r.so_error.calls += 1;
                    r.so_error.query_consumed = true;
                    r.so_error.from_ready_seq = r.select.ready_seq;
                    r.so_error.entered_seq = Some(entered);
                }
                ClientOp::Send { .. } => r.data.send_calls += 1,
                ClientOp::Receive(_) => r.data.recv_calls += 1,
                _ => {}
            }
            if setup {
                self.harness.budgets.setup_calls += 1;
            } else {
                self.harness.budgets.io_calls += 1;
            }
            self.harness.budgets.primary_operations += 1;
            self.harness.budgets.spacing_qualified = Some(
                self.harness.budgets.spacing_qualified.unwrap_or(true)
                    && self.client_last.is_none_or(|last| {
                        now.checked_duration_since(last)
                            .is_some_and(|v| v >= Duration::from_millis(25))
                    }),
            );
            self.harness.budgets.original_clock_matched =
                Some(self.timing.matches_start(self.started));
            self.client_last = Some(now);
            c.in_flight = true;
            c.action_seq = Some(entered);
            Some(ClientAction {
                ordinal: n,
                record: c.id,
                handle,
                kind,
                attempt: Rc::downgrade(&self.identity),
                entered,
                before: now,
                before_current: current,
                endpoint,
                endpoint_port: self.server.port,
                nonblocking,
                from_ready: r.select.ready_seq,
            })
        }
        fn apply_client(&mut self, a: ClientAction, native: NativeClientReturn) {
            // Native primary + immediate error-read have already returned. Keep
            // every side effect/count even if later identity/time checks fail.
            let returned_seq = self.seq();
            if let Some(socket) = native.socket {
                self.clients[a.ordinal].lease = Some(SocketLease::new(socket, u64::from(a.record)));
                self.harness.slots[a.ordinal].close.owned_at_final = true;
                self.inventory();
            }
            // A separate physical check at return, before the response scratch
            // and reservation-token Drops checked again below.
            let current_after_return = self.live(Instant::now())
                && self.clients[a.ordinal].id == a.record
                && self.clients[a.ordinal].action_seq == Some(a.entered)
                && self.clients[a.ordinal].in_flight
                && self.clients[a.ordinal].lease.as_ref().is_some_and(|lease| {
                    lease._id == u64::from(a.record)
                        && lease.owned()
                        && !lease.close_attempted
                        && (matches!(a.kind, ClientOp::Setup(0)) || Some(lease.handle) == a.handle)
                })
                && a.attempt
                    .upgrade()
                    .is_some_and(|identity| Rc::ptr_eq(&identity, &self.identity));
            let raw_return = native.returned;
            let error = native.error;
            let socket = native.socket;
            let ok = native.ok;
            let wc = native.write_count;
            let ec = native.except_count;
            let wm = native.write_member;
            let em = native.except_member;
            let so_length = native.so_length;
            let so_value = native.so_value;
            let len = native.length;
            let mut bytes = native.bytes;
            drop(native);
            let ClientAction {
                ordinal: n,
                record,
                handle,
                kind,
                attempt,
                entered,
                before,
                before_current,
                endpoint,
                endpoint_port,
                nonblocking,
                from_ready,
            } = a;
            let attempt_matched = attempt
                .upgrade()
                .is_some_and(|identity| Rc::ptr_eq(&identity, &self.identity));
            drop(attempt);
            let post_seq = self.seq();
            let now = Instant::now();
            let same_record = self.clients[n].id == record
                && self.clients[n].action_seq == Some(entered)
                && self.clients[n].in_flight
                && self.clients[n].lease.as_ref().is_some_and(|lease| {
                    lease._id == u64::from(record)
                        && lease.owned()
                        && !lease.close_attempted
                        && (matches!(kind, ClientOp::Setup(0)) || Some(lease.handle) == handle)
                });
            let after = self.live(now)
                && same_record
                && attempt_matched
                && returned_seq.is_some()
                && post_seq.is_some();
            let q = ClientQ {
                same_record: Some(same_record),
                same_attempt: Some(attempt_matched),
                endpoint_argument_matched: Some(endpoint && endpoint_port == self.server.port),
                nonblocking_ack: Some(nonblocking),
                current_before: Some(before_current),
                current_after: Some(current_after_return),
                time_before: Some(self.timing.before(before)),
                time_after: Some(self.timing.before(now)),
                post_drop_current: Some(after),
            };
            let qualified = q.positive();
            let setup = matches!(kind, ClientOp::Setup(_));
            if error.is_some() {
                if setup {
                    self.harness.budgets.setup_calls += 1;
                    self.harness.budgets.setup_error_reads += 1;
                    self.harness.slots[n].setup_error_reads += 1;
                } else {
                    self.harness.budgets.io_calls += 1;
                    self.harness.budgets.io_error_reads += 1;
                }
            }
            let mut failure = None;
            let mut refused_target = false;
            let mut response_target = false;
            {
                let c = &mut self.clients[n];
                let r = &mut self.harness.slots[n];
                match kind {
                    ClientOp::Setup(step) => {
                        match step {
                            0 => r.socket_created = Some(socket.is_some()),
                            1 => r.linger_set_ack = Some(ok),
                            2 => r.linger_read_ack = Some(ok),
                            3 => r.nonblocking_ack = Some(ok),
                            _ => {}
                        }
                        // Setup has no endpoint/nonblocking completion Q; it must
                        // establish those facts rather than assume them.
                        if ok
                            && after
                            && before_current
                            && self.timing.before(before)
                            && self.timing.before(now)
                        {
                            c.setup += 1;
                        } else {
                            failure = Some("setupFailure");
                        }
                    }
                    ClientOp::Connect(_) => {
                        r.connect.returned = Some(raw_return);
                        r.connect.last_error = error;
                        r.connect.returned_seq = returned_seq;
                        r.connect.at_ms = self.timing.ms(now);
                        r.connect.q = q;
                        let (disposition, connection) = if !qualified {
                            ("Unqualified", Connection::Failed)
                        } else if raw_return == 0 && error.is_none() {
                            ("ImmediateConnected", Connection::Connected)
                        } else if raw_return == ws::SOCKET_ERROR
                            && error == Some(ws::WSAEWOULDBLOCK)
                        {
                            ("Pending", Connection::Pending)
                        } else if raw_return == ws::SOCKET_ERROR
                            && error == Some(ws::WSAECONNREFUSED)
                        {
                            ("ImmediateRefused", Connection::Refused)
                        } else {
                            ("OtherError", Connection::Failed)
                        };
                        r.connect.disposition = disposition;
                        c.connection = connection;
                        if connection == Connection::Failed {
                            failure = Some("connectOtherError");
                        } else if connection == Connection::Refused {
                            if n == 1 {
                                refused_target = true;
                            } else {
                                failure = Some("setupFailure");
                            }
                        }
                    }
                    ClientOp::Select => {
                        r.select.last_return = Some(raw_return);
                        r.select.last_error = error;
                        r.select.write_count = wc;
                        r.select.except_count = ec;
                        r.select.write_current_member = wm;
                        r.select.except_current_member = em;
                        r.select.q = q;
                        if error.is_some() {
                            r.select.error_reads += 1;
                        }
                        let class = if !qualified {
                            "Unqualified"
                        } else if raw_return == ws::SOCKET_ERROR {
                            "Error"
                        } else if raw_return == 0 && wc == Some(0) && ec == Some(0) {
                            "NoReady"
                        } else if raw_return == 1
                            && wc == Some(1)
                            && ec == Some(0)
                            && wm == Some(true)
                            && em == Some(false)
                        {
                            "WriteOnly"
                        } else if raw_return == 1
                            && wc == Some(0)
                            && ec == Some(1)
                            && wm == Some(false)
                            && em == Some(true)
                        {
                            "ExceptOnly"
                        } else if wc == Some(1) && ec == Some(1) {
                            "BothReady"
                        } else {
                            "Invalid"
                        };
                        r.select.ready_class = class;
                        match class {
                            "NoReady" => r.select.no_ready += 1,
                            "WriteOnly" | "ExceptOnly" => {
                                r.select.ready_seq = post_seq;
                                r.select.ready_at_ms = self.timing.ms(now);
                            }
                            "Error" => {
                                failure = Some("selectFailure");
                                c.connection = Connection::Failed;
                            }
                            _ => {
                                failure = Some("ambiguousReadiness");
                                c.connection = Connection::Failed;
                            }
                        }
                    }
                    ClientOp::SoError => {
                        r.so_error.returned = Some(raw_return);
                        r.so_error.last_error = error;
                        r.so_error.length = so_length;
                        r.so_error.buffer_value = so_value;
                        r.so_error.returned_seq = returned_seq;
                        r.so_error.at_ms = self.timing.ms(now);
                        r.so_error.q = q;
                        let available = raw_return == 0
                            && error.is_none()
                            && so_length == Some(4)
                            && so_value.is_some_and(|v| v != i32::MIN);
                        r.so_error.value_available = Some(available);
                        let valid = qualified
                            && available
                            && from_ready.is_some()
                            && from_ready == r.select.ready_seq
                            && r.so_error.from_ready_seq == from_ready
                            && r.so_error.calls == 1
                            && r.so_error.query_consumed
                            && !r.so_error.obligation_cancelled
                            && r.connect.disposition == "Pending";
                        let connection = if valid
                            && r.select.ready_class == "WriteOnly"
                            && so_value == Some(0)
                        {
                            Connection::Connected
                        } else if valid
                            && r.select.ready_class == "ExceptOnly"
                            && so_value == Some(ws::WSAECONNREFUSED)
                        {
                            Connection::Refused
                        } else {
                            Connection::Failed
                        };
                        r.so_error.completion = match connection {
                            Connection::Connected => "Connected",
                            Connection::Refused => "Refused",
                            _ if valid
                                && r.select.ready_class == "ExceptOnly"
                                && so_value.is_some_and(|v| v != 0) =>
                            {
                                "OtherError"
                            }
                            _ => "Unqualified",
                        };
                        if matches!(connection, Connection::Connected | Connection::Refused)
                            || r.so_error.completion == "OtherError"
                        {
                            r.so_error.completion_count = 1;
                        }
                        c.connection = connection;
                        if connection == Connection::Failed {
                            failure = Some("soErrorFailure");
                        } else if connection == Connection::Refused {
                            if n == 1 {
                                refused_target = true;
                            } else {
                                failure = Some("setupFailure");
                            }
                        }
                    }
                    ClientOp::Send {
                        bytes: sent_argument,
                        len: asked,
                    } => {
                        if error.is_some() {
                            r.data.send_error_reads += 1;
                        }
                        let start = usize::from(r.data.sent_bytes);
                        let matched = n == 1
                            && self.request_len > 0
                            && asked <= 128
                            && start.checked_add(asked) == Some(self.request_len)
                            && sent_argument[..asked] == self.request[start..self.request_len];
                        r.data.request_matched =
                            Some(r.data.request_matched.unwrap_or(true) && matched);
                        if ok {
                            r.data.sent_bytes = r.data.sent_bytes.saturating_add(len as u16);
                        }
                        r.data.original_timely =
                            Some(r.data.original_timely.unwrap_or(true) && qualified);
                        let nonprogress = matches!(error, Some(ws::WSAEWOULDBLOCK | ws::WSAEINTR));
                        if !qualified
                            || (!ok && !nonprogress)
                            || (ok
                                && (len == 0
                                    || len > asked
                                    || usize::from(r.data.sent_bytes) > self.request_len))
                        {
                            failure = Some("transportFailure");
                        }
                    }
                    ClientOp::Receive(asked) => {
                        if error.is_some() {
                            r.data.recv_error_reads += 1;
                        }
                        if ok && len <= asked && usize::from(r.data.received_bytes) + len <= 139 {
                            let start = usize::from(r.data.received_bytes);
                            self.response[start..start + len].copy_from_slice(&bytes[..len]);
                            r.data.received_bytes += len as u16;
                        }
                        r.data.original_timely =
                            Some(r.data.original_timely.unwrap_or(true) && qualified);
                        let nonprogress = matches!(error, Some(ws::WSAEWOULDBLOCK | ws::WSAEINTR));
                        if !qualified || (!ok && !nonprogress) || (ok && (len == 0 || len > asked))
                        {
                            failure = Some("transportFailure");
                        } else if r.data.received_bytes == 139 {
                            let matched = self.response == *RESPONSE;
                            r.data.response_matched = Some(matched);
                            if matched {
                                response_target = true;
                            } else {
                                failure = Some("byteMismatch");
                            }
                        }
                    }
                }
                c.in_flight = false;
                c.action_seq = None;
            }
            bytes.fill(0);
            self.inventory();
            if let Some(reason) = failure {
                self.fail(reason, now);
            } else if refused_target {
                if self.legacy_listener_closed_before_b
                    && self.server.accepted_setup == 4
                    && self.server.facts.request_bytes == 0
                    && self.server.facts.parsed_requests == 0
                    && self.server.facts.sent_bytes == 0
                {
                    self.target(Some("legacyAdmissionRefused"), now);
                } else {
                    self.fail("transportFailure", now);
                }
            } else if response_target {
                self.target(None, now);
            }
        }
        fn startup_clients(&mut self) {
            let before = Instant::now();
            if !self.live(before) {
                self.fail("deadline", before);
                return;
            }
            self.harness.startup.calls = 1;
            let mut data = ws::WSADATA::default();
            let r = unsafe { ws::WSAStartup(0x0202, &mut data) };
            // WSAStartup returns its error directly: no WSAGetLastError here.
            self.harness.startup.returned = Some(r);
            self.harness.startup.succeeded = Some(r == 0);
            if r == 0 {
                self.harness.startup.reference_owned = true;
                self.harness.startup.version = Some(data.wVersion);
                self.harness.startup.version_matched = Some(data.wVersion == 0x0202);
            }
            let now = Instant::now();
            self.harness.ownership.no_ambient_startup_reliance =
                Some(r == 0 && self.harness.startup.reference_owned);
            self.inventory();
            if r != 0 || data.wVersion != 0x0202 {
                self.fail("setupFailure", now);
            } else if !self.live(now) {
                self.fail("deadline", now);
            }
        }
        fn close_clients(&mut self) {
            // Explicitly cancel only ready obligations that have never invoked
            // SO_ERROR, after revocation. No completion is manufactured.
            for r in &mut self.harness.slots {
                if r.select.ready_seq.is_some() && !r.so_error.query_consumed {
                    r.so_error.obligation_cancelled = true;
                }
            }
            for n in [0_usize, 1] {
                if self.clients[n].in_flight {
                    self.unknown = true;
                    continue;
                }
                if self.clients[n]
                    .lease
                    .as_ref()
                    .is_some_and(SocketLease::owned)
                    && !self.close_live(Instant::now())
                {
                    self.unknown = true;
                    self.fail("deadline", Instant::now());
                    continue;
                }
                let h = self.clients[n]
                    .lease
                    .as_mut()
                    .and_then(SocketLease::reserve_close);
                if let Some(handle) = h {
                    self.harness.slots[n].close.calls = 1;
                    self.harness.slots[n].close.in_flight = true;
                    let before = Instant::now();
                    let r = unsafe { ws::closesocket(handle) };
                    let error = socket_error(r);
                    let now = Instant::now();
                    let current = self.clients[n].id == (n + 1) as u8
                        && self.clients[n].lease.as_ref().is_some_and(|lease| {
                            lease.handle == handle
                                && lease._id == (n + 1) as u64
                                && lease.close_attempted
                                && lease.owned()
                        });
                    let qualified = r == 0
                        && error.is_none()
                        && current
                        && self.close_live(before)
                        && self.close_live(now);
                    if error.is_some() {
                        self.harness.cleanup.close_error_reads += 1;
                    }
                    if let Some(lease) = self.clients[n].lease.as_mut() {
                        lease.close_returned(qualified);
                    }
                    let close = &mut self.harness.slots[n].close;
                    close.returned = Some(r);
                    close.last_error = error;
                    close.qualified = Some(qualified);
                    close.in_flight = false;
                    close.owned_at_final = self.clients[n]
                        .lease
                        .as_ref()
                        .is_some_and(SocketLease::owned);
                    if !qualified {
                        self.unknown = true;
                        self.fail("closeFailure", now);
                    }
                    self.inventory();
                }
            }
            self.inventory();
            if self.harness.startup.reference_owned {
                let eligible = self.harness.ownership.remaining_client_sockets == 0
                    && self.harness.ownership.in_flight_client_actions == 0
                    && self.harness.ownership.completion_obligations == 0
                    && self.harness.cleanup.calls == 0
                    && self.close_live(Instant::now());
                self.harness.cleanup.eligible_before = Some(eligible);
                if eligible {
                    self.harness.cleanup.calls = 1;
                    let before = Instant::now();
                    let r = unsafe { ws::WSACleanup() };
                    let error = socket_error(r);
                    let now = Instant::now();
                    let qualified = r == 0
                        && error.is_none()
                        && self.close_live(before)
                        && self.close_live(now);
                    self.harness.cleanup.returned = Some(r);
                    self.harness.cleanup.last_error = error;
                    self.harness.cleanup.qualified = Some(qualified);
                    if error.is_some() {
                        self.harness.cleanup.cleanup_error_reads += 1;
                    }
                    if qualified {
                        self.harness.startup.reference_owned = false;
                    } else {
                        self.unknown = true;
                        self.fail("cleanupFailure", now);
                    }
                    // This is the final client call site; no ordinary client
                    // reservation remains eligible after this attempt.
                    self.harness.cleanup.no_client_use_after_attempt =
                        Some(!self.progress && self.clients.iter().all(|c| !c.in_flight));
                } else {
                    self.unknown = true;
                }
            }
            self.inventory();
        }
        fn cleanup(&mut self) {
            let now = Instant::now();
            if self.progress {
                self.fail("notCompleted", now);
            }
            if self.timing.first_close.is_none() {
                self.timing.close(now);
            }
            self.server_close(true);
            self.server_close(false);
            self.server_balance();
            self.close_clients();
            self.inventory();
            self.server.facts.final_inventory_observed = Some(true);
            if self.server.facts.sockets_remaining == 0
                && self.server.facts.startup_remaining == 0
                && !self.server.in_action
            {
                self.server.phase = "Closed";
                core_inventory(&mut self.server.parts());
            }
        }
        fn drive(&mut self) {
            self.startup_clients();
            for _ in 0..TURN_LIMIT {
                let now = Instant::now();
                if !self.progress {
                    break;
                }
                if !self.live(now) {
                    self.fail("deadline", now);
                    break;
                }
                if self.server.setup == 9 && self.request_len == 0 {
                    if let Some(port) = self.server.port {
                        let text = format!(
                            "GET /code0-http-auth HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n\r\n"
                        );
                        if text.len() > self.request.len() {
                            self.fail("setupFailure", now);
                            break;
                        }
                        self.request_len = text.len();
                        self.request[..text.len()].copy_from_slice(text.as_bytes());
                    } else {
                        self.fail("setupFailure", now);
                        break;
                    }
                }
                let mut selected = None;
                // At most three memory-only inspections, not native retries.
                for offset in 0..3 {
                    let source = (self.cursor + offset) % 3;
                    if source == 0 {
                        if (self.server.setup < 9
                            || self.clients[0].connection == Connection::Connected)
                            && matches!(
                                core_next(true, true, &self.server.parts()),
                                CoreNext::Operation(_)
                            )
                        {
                            selected = Some(source);
                            break;
                        }
                    } else if self.client_kind(source - 1).is_some() {
                        selected = Some(source);
                        break;
                    }
                }
                let mut performed = false;
                if let Some(source) = selected {
                    if source == 0 {
                        performed = self.server_op(now);
                    } else if let Some(action) = self.reserve_client(source - 1, now) {
                        let native = perform_client(&action);
                        self.apply_client(action, native);
                        performed = true;
                    }
                    if performed {
                        self.cursor = (source + 1) % 3;
                    }
                }
                if !self.progress {
                    break;
                }
                // Passive wait on this libtest thread only, bounded request; no
                // UI pump, additional producer, thread, timer or busy yield.
                std::thread::sleep(SLEEP_MAX);
                let woke = Instant::now();
                if !self.live(woke) {
                    self.fail("deadline", woke);
                    break;
                }
            }
            if self.progress {
                self.fail("budget", Instant::now());
            }
            self.cleanup();
        }
        fn finish(mut self, run_id: String) -> OutcomeRecord {
            self.drive();
            let final_at = Instant::now();
            let close = self.timing.first_close;
            let target = self.target;
            let elapsed = close
                .and_then(|at| final_at.checked_duration_since(at))
                .and_then(|v| u64::try_from(v.as_millis()).ok());
            let within = target.zip(close).is_some_and(|(target, close)| {
                target <= close
                    && close <= final_at
                    && self.timing.before(target)
                    && final_at
                        .checked_duration_since(close)
                        .is_some_and(|v| v < Duration::from_secs(10))
                    && final_at
                        .checked_duration_since(self.started)
                        .is_some_and(|v| v < Duration::from_secs(75))
            });
            let timing = ControlledTiming {
                target_at_ms: target.and_then(|at| self.timing.ms(at)),
                first_close_at_ms: close.and_then(|at| self.timing.ms(at)),
                final_at_ms: self.timing.ms(final_at),
                cleanup_elapsed_ms: elapsed,
                original_clock_matched: Some(self.timing.matches_start(self.started)),
                close_immediately_after_target: self.immediate,
                first_close_adopted: Some(close.is_some() && close == self.close_origin),
                within_close_deadline: Some(within),
            };
            let connection = self.clients[1].connection;
            let result = if self.first_failure == Some("legacyAdmissionRefused") {
                "legacyAdmissionRefused"
            } else if self.first_failure.is_none()
                && self.harness.slots[1].data.response_matched == Some(true)
            {
                "transportGreen"
            } else {
                "failed"
            };
            let raw = ControlledRaw {
                schema: 1,
                run_id,
                case_id: "idle-first-request-second",
                variant: "legacyOneAccept",
                result,
                failure: self.first_failure.unwrap_or("none"),
                complete: false,
                unknown: self.unknown,
                overflow: self.harness.budgets.overflow,
                server: self.server.facts.clone(),
                transport: self.server.observation.clone(),
                client_harness: self.harness.clone(),
                timing,
            };
            let mut record = OutcomeRecord {
                raw,
                connection,
                legacy_order: self.legacy_listener_closed_before_b,
            };
            record.raw.complete = record.nominated();
            self.request.fill(0);
            self.response.fill(0);
            record
        }
    }
    impl ClientQ {
        fn positive(&self) -> bool {
            [
                self.same_record,
                self.same_attempt,
                self.endpoint_argument_matched,
                self.nonblocking_ack,
                self.current_before,
                self.current_after,
                self.time_before,
                self.time_after,
                self.post_drop_current,
            ]
            .into_iter()
            .all(|v| v == Some(true))
        }
    }
    impl ClientReceipt {
        fn configured(&self) -> bool {
            self.socket_calls == 1
                && self.socket_created == Some(true)
                && self.linger_set_calls == 1
                && self.linger_set_ack == Some(true)
                && self.linger_read_calls == 1
                && self.linger_read_ack == Some(true)
                && self.nonblocking_calls == 1
                && self.nonblocking_ack == Some(true)
                && self.setup_error_reads == 0
        }
        fn completed(&self, want: Connection) -> bool {
            if !self.configured()
                || self.connect.calls != 1
                || !self.connect.q.positive()
                || self.connect.at_ms.is_none_or(|v| v >= 45000)
                || !self
                    .connect
                    .entered_seq
                    .zip(self.connect.returned_seq)
                    .is_some_and(|(a, b)| a < b)
            {
                return false;
            }
            if self.connect.disposition == "ImmediateConnected"
                || self.connect.disposition == "ImmediateRefused"
            {
                let actual =
                    if self.connect.returned == Some(0) && self.connect.last_error.is_none() {
                        Connection::Connected
                    } else if self.connect.returned == Some(ws::SOCKET_ERROR)
                        && self.connect.last_error == Some(ws::WSAECONNREFUSED)
                    {
                        Connection::Refused
                    } else {
                        Connection::Failed
                    };
                return actual == want
                    && ((want == Connection::Connected
                        && self.connect.disposition == "ImmediateConnected")
                        || (want == Connection::Refused
                            && self.connect.disposition == "ImmediateRefused"))
                    && self.select.calls == 0
                    && self.so_error.calls == 0
                    && !self.so_error.query_consumed
                    && !self.so_error.obligation_cancelled
                    && self.so_error.completion_count == 0
                    && self.select
                        == ClientSelect {
                            ready_class: "Unobserved",
                            ..ClientSelect::default()
                        }
                    && self.so_error
                        == ClientSoError {
                            completion: "Unobserved",
                            ..ClientSoError::default()
                        };
            }
            if self.connect.disposition != "Pending"
                || self.connect.returned != Some(ws::SOCKET_ERROR)
                || self.connect.last_error != Some(ws::WSAEWOULDBLOCK)
                || !self.select.q.positive()
                || !self.so_error.q.positive()
            {
                return false;
            }
            let ready = match want {
                Connection::Connected => {
                    self.select.ready_class == "WriteOnly"
                        && self.select.write_count == Some(1)
                        && self.select.except_count == Some(0)
                        && self.select.write_current_member == Some(true)
                        && self.select.except_current_member == Some(false)
                }
                Connection::Refused => {
                    self.select.ready_class == "ExceptOnly"
                        && self.select.write_count == Some(0)
                        && self.select.except_count == Some(1)
                        && self.select.write_current_member == Some(false)
                        && self.select.except_current_member == Some(true)
                }
                _ => false,
            };
            let error = if want == Connection::Connected {
                0
            } else {
                ws::WSAECONNREFUSED
            };
            let spelling = if want == Connection::Connected {
                "Connected"
            } else {
                "Refused"
            };
            ready
                && self.select.last_return == Some(1)
                && self.select.last_error.is_none()
                && self.select.error_reads == 0
                && self.select.calls == self.select.no_ready + 1
                && self.so_error.calls == 1
                && self.so_error.query_consumed
                && !self.so_error.obligation_cancelled
                && self.so_error.returned == Some(0)
                && self.so_error.last_error.is_none()
                && self.so_error.length == Some(4)
                && self.so_error.buffer_value == Some(error)
                && self.so_error.value_available == Some(true)
                && self.so_error.completion_count == 1
                && self.so_error.completion == spelling
                && self.so_error.from_ready_seq == self.select.ready_seq
                && self
                    .select
                    .ready_seq
                    .zip(self.so_error.entered_seq)
                    .is_some_and(|(a, b)| a < b)
                && self
                    .so_error
                    .entered_seq
                    .zip(self.so_error.returned_seq)
                    .is_some_and(|(a, b)| a < b)
                && self
                    .select
                    .ready_at_ms
                    .zip(self.so_error.at_ms)
                    .is_some_and(|(a, b)| a <= b && b < 45000)
        }
    }
    pub(in super::super) struct OutcomeRecord {
        pub(in super::super) raw: ControlledRaw,
        pub(in super::super) connection: Connection,
        legacy_order: bool,
    }
    impl OutcomeRecord {
        fn nominated(&self) -> bool {
            let r = &self.raw;
            let s = &r.server;
            let t = &r.transport;
            let h = &r.client_harness;
            let a = &h.slots[0];
            let b = &h.slots[1];
            let setup_primary: u16 = h
                .slots
                .iter()
                .map(|c| {
                    c.socket_calls + c.linger_set_calls + c.linger_read_calls + c.nonblocking_calls
                })
                .sum();
            let io_primary: u16 = h
                .slots
                .iter()
                .map(|c| {
                    c.connect.calls
                        + c.select.calls
                        + c.so_error.calls
                        + c.data.send_calls
                        + c.data.recv_calls
                })
                .sum();
            let io_errors: u16 = h
                .slots
                .iter()
                .map(|c| {
                    u16::from(
                        c.connect.returned == Some(ws::SOCKET_ERROR)
                            && c.connect.last_error.is_some(),
                    ) + c.select.error_reads
                        + u16::from(
                            c.so_error.returned == Some(ws::SOCKET_ERROR)
                                && c.so_error.last_error.is_some(),
                        )
                        + c.data.send_error_reads
                        + c.data.recv_error_reads
                })
                .sum();
            let budget = h.budgets.setup_calls == setup_primary + h.budgets.setup_error_reads
                && h.budgets.setup_error_reads
                    == h.slots.iter().map(|c| c.setup_error_reads).sum::<u16>()
                && h.budgets.io_calls == io_primary + io_errors
                && h.budgets.io_error_reads == io_errors
                && h.budgets.primary_operations == setup_primary + io_primary
                && h.budgets.setup_calls <= 64
                && h.budgets.io_calls <= 2048
                && h.budgets.spacing_qualified == Some(true)
                && h.budgets.original_clock_matched == Some(true)
                && !h.budgets.overflow;
            let drained = !r.unknown
                && !r.overflow
                && s.sockets_remaining == 0
                && s.startup_remaining == 0
                && s.server_threads == 0
                && s.server_joins == 0
                && s.final_inventory_observed == Some(true)
                && s.startup_attempts == 1
                && s.cleanup_attempts == 1
                && s.cleanup_successes == 1
                && h.startup.calls == 1
                && h.startup.returned == Some(0)
                && h.startup.version == Some(0x0202)
                && h.startup.version_matched == Some(true)
                && h.startup.succeeded == Some(true)
                && !h.startup.reference_owned
                && h.cleanup.calls == 1
                && h.cleanup.returned == Some(0)
                && h.cleanup.last_error.is_none()
                && h.cleanup.qualified == Some(true)
                && h.cleanup.reference_remaining == 0
                && h.cleanup.eligible_before == Some(true)
                && h.cleanup.no_client_use_after_attempt == Some(true)
                && h.cleanup.close_error_reads == 0
                && h.cleanup.cleanup_error_reads == 0
                && h.ownership.remaining_server_sockets == 0
                && h.ownership.remaining_client_sockets == 0
                && h.ownership.in_flight_client_actions == 0
                && h.ownership.completion_obligations == 0
                && h.ownership.harness_reference_held_at_server_cleanup == Some(true)
                && h.ownership.no_ambient_startup_reliance == Some(true)
                && h.ownership.server_peak == t.socket_peak
                && h.ownership.server_peak <= 3
                && h.ownership.client_peak == 2
                && h.ownership.combined_peak <= 5
                && h.slots.iter().all(|c| {
                    c.close.calls == 1
                        && c.close.returned == Some(0)
                        && c.close.last_error.is_none()
                        && c.close.qualified == Some(true)
                        && !c.close.owned_at_final
                        && !c.close.in_flight
                });
            let server_setup = [
                s.startup_succeeded,
                s.version22,
                s.exclusive_ack,
                s.listener_linger_off,
                s.listener_nonblocking,
                s.bound_endpoint_matched,
                s.listen_succeeded,
                s.accepted_local_matched,
                s.accepted_peer_matched,
                s.stream_linger_off,
                s.stream_nonblocking,
            ]
            .into_iter()
            .all(|v| v == Some(true))
                && s.setup_calls == 9
                && s.socket_creates == 1
                && s.binds == 1
                && s.listens == 1
                && s.accepted_setup_calls == 4 * s.accepts
                && s.listener_close_attempts == 1
                && s.listener_close_successes == 1
                && s.stream_close_attempts == s.accepts
                && s.stream_close_successes == s.accepts;
            let clock = r.timing.original_clock_matched == Some(true)
                && r.timing.close_immediately_after_target == Some(true)
                && r.timing.first_close_adopted == Some(true)
                && r.timing.within_close_deadline == Some(true)
                && r.timing
                    .target_at_ms
                    .zip(r.timing.first_close_at_ms)
                    .is_some_and(|(target, close)| target < 45000 && target <= close)
                && r.timing
                    .first_close_at_ms
                    .zip(r.timing.final_at_ms)
                    .zip(r.timing.cleanup_elapsed_ms)
                    .is_some_and(|((close, end), elapsed)| {
                        close <= end
                            && elapsed < 10000
                            && (end - close == elapsed || end - close == elapsed + 1)
                    });
            let idle_a = a.data.send_calls == 0
                && a.data.recv_calls == 0
                && a.data.sent_bytes == 0
                && a.data.received_bytes == 0
                && a.completed(Connection::Connected);
            let legacy = self.connection == Connection::Refused
                && self.legacy_order
                && r.result == "legacyAdmissionRefused"
                && r.failure == "legacyAdmissionRefused"
                && b.completed(Connection::Refused)
                && b.data.send_calls == 0
                && b.data.recv_calls == 0
                && b.data.sent_bytes == 0
                && b.data.received_bytes == 0
                && s.accepts == 1
                && s.parsed_requests == 0
                && s.request_bytes == 0
                && s.sent_bytes == 0
                && t.accepted_total == 1
                && t.winner_bindings == 0
                && t.chosen_slot.is_none()
                && t.first_chunk_bytes == 0
                && t.listener_retirement == Some("legacyFirstAccept")
                && t.listener_retired == Some(true)
                && t.slots[0].accepted
                && t.slots[0].setup_calls == 4
                && t.slots[0].recv_bytes == 0
                && t.slots[0].close_attempts == 1
                && t.slots[0].close_successes == 1
                && !t.slots[0].owned_at_final
                && !t.slots[1].accepted
                && t.slots[1].setup_calls == 0
                && t.slots[1]
                    == TransportSlot {
                        ordinal: 2,
                        ..TransportSlot::default()
                    }
                && t.auxiliary_count == 0
                && t.seal_complete.is_none()
                && t.received_bytes == 0
                && t.send_attempts == 0
                && t.recv_attempts == t.slots[0].recv_attempts
                && !t.slots[0].chosen
                && t.slots[0].sent_bytes == 0
                && t.slots[0].parsed_requests == 0
                && t.slots[0].eof_count == 0
                && t.slots[0].seal_attempt_count == 0
                && t.slots[0].seal_final_count == 0
                && t.slots[0].seal_class.is_none()
                && t.slots[0].accepted_local_matched == Some(true)
                && t.slots[0].accepted_peer_matched == Some(true)
                && t.slots[0].linger_off == Some(true)
                && t.slots[0].nonblocking == Some(true)
                && t.slots[0].nonprogress_count <= t.slots[0].recv_attempts
                && h.ownership.server_peak == 2
                && h.ownership.combined_peak == 3
                && a.ordinal == 1
                && b.ordinal == 2
                && s.request_valid.is_none()
                && s.response_complete.is_none();
            // Retain the exact same Connected expectation for a future reviewed
            // bounded core, but never synthesize that unimplemented behavior.
            let green = self.connection == Connection::Connected
                && r.result == "transportGreen"
                && r.failure == "none"
                && b.completed(Connection::Connected)
                && b.data.request_matched == Some(true)
                && b.data.response_matched == Some(true)
                && b.data.received_bytes == 139
                && s.accepts == 2
                && s.parsed_requests == 1
                && s.sent_bytes == 139
                && t.accepted_total == 2
                && t.chosen_slot == Some(2)
                && t.winner_bindings == 1
                && t.auxiliary_count == 1
                && t.seal_complete == Some(true)
                && t.listener_retirement == Some("capacityReached")
                && t.slots[0].seal_final_count == 1
                && t.slots[0].recv_bytes == 0
                && t.slots[0].sent_bytes == 0
                && t.slots[0].close_successes == 1
                && t.slots[1].chosen;
            drained
                && budget
                && server_setup
                && clock
                && idle_a
                && t.current == Some(true)
                && t.original_timely == Some(true)
                && !t.pending_accept
                && s.io_attempts == t.accept_attempts + t.recv_attempts + t.send_attempts
                && s.io_attempts <= 2048
                && (legacy || green)
        }
    }
    pub(in super::super) fn exercise(run_id: String) -> OutcomeRecord {
        Driver::new(Instant::now()).finish(run_id)
    }
}
