import Foundation

public enum CLIAuthenticationScriptError: Error, Equatable {
    case unsupportedProvider
    case invalidCompletionTokenPipe
    case invalidLoginTimeout
}

public struct CLIAuthenticationScriptBuilder: Sendable {
    public init() {}

    // Guardian calls commit in-process. Shell's child only stages; its parent
    // commits with a builtin after the child has actually exited and been reaped.
    static let atomicStatusPublicationSubroutine = #"""
    use File::Temp (); use Fcntl qw(:DEFAULT);
    sub atomic_publish_status {
      my ($path, $value, $staging_only) = @_;
      my ($handle, $temporary, @identity);
      my $ok = eval {
        if ($staging_only) {
          $temporary = $path;
          sysopen($handle, $temporary, O_WRONLY | O_CREAT | O_EXCL, 0600) or die "publication create";
        } else {
          ($handle, $temporary) = File::Temp::tempfile("${path}.publish.XXXXXXXX", UNLINK => 0);
        }
        @identity = stat($handle);
        @identity && $identity[4] == $< && $identity[3] == 1 or die "publication identity";
        chmod(0600, $handle) == 1 or die "publication mode";
        my $bytes = "$value\n";
        my $offset = 0;
        while ($offset < length($bytes)) {
          my $written = syswrite($handle, $bytes, length($bytes) - $offset, $offset);
          if (!defined($written)) { next if $! == EINTR; die "publication write"; }
          $written > 0 or die "publication short write";
          $offset += $written;
        }
        close($handle) or die "publication close";
        undef $handle;
        my @staged = lstat($temporary);
        @staged && $staged[0] == $identity[0] && $staged[1] == $identity[1]
          && $staged[4] == $< && $staged[3] == 1 && ($staged[2] & 0777) == 0600 or die "publication changed";
        if (!$staging_only) { rename($temporary, $path) or die "publication rename"; }
        undef $temporary;
        1;
      };
      if (!$ok) {
        close($handle) if defined($handle);
        if (defined($temporary) && @identity) {
          my @current = lstat($temporary);
          unlink($temporary) if @current && $current[0] == $identity[0] && $current[1] == $identity[1];
        }
        return 0;
      }
      return 1;
    }
    """#

    static let statusStagingProgram = #"""
    use strict; use warnings; use Errno qw(EINTR);
    \#(atomicStatusPublicationSubroutine)
    my ($path, $value) = @ARGV;
    $SIG{ALRM} = sub { die "publication deadline\n"; };
    alarm 1;
    my $ok = atomic_publish_status($path, $value, 1);
    alarm 0;
    exit($ok ? 0 : 79);
    """#

    static let guardianStopPermissionProbeProgram = #"""
    use strict; use warnings; use Errno qw(EINTR);
    my ($pid, $identity_path, $permission_path) = @ARGV;
    exit 1 unless $pid =~ /^[1-9][0-9]*$/;
    $SIG{ALRM} = sub { exit 1 }; alarm 1;
    my $read = sub {
      my ($path) = @_;
      my @before = lstat($path);
      return undef unless @before && -f _ && $before[4] == $< && $before[3] == 1 && $before[7] <= 128 && ($before[2] & 0777) == 0600;
      open my $handle, "<", $path or return undef;
      my @opened = stat($handle);
      if (!@opened || $opened[0] != $before[0] || $opened[1] != $before[1]
          || $opened[4] != $< || $opened[3] != 1 || ($opened[2] & 0777) != 0600) { close $handle; return undef; }
      my $text = ""; my $eof = 0;
      while (length($text) <= 128) {
        my $count = sysread($handle, my $chunk, 129 - length($text));
        if (!defined($count)) { next if $! == EINTR; close $handle; return undef; }
        if ($count == 0) { $eof = 1; last; }
        $text .= $chunk;
      }
      close($handle) or return undef;
      return $eof && length($text) <= 128 ? $text : undef;
    };
    my $identity = $read->($identity_path);
    exit 1 unless defined($identity) && $identity =~ /^\Q$pid\E ([0-9a-f]{32})\n\z/;
    my $nonce = $1;
    my $permission = $read->($permission_path);
    exit(defined($permission) && $permission eq "guardian_stop_accepted_v1:$pid:$nonce\n" ? 0 : 1);
    """#

    static let boundedProcessStateProbeSubroutine = #"""
    sub bounded_process_state {
      return bounded_process_state_with_completion(undef, @_);
    }
    sub bounded_process_state_with_completion {
      my ($completion, $probe_path, @probe_args) = @_;
      $$completion = undef if defined($completion);
      return undef unless defined($probe_path) && length($probe_path);
      pipe(my $reader, my $writer) or return undef;
      my $flags = fcntl($reader, F_GETFL, 0);
      if (!defined($flags) || !defined(fcntl($reader, F_SETFL, $flags | O_NONBLOCK))) {
        close $reader;
        close $writer;
        return undef;
      }
      my $probe_pid = fork();
      if (!defined($probe_pid)) { close $reader; close $writer; return undef; }
      if ($probe_pid == 0) {
        close $reader;
        open STDOUT, ">&", $writer or _exit(127);
        close $writer;
        open STDERR, ">", "/dev/null";
        exec { $probe_path } $probe_path, @probe_args;
        _exit(127);
      }
      close $writer;
      my $deadline = clock_gettime(CLOCK_MONOTONIC) + 0.25;
      my $output = "";
      my ($eof, $child_done, $child_owned, $child_status, $read_failed) = (0, 0, 1, 0, 0);
      while (clock_gettime(CLOCK_MONOTONIC) < $deadline) {
        while (!$eof) {
          my $count = sysread($reader, my $chunk, 1024);
          if (!defined($count)) {
            next if $! == EINTR && clock_gettime(CLOCK_MONOTONIC) < $deadline;
            last if $! == EAGAIN || $! == EWOULDBLOCK;
            $read_failed = 1;
            last;
          }
          if ($count == 0) { $eof = 1; last; }
          $output .= $chunk;
          if (length($output) > 4096) { $read_failed = 1; last; }
        }
        last if $read_failed;
        if (!$child_done) {
          my $waited = waitpid($probe_pid, WNOHANG);
          if ($waited == $probe_pid) { $child_done = 1; $child_status = $?; }
          elsif ($waited == -1 && $! != EINTR) { $read_failed = 1; $child_owned = 0; last; }
        }
        last if $child_done && $eof;
        select undef, undef, undef, 0.01;
      }
      if ($child_done && $eof && !$read_failed && $child_status == 0) {
        close $reader;
        # List callers may identify the exact query child after it was reaped.
        # Scalar callers retain the original process-state text contract.
        return wantarray ? ($output, $probe_pid) : $output;
      }
      # Only a timely, fully drained and reaped empty exit1 is confirmed absence.
      # Mark before cleanup so timeout/read/ownership/cancellation failures stay unknown.
      if (defined($completion) && $child_done && $child_owned && $eof && !$read_failed
          && $child_status == (1 << 8) && length($output) == 0
          && clock_gettime(CLOCK_MONOTONIC) < $deadline) {
        $$completion = 1;
      }
      close $reader;
      if (!$child_done && $child_owned) {
        kill "TERM", $probe_pid;
        my $term_deadline = clock_gettime(CLOCK_MONOTONIC) + 0.05;
        while (clock_gettime(CLOCK_MONOTONIC) < $term_deadline) {
          my $waited = waitpid($probe_pid, WNOHANG);
          if ($waited == $probe_pid) { $child_done = 1; last; }
          if ($waited == -1 && $! != EINTR) { $child_owned = 0; last; }
          select undef, undef, undef, 0.01;
        }
        if (!$child_done && $child_owned) {
          kill "KILL", $probe_pid;
          my $kill_deadline = clock_gettime(CLOCK_MONOTONIC) + 0.1;
          while (clock_gettime(CLOCK_MONOTONIC) < $kill_deadline) {
            my $waited = waitpid($probe_pid, WNOHANG);
            if ($waited == $probe_pid) { $child_done = 1; last; }
            if ($waited == -1 && $! != EINTR) { $child_owned = 0; last; }
            select undef, undef, undef, 0.01;
          }
        }
      }
      return undef;
    }
    """#

    static let boundedProcessStateProbeProgram = #"""
    use strict; use warnings; use Fcntl qw(F_GETFL F_SETFL O_NONBLOCK); use POSIX qw(_exit waitpid WNOHANG); use Errno qw(EINTR EAGAIN EWOULDBLOCK); use Time::HiRes qw(clock_gettime CLOCK_MONOTONIC);
    \#(boundedProcessStateProbeSubroutine)
    my ($probe_path, @probe_args) = @ARGV;
    my $owned_query = @probe_args && $probe_args[0] eq "--owned-query-completion";
    shift @probe_args if $owned_query;
    my $completion;
    my $output = $owned_query
      ? bounded_process_state_with_completion(\$completion, $probe_path, @probe_args)
      : bounded_process_state($probe_path, @probe_args);
    defined($output) or exit($owned_query && defined($completion) && $completion == 1 ? 1 : 2);
    print $output;
    exit 0;
    """#

    public func build(
        provider: UsageProvider,
        executableURL: URL,
        completionTokenPipeURL: URL? = nil,
        completionStatusFileURL: URL? = nil,
        completionExecStatusFileURL: URL? = nil,
        cancellationRequestFileURL: URL? = nil,
        completionOpenExecutableURL: URL = URL(fileURLWithPath: "/usr/bin/open"),
        watchdogExecutableURL: URL = URL(fileURLWithPath: "/usr/bin/perl"),
        processStateExecutableURL: URL = URL(fileURLWithPath: "/bin/ps"),
        loginTimeoutSeconds: Int = 300
    ) throws -> String {
        let command: String
        switch provider {
        case .claude:
            command = "exec \(shellQuote(executableURL.path)) auth login"
        case .codex:
            command = "exec \(shellQuote(executableURL.path)) login"
        case .gemini:
            guard let completionTokenPipeURL, completionTokenPipeURL.isFileURL,
                  !completionTokenPipeURL.path.isEmpty else {
                throw CLIAuthenticationScriptError.invalidCompletionTokenPipe
            }
            guard (1...1_800).contains(loginTimeoutSeconds) else {
                throw CLIAuthenticationScriptError.invalidLoginTimeout
            }
            if let completionStatusFileURL,
               (!completionStatusFileURL.isFileURL || completionStatusFileURL.path.isEmpty) {
                throw CLIAuthenticationScriptError.invalidCompletionTokenPipe
            }
            let completionStatusPath = completionStatusFileURL?.path
                ?? completionTokenPipeURL.appendingPathExtension("status").path
            let completionExecStatusPath = completionExecStatusFileURL?.path
                ?? completionStatusPath + ".exec"
            if let completionExecStatusFileURL,
               (!completionExecStatusFileURL.isFileURL || completionExecStatusFileURL.path.isEmpty) {
                throw CLIAuthenticationScriptError.invalidCompletionTokenPipe
            }
            let cancellationRequestPath = cancellationRequestFileURL?.path ?? completionStatusPath + ".cancel"
            if let cancellationRequestFileURL,
               (!cancellationRequestFileURL.isFileURL || cancellationRequestFileURL.path.isEmpty) {
                throw CLIAuthenticationScriptError.invalidCompletionTokenPipe
            }
            let coordinationPaths = [completionTokenPipeURL.path, completionStatusPath, completionExecStatusPath,
                                     cancellationRequestPath] + ["lock", "ready", "process-group", "guardian", "guardian-stop", "stopped", "stopped.tmp"]
                .map { completionStatusPath + "." + $0 }
            let standardizedPaths = coordinationPaths.map { URL(fileURLWithPath: $0).standardizedFileURL.path }
            guard Set(standardizedPaths).count == standardizedPaths.count else {
                throw CLIAuthenticationScriptError.invalidCompletionTokenPipe
            }
            guard processStateExecutableURL.isFileURL, !processStateExecutableURL.path.isEmpty else {
                throw CLIAuthenticationScriptError.invalidCompletionTokenPipe
            }
            let supervisorProgram = #"""
            use strict; use warnings; use Fcntl qw(:DEFAULT O_NONBLOCK); use POSIX qw(setpgid getpgrp waitpid WNOHANG WUNTRACED WIFSTOPPED _exit); use Errno qw(EINTR EAGAIN EWOULDBLOCK EACCES); use Time::HiRes qw(time sleep clock_gettime CLOCK_MONOTONIC);
            \#(Self.boundedProcessStateProbeSubroutine)
            \#(Self.atomicStatusPublicationSubroutine)
            my ($gate_path, $status_path, $cancel_request_path, $exec_status_path, $process_group_file, $guardian_state_file, $guardian_stop_file, $process_state_path, $parent_pid, @command) = @ARGV;
            my ($publication_failed, $cleanup_uncertain) = (0, 0);
            my $publish_status = sub {
              return 0 if $publication_failed;
              return 1 if atomic_publish_status($status_path, $_[0]);
              $publication_failed = 1; $cleanup_uncertain = 1;
              return 0;
            };
            my $publish_launch_stage = sub {
              my ($stage) = @_;
              open my $handle, ">", $exec_status_path or _exit(127);
              chmod 0600, $exec_status_path or _exit(127);
              print {$handle} "$stage\n" or _exit(127);
              close $handle or _exit(127);
            };
            # Establish the actual owner before any potentially contended gate.
            setpgid(0, 0) == 0 or _exit(127);
            open my $random, "<", "/dev/urandom" or _exit(127); read($random, my $nonce_bytes, 16) == 16 or _exit(127); close $random; my $nonce = unpack("H*", $nonce_bytes);
            open my $identity_handle, ">", $guardian_state_file or _exit(127); chmod 0600, $guardian_state_file or _exit(127); print {$identity_handle} "$$ $nonce\n" or _exit(127); close $identity_handle or _exit(127);
            my $stopping = 0;
            sub group_peers {
              my ($process_rows, $query_pid) = bounded_process_state($process_state_path, "-o", "pid=,stat=", "-g", "$$");
              return undef unless defined($process_rows) && defined($query_pid) && $query_pid > 0 && $query_pid != $$;
              my $found = 0;
              my $self_seen = 0;
              my $query_seen = 0;
              for my $line (split /\n/, $process_rows) {
                my ($pid, $state) = $line =~ /^\s*([0-9]+)\s+(\S+)\s*$/;
                return undef unless defined($pid) && defined($state) && $pid > 0;
                if ($pid == $$) {
                  return undef if $self_seen || $state =~ /Z/;
                  $self_seen = 1;
                  next;
                }
                # /bin/ps was part of this group in its own snapshot, but the
                # bounded probe has already reaped this exact child. Every
                # other live PID still prevents a clean completion receipt.
                if ($pid == $query_pid) { return undef if $query_seen++; next; }
                if ($state !~ /Z/) { $found = 1; last; }
              }
              return undef unless $self_seen;
              return $found;
            }
            my $read_stop_control = sub {
              my ($path) = @_;
              my @metadata = lstat($path);
              return undef unless @metadata && -f _ && $metadata[4] == $< && $metadata[3] == 1 && $metadata[7] <= 128;
              open my $handle, "<", $path or return undef;
              my @opened = stat($handle);
              if (!@opened || $opened[0] != $metadata[0] || $opened[1] != $metadata[1]) { close $handle; return undef; }
              my $value = ""; my $eof = 0;
              while (length($value) <= 128) {
                my $count = sysread($handle, my $chunk, 129 - length($value));
                if (!defined($count)) { next if $! == EINTR; close $handle; return undef; }
                if ($count == 0) { $eof = 1; last; }
                $value .= $chunk;
              }
              close($handle) or return undef;
              return $eof && length($value) <= 128 ? $value : undef;
            };
            my $matched_stop_request = sub {
              my $cancel = $read_stop_control->($cancel_request_path);
              return 1 if defined($cancel) && $cancel eq "cancelled\n";
              my $request = $read_stop_control->($guardian_stop_file);
              my $identity = $read_stop_control->($guardian_state_file);
              return defined($request) && defined($identity) && $$ == getpgrp()
                && $request eq $identity && $identity =~ /^$$ [0-9a-f]{32}\n\z/;
            };
            my $stop_requested = sub { return getppid() != $parent_pid || $matched_stop_request->(); };
            my $mark_cleanup_unconfirmed = sub {
              $cleanup_uncertain = 1;
              $publish_status->("helper_stop_unconfirmed");
            };
            my ($resume_reader, $resume_writer, $pre_exec_child); my $child_owned = 0;
            my $clean_group = sub {
              my ($signal) = @_; return if $stopping; $stopping = 1; $SIG{TERM} = "IGNORE"; $SIG{INT} = "IGNORE"; $SIG{HUP} = "IGNORE";
              if (defined($resume_writer)) { close $resume_writer; undef $resume_writer; }
              if (defined($pre_exec_child) && !$child_owned) { $cleanup_uncertain = 1; undef $pre_exec_child; }
              if (defined($pre_exec_child)) {
                # Only an unreaped directly owned child is an individual target.
                # Lost ownership is monotonic and never rebuilt from its PID.
                kill "KILL", $pre_exec_child;
                my $reap_deadline = clock_gettime(CLOCK_MONOTONIC) + 0.25;
                while (clock_gettime(CLOCK_MONOTONIC) < $reap_deadline) {
                  my $waited = waitpid($pre_exec_child, WNOHANG);
                  if ($waited == $pre_exec_child) { undef $pre_exec_child; $child_owned = 0; last; }
                  if ($waited == -1 && $! != EINTR) { $cleanup_uncertain = 1; $child_owned = 0; undef $pre_exec_child; last; }
                  sleep 0.01;
                }
              }
              $cleanup_uncertain = 1 if defined($pre_exec_child);
              my $identity = ""; if (open my $handle, "<", $guardian_state_file) { $identity = <$handle> // ""; close $handle; }
              if ($$ == getpgrp() && $identity eq "$$ $nonce\n") {
                # This is acceptance, not cleanup completion. No uncertainty may
                # follow this permission: this one-way routine never returns.
                if (!$cleanup_uncertain && !$publication_failed && $matched_stop_request->()) {
                  if (!atomic_publish_status("${status_path}.stopped", "guardian_stop_accepted_v1:$$:$nonce")) {
                    $publication_failed = 1; $cleanup_uncertain = 1;
                  }
                }
                # Do not fork /bin/ps from a Perl signal handler. TERM gets a
                # fixed grace period, then the still-owned group is killed.
                # Perl expresses process-group delivery with a negative signal
                # and a positive process-group ID.
                kill "-TERM", $$; sleep 1.0;
                kill "-KILL", $$;
              }
              _exit($signal eq "INT" ? 130 : $signal eq "HUP" ? 129 : 143);
            };
            $SIG{TERM} = sub { $clean_group->("TERM"); }; $SIG{INT} = sub { $clean_group->("INT"); }; $SIG{HUP} = sub { $clean_group->("HUP"); };
            open my $gate, "+<", $gate_path or do { $publish_launch_stage->("pre_exec_failed"); _exit(127); };
            my $flags = fcntl($gate, F_GETFD, 0); defined($flags) && defined(fcntl($gate, F_SETFD, $flags | FD_CLOEXEC)) or do { close $gate; $publish_launch_stage->("pre_exec_failed"); _exit(127); };
            my $record_lock = pack("qqiss", 0, 0, 0, F_WRLCK, SEEK_SET);
            my $launch_deadline = clock_gettime(CLOCK_MONOTONIC) + 10;
            while (1) {
              if ($stop_requested->()) { close $gate; $publish_launch_stage->("pre_exec_cancelled"); _exit(130); }
              if (clock_gettime(CLOCK_MONOTONIC) >= $launch_deadline) { close $gate; $publish_launch_stage->("launch_gate_timeout"); _exit(124); }
              last if defined(fcntl($gate, F_SETLK, $record_lock));
              if ($! != EINTR && $! != EACCES && $! != EAGAIN && $! != EWOULDBLOCK) { close $gate; $publish_launch_stage->("pre_exec_failed"); _exit(127); }
              sleep 0.05;
            }
            my $status = ""; if (open my $handle, "<", $status_path) { $status = <$handle> // ""; close $handle; chomp $status; }
            if ($status ne "ready" || $stop_requested->()) { close $gate; $publish_launch_stage->("pre_exec_cancelled"); _exit(130); }
            close $gate;
            open STDIN, "<", "/dev/tty" or do { $publish_launch_stage->("pre_exec_failed"); _exit(127); };
            open STDOUT, ">", "/dev/tty" or do { $publish_launch_stage->("pre_exec_failed"); _exit(127); };
            open STDERR, ">", "/dev/tty" or do { $publish_launch_stage->("pre_exec_failed"); _exit(127); };
            if ($stop_requested->()) { $publish_launch_stage->("pre_exec_cancelled"); _exit(130); }
            pipe(my $exec_status_reader, my $exec_status_writer) or do { open my $exec_status, ">", $exec_status_path; print {$exec_status} "launch_indeterminate\n" if $exec_status; close $exec_status if $exec_status; _exit(127); };
            my $reader_flags = fcntl($exec_status_reader, F_GETFL, 0); defined($reader_flags) && defined(fcntl($exec_status_reader, F_SETFL, $reader_flags | O_NONBLOCK)) or do { close $exec_status_reader; close $exec_status_writer; open my $exec_status, ">", $exec_status_path; print {$exec_status} "launch_indeterminate\n" if $exec_status; close $exec_status if $exec_status; _exit(127); };
            my $exec_flags = fcntl($exec_status_writer, F_GETFD, 0); defined($exec_flags) && defined(fcntl($exec_status_writer, F_SETFD, $exec_flags | FD_CLOEXEC)) or do { close $exec_status_reader; close $exec_status_writer; open my $exec_status, ">", $exec_status_path; print {$exec_status} "launch_indeterminate\n" if $exec_status; close $exec_status if $exec_status; _exit(127); };
            my $guardian_pid = $$;
            if ($stop_requested->()) { $publish_launch_stage->("pre_exec_cancelled"); _exit(130); }
            pipe($resume_reader, $resume_writer) or do { $publish_launch_stage->("pre_exec_failed"); _exit(127); };
            for my $handle ($resume_reader, $resume_writer) {
              my $flags = fcntl($handle, F_GETFD, 0);
              defined($flags) && defined(fcntl($handle, F_SETFD, $flags | FD_CLOEXEC)) or do { $publish_launch_stage->("pre_exec_failed"); _exit(127); };
            }
            my $resume_flags = fcntl($resume_reader, F_GETFL, 0);
            defined($resume_flags) && defined(fcntl($resume_reader, F_SETFL, $resume_flags | O_NONBLOCK)) or do { $publish_launch_stage->("pre_exec_failed"); _exit(127); };
            # Preserve the inherited mask across the real fork/registration and
            # child-handler switch. Pending cleanup cannot skip a new owner.
            my $fork_signals = POSIX::SigSet->new(POSIX::SIGTERM(), POSIX::SIGINT(), POSIX::SIGHUP());
            my $original_signals = POSIX::SigSet->new();
            defined(POSIX::sigprocmask(POSIX::SIG_BLOCK(), $fork_signals, $original_signals)) or do { $mark_cleanup_unconfirmed->(); $clean_group->("TERM"); _exit(127); };
            my $child = fork(); defined($child) or do {
              $cleanup_uncertain = 1;
              defined(POSIX::sigprocmask(POSIX::SIG_SETMASK(), $original_signals)) or do { $clean_group->("TERM"); _exit(127); };
              close $exec_status_reader; close $exec_status_writer; open my $exec_status, ">", $exec_status_path; print {$exec_status} "launch_indeterminate\n" if $exec_status; close $exec_status if $exec_status; _exit(127);
            };
            if ($child == 0) {
              close $exec_status_reader; close $resume_writer; undef $resume_writer;
              $SIG{TERM} = "DEFAULT"; $SIG{INT} = "DEFAULT"; $SIG{HUP} = "DEFAULT";
              defined(POSIX::sigprocmask(POSIX::SIG_SETMASK(), $original_signals)) or _exit(127);
              kill "STOP", $$ or _exit(127);
              # A resumed process still needs its guardian's private permission.
              # An incidental CONT during shutdown must never authorize exec.
              my $permission = "";
              my $permission_deadline = clock_gettime(CLOCK_MONOTONIC) + 12;
              while (clock_gettime(CLOCK_MONOTONIC) < $permission_deadline) {
                my $cancel = ""; if (open my $handle, "<", $cancel_request_path) { $cancel = <$handle> // ""; close $handle; chomp $cancel; }
                if ($cancel eq "cancelled" || getppid() != $guardian_pid) { close $resume_reader; syswrite($exec_status_writer, "cancelled\n"); _exit(130); }
                my $count = sysread($resume_reader, $permission, 1);
                if (defined($count)) { last; }
                last unless $! == EINTR || $! == EAGAIN || $! == EWOULDBLOCK;
                sleep 0.02;
              }
              close $resume_reader;
              if ($permission ne "1") { syswrite($exec_status_writer, "not_authorized\n"); _exit(127); }
              my $cancel = ""; if (open my $handle, "<", $cancel_request_path) { $cancel = <$handle> // ""; close $handle; chomp $cancel; }
              if ($cancel eq "cancelled" || getppid() != $guardian_pid) { syswrite($exec_status_writer, "cancelled\n"); _exit(130); }
              my $status = ""; if (open my $handle, "<", $status_path) { $status = <$handle> // ""; close $handle; chomp $status; }
              if ($status ne "ready") { syswrite($exec_status_writer, "not_authorized\n"); _exit(127); }
              exec @command;
              my $exec_error = "$!\n"; my $write_offset = 0;
              while ($write_offset < length($exec_error)) {
                my $written = syswrite($exec_status_writer, $exec_error, length($exec_error) - $write_offset, $write_offset);
                if (!defined($written)) { next if $! == EINTR; last; }
                last if $written == 0;
                $write_offset += $written;
              }
              _exit(127);
            }
            $pre_exec_child = $child; $child_owned = 1;
            defined(POSIX::sigprocmask(POSIX::SIG_SETMASK(), $original_signals)) or do { $mark_cleanup_unconfirmed->(); $clean_group->("TERM"); _exit(127); };
            close $exec_status_writer; close $resume_reader; undef $resume_reader;
            my $child_stopped = 0; my $stop_deadline = time + 2;
            while (time < $stop_deadline) {
              if ($stop_requested->()) { $clean_group->("TERM"); }
              # Reaping/loss and ownership retirement are one short transition.
              # No poll sleep or blocking wait occurs with cleanup masked.
              my $stop_wait_original = POSIX::SigSet->new();
              defined(POSIX::sigprocmask(POSIX::SIG_BLOCK(), $fork_signals, $stop_wait_original)) or do { $mark_cleanup_unconfirmed->(); $clean_group->("TERM"); _exit(127); };
              my $waited = waitpid($child, WNOHANG | WUNTRACED);
              my $stop_wait_error = 0 + $!;
              my $stop_wait_status = ${^CHILD_ERROR_NATIVE};
              if ($waited == $child) {
                $child_stopped = WIFSTOPPED($stop_wait_status) ? 1 : 0;
                if (!$child_stopped) { $cleanup_uncertain = 1; undef $pre_exec_child; $child_owned = 0; }
              } elsif ($waited == -1 && $stop_wait_error != EINTR) {
                $cleanup_uncertain = 1; $child_owned = 0; undef $pre_exec_child;
              }
              defined(POSIX::sigprocmask(POSIX::SIG_SETMASK(), $stop_wait_original)) or do { $mark_cleanup_unconfirmed->(); $clean_group->("TERM"); _exit(127); };
              last if $waited == $child || ($waited == -1 && $stop_wait_error != EINTR);
              sleep 0.02;
            }
            if (!$child_stopped) {
              $cleanup_uncertain = 1;
              open my $failed_exec_status, ">", $exec_status_path;
              if ($failed_exec_status) { print {$failed_exec_status} "launch_indeterminate\n"; close $failed_exec_status; }
              $clean_group->("TERM");
              _exit(127);
            }
            $pre_exec_child = $child;
            open my $group_handle, ">", $process_group_file or do { $clean_group->("TERM"); _exit(127); };
            chmod 0600, $process_group_file or do { close $group_handle; $clean_group->("TERM"); _exit(127); };
            print {$group_handle} "$$\n" or do { close $group_handle; $clean_group->("TERM"); _exit(127); };
            close $group_handle or do { $clean_group->("TERM"); _exit(127); };
            # Only the stopped CLI needs CONT. Sending it to the running
            # guardian can overwrite zsh's saved wait status on macOS.
            my $foreground_deadline = clock_gettime(CLOCK_MONOTONIC) + 10;
            while (1) {
              if ($stop_requested->()) { $clean_group->("TERM"); }
              if (clock_gettime(CLOCK_MONOTONIC) >= $foreground_deadline) {
                $mark_cleanup_unconfirmed->(); $clean_group->("TERM");
              }
              my $foreground_group = POSIX::tcgetpgrp(fileno(STDIN));
              if (!defined($foreground_group) || $foreground_group <= 0) {
                next if $! == EINTR;
                $mark_cleanup_unconfirmed->(); $clean_group->("TERM");
              }
              last if $foreground_group == $$;
              sleep 0.02;
            }
            if ($stop_requested->()) { $clean_group->("TERM"); }
            kill "CONT", $child or do { $mark_cleanup_unconfirmed->(); $clean_group->("TERM"); };
            my $permission_written;
            {
              local $SIG{PIPE} = "IGNORE";
              my $write_deadline = clock_gettime(CLOCK_MONOTONIC) + 1;
              while (clock_gettime(CLOCK_MONOTONIC) < $write_deadline) {
                $permission_written = syswrite($resume_writer, "1", 1);
                last if defined($permission_written) || $! != EINTR;
              }
            }
            if (!defined($permission_written) || $permission_written != 1) { $mark_cleanup_unconfirmed->(); $clean_group->("TERM"); }
            close $resume_writer; undef $resume_writer; undef $pre_exec_child;
            my $exec_error = ""; my $exec_pipe_closed = 0;
            while (!$exec_pipe_closed) {
              if ($stop_requested->()) { $clean_group->("TERM"); }
              my $read_count = sysread($exec_status_reader, my $chunk, 256);
              if (!defined($read_count)) { next if $! == EINTR; if ($! == EAGAIN || $! == EWOULDBLOCK) { sleep 0.02; next; } $exec_error = "indeterminate"; last; }
              if ($read_count == 0) { $exec_pipe_closed = 1; last; }
              $exec_error .= $chunk;
            }
            close $exec_status_reader;
            my $exec_state = "exec_confirmed";
            my $child_status = 0; my $child_done = 0;
            if (length($exec_error)) { $exec_state = $exec_error eq "cancelled\n" ? "pre_exec_cancelled" : $exec_error =~ /\n\z/ ? "exec_failed" : "launch_indeterminate"; }
            else { my $probe = waitpid($child, WNOHANG); if ($probe == $child) { $child_status = $?; $child_done = 1; $child_owned = 0; $exec_state = "launch_indeterminate" if ($child_status & 127); } elsif ($probe == -1 && $! != EINTR) { $child_owned = 0; $cleanup_uncertain = 1; $exec_state = "launch_indeterminate"; } }
            my $exec_status_ok = 0;
            if (open my $exec_status, ">", $exec_status_path) {
              $exec_status_ok = chmod(0600, $exec_status_path)
                && print({$exec_status} "$exec_state\n")
                && close($exec_status);
              close $exec_status if !$exec_status_ok;
            }
            if (!$exec_status_ok) { $mark_cleanup_unconfirmed->(); $clean_group->("TERM"); _exit(127); }
            if ($exec_state ne "exec_confirmed") {
              if ($exec_state eq "pre_exec_cancelled") { $pre_exec_child = $child if $child_owned && !$child_done; $clean_group->("TERM"); _exit(130); }
              if ($exec_state eq "launch_indeterminate") { $cleanup_uncertain = 1; $pre_exec_child = $child if $child_owned && !$child_done; $clean_group->("TERM"); _exit(127); }
              waitpid($child, 0); my $failed_status = $?; my $failed_exit = ($failed_status & 127) ? 128 + ($failed_status & 127) : ($failed_status >> 8); _exit($failed_exit || 127);
            }
            while (!$child_done) {
              if ($stop_requested->()) { $clean_group->("TERM"); }
              my $waited = waitpid($child, WNOHANG);
              if ($waited == $child) { $child_status = $?; $child_done = 1; $child_owned = 0; next; }
              if ($waited == -1 && $! != EINTR) { $child_owned = 0; $mark_cleanup_unconfirmed->(); $clean_group->("TERM"); _exit(125); }
              sleep 0.05;
            }
            my $exit_code = ($child_status & 127) ? 128 + ($child_status & 127) : ($child_status >> 8);
            while (1) {
              if ($stop_requested->()) { $clean_group->("TERM"); }
              my $has_peers = group_peers();
              if (!defined($has_peers)) {
                $cleanup_uncertain = 1;
                $publish_status->("process_state_unavailable");
                $clean_group->("TERM");
                _exit(127);
              }
              last unless $has_peers;
              sleep 0.1;
            }
            _exit($exit_code & 255);
            """#
            command = """
            token_pipe=\(shellQuote(completionTokenPipeURL.path))
            status_file=\(shellQuote(completionStatusPath))
            cancel_request_file=\(shellQuote(cancellationRequestPath))
            process_state_executable=\(shellQuote(processStateExecutableURL.path))
            launch_gate="${status_file}.lock"
            watchdog_ready_file="${status_file}.ready"
            process_group_file="${status_file}.process-group"
            guardian_state_file="${status_file}.guardian"
            guardian_stop_file="${status_file}.guardian-stop"
            rm -f -- "$process_group_file"
            rm -f -- "$guardian_state_file"
            rm -f -- "$guardian_stop_file"
            rm -f -- "$watchdog_ready_file"
            cleanup_ack_file="${status_file}.stopped"
            completion_token=""
            status_publication_failed=0
            status_publication_critical=0
            status_publication_sequence=0
            cleanup_state_uncertain=0
            launch_critical=0
            deferred_app_cancel=0
            deferred_cancel_exit=0
            cancellation_in_progress=0
            guardian_state_preserved=0
            report_status() {
              if (( status_publication_failed || status_publication_critical )); then cleanup_state_uncertain=1; guardian_state_preserved=1; return 1; fi
              case "$1" in
                process_state_unavailable|helper_stop_unconfirmed|guardian_identity_mismatch|terminal_foreground_restore_failed) cleanup_state_uncertain=1; guardian_state_preserved=1 ;;
              esac
              status_publication_critical=1
              if ! builtin zmodload -F zsh/files b:zf_mv b:zf_rm; then
                status_publication_failed=1; cleanup_state_uncertain=1; guardian_state_preserved=1
                status_publication_critical=0
                return 1
              fi
              (( status_publication_sequence += 1 ))
              local publisher_temporary="${status_file}.publish-shell.$$.${status_publication_sequence}"
              \(shellQuote(URL(fileURLWithPath: "/usr/bin/perl").path)) -e \(shellQuote(Self.statusStagingProgram)) "$publisher_temporary" "$1" &
              local publisher_spawn_status=$? publisher_pid publisher_exit publisher_commit=1
              if (( publisher_spawn_status != 0 )); then
                status_publication_failed=1; cleanup_state_uncertain=1; guardian_state_preserved=1
                status_publication_critical=0
                return 1
              fi
              publisher_pid=$!
              while true; do
                wait "$publisher_pid" 2>/dev/null
                publisher_exit=$?
                # Only the publisher's fixed normal exit codes prove a join.
                if [[ "$publisher_exit" == 0 || "$publisher_exit" == 79 ]]; then break; fi
                # Retry only builtin wait before creating any observer child.
                # A later child could reuse this PID and replace zsh's cached
                # wait status; after that point outcome must remain failed.
                wait "$publisher_pid" 2>/dev/null
                publisher_exit=$?
                if [[ "$publisher_exit" == 0 || "$publisher_exit" == 79 ]]; then break; fi
                status_publication_failed=1
                cleanup_state_uncertain=1
                guardian_state_preserved=1
                if /usr/bin/perl -MErrno=ESRCH -e 'my $pid=shift; local $SIG{ALRM}=sub { exit 1 }; alarm 1; local $!=0; my $n=kill 0,$pid; exit(!$n && $! == ESRCH ? 0 : 1);' "$publisher_pid" 2>/dev/null; then
                  wait "$publisher_pid" 2>/dev/null
                  publisher_exit=$?
                  break
                fi
                # Unknown/reused PID: keep this helper and defer all cleanup.
                # Observations never retry publication or signal a numeric PID.
                /bin/sleep 0.01 || true
              done
              if [[ "$publisher_exit" != 0 ]]; then status_publication_failed=1; cleanup_state_uncertain=1; guardian_state_preserved=1; fi
              if (( !status_publication_failed )); then
                if [[ -f "$publisher_temporary" && ! -L "$publisher_temporary" && -O "$publisher_temporary" ]] &&
                   [[ ( ! -e "$status_file" && ! -L "$status_file" ) || ( -f "$status_file" && ! -L "$status_file" && -O "$status_file" ) ]]; then
                  builtin zf_mv -f "$publisher_temporary" "$status_file"
                  publisher_commit=$?
                fi
                if (( publisher_commit != 0 )); then
                  status_publication_failed=1; cleanup_state_uncertain=1; guardian_state_preserved=1
                  # Only a successful stage/join proves this path is ours.
                  builtin zf_rm -f "$publisher_temporary" || true
                fi
              fi
              status_publication_critical=0
              if (( !cancellation_in_progress && !launch_critical && ${+functions[finish_launch_claim]} )); then
                finish_launch_claim
              elif (( !cancellation_in_progress && deferred_cancel_exit )); then
                exit "$deferred_cancel_exit"
              fi
              (( !status_publication_failed ))
            }
            publish_completed_failure() {
              (( !status_publication_failed && !status_publication_critical )) || return 1
              local failure
              [[ -r "$status_file" ]] || return 1
              failure="$(<"$status_file")"
              case "$failure" in
                missing_pipe|pipe_open_failed|handoff_timeout|invalid_handoff|callback_failed|watchdog_start_failed|terminal_foreground_failed) ;;
                *) return 1 ;;
              esac
              # Private per-attempt pathname + original failure bind this ACK.
              # No later handler/owner may write after this atomic publication.
              local temporary_ack="${cleanup_ack_file}.tmp"
              (umask 077; setopt noclobber; print -r -- "helper_stopped_v1:${status_file:t}:${failure}" > "$temporary_ack") || return 1
              builtin zmodload -F zsh/files b:zf_mv || return 1
              [[ -f "$temporary_ack" && ! -L "$temporary_ack" && -O "$temporary_ack" ]] || return 1
              [[ ( ! -e "$cleanup_ack_file" && ! -L "$cleanup_ack_file" ) || ( -f "$cleanup_ack_file" && ! -L "$cleanup_ack_file" && -O "$cleanup_ack_file" ) ]] || return 1
              builtin zf_mv -f "$temporary_ack" "$cleanup_ack_file"
            }
            cleanup_before_launch() {
              # This EXIT handler is replaced before the first child is spawned.
              # No CLI, supervisor or watchdog has existed in this phase.
              rm -f -- "$watchdog_ready_file"
              trap - EXIT
              publish_completed_failure || true
            }
            trap 'cleanup_before_launch' EXIT
            request_prelaunch_cancel() {
              if (( status_publication_critical )); then deferred_cancel_exit="$1"; else exit "$1"; fi
            }
            trap 'request_prelaunch_cancel 129' HUP
            trap 'request_prelaunch_cancel 130' INT
            trap 'request_prelaunch_cancel 143' TERM
            stop_requested() { [[ -r "$cancel_request_file" ]] && [[ "$(<"$cancel_request_file")" == cancelled ]]; }
            acknowledge_pre_exec_stop() {
              if [[ -r "$status_file" && "$(<"$status_file")" == ready ]] && stop_requested; then
                exec 3>&-
                rm -f -- "$token_pipe" "$watchdog_ready_file"
                trap - EXIT
                report_status helper_cancelled
              fi
            }
            print -r -- "AI Token Meter: Antigravity login helper started; waiting for the app handoff."
            if [[ ! -r "$status_file" ]] || [[ "$(<"$status_file")" != ready ]] || stop_requested; then
              acknowledge_pre_exec_stop
              exit 130
            fi
            if [[ ! -p "$token_pipe" ]]; then
              if stop_requested; then acknowledge_pre_exec_stop; exit 130; fi
              print -u2 -r -- "AI Token Meter: handoff pipe is missing. Close this window and start a new one-time quota check."
              report_status missing_pipe
              rm -f -- "$token_pipe"
              exit 124
            fi
            if ! exec 3<> "$token_pipe"; then
              if stop_requested; then acknowledge_pre_exec_stop; exit 130; fi
              print -u2 -r -- "AI Token Meter: could not open the app handoff pipe. Close this window and start a new one-time quota check."
              report_status pipe_open_failed
              rm -f -- "$token_pipe"
              exit 124
            fi
            handoff_waited=0
            handoff_received=0
            while (( handoff_waited < \(loginTimeoutSeconds) )); do
              if IFS= read -r -t 1 -u 3 completion_token; then
                handoff_received=1
                break
              fi
              if [[ ! -r "$status_file" ]] || [[ "$(<"$status_file")" != ready ]] || stop_requested; then
                acknowledge_pre_exec_stop
                exec 3>&-
                rm -f -- "$token_pipe"
                exit 130
              fi
              (( handoff_waited += 1 ))
            done
            if (( ! handoff_received )); then
              exec 3>&-
              rm -f -- "$token_pipe"
              print -u2 -r -- "AI Token Meter: timed out waiting for the app handoff after \(loginTimeoutSeconds) seconds. Start a new one-time quota check."
              report_status handoff_timeout
              exit 124
            fi
            exec 3>&-
            rm -f -- "$token_pipe"
            if [[ ! "$completion_token" =~ '^[0-9A-Fa-f]{8}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{12}$' ]]; then
              print -u2 -r -- "AI Token Meter: the app handoff was invalid; this login session cannot continue."
              report_status invalid_handoff
              exit 127
            fi
            if [[ ! -r "$status_file" ]] || [[ "$(<"$status_file")" != ready ]] || stop_requested; then
              acknowledge_pre_exec_stop
              exit 130
            fi
            print -r -- "AI Token Meter: handoff received. Waiting for the Antigravity CLI process to start."
            callback_base="aitokenmeter://antigravity-login-complete?token=${completion_token}"
            open_command=\(shellQuote(completionOpenExecutableURL.path))
            agy_pid=""
            agy_child_reaped=0
            agy_wait_status=0
            agy_stop_requested=0
            watchdog_pid=""
            launch_critical=0
            deferred_app_cancel=0
            deferred_cancel_exit=0
            cleanup_uncertain=0
            guardian_state_preserved=0
            timeout_cleanup_confirmed=0
            timed_out=0
            terminal_shell_pgid=$(/usr/bin/perl -MPOSIX -e 'print POSIX::getpgrp()')
            terminal_foreground_transferred=0

            set_terminal_foreground() {
              /usr/bin/perl -MPOSIX -e 'my ($pgid) = @ARGV; open my $tty, "<", "/dev/tty" or exit 1; exit(POSIX::tcsetpgrp(fileno($tty), $pgid) == 0 ? 0 : 1);' "$1"
            }

            restore_terminal_foreground() {
              for _ in {1..3}; do
                if /usr/bin/perl -MPOSIX -e 'my ($pgid) = @ARGV; open my $tty, "<", "/dev/tty" or exit 1; local $SIG{TTOU} = "IGNORE"; exit(POSIX::tcsetpgrp(fileno($tty), $pgid) == 0 ? 0 : 1);' "$terminal_shell_pgid"; then
                  return 0
                fi
                /bin/sleep 0.1 || true
              done
              return 1
            }

            send_receipt() {
              (( !status_publication_failed && !status_publication_critical )) || return 1
              local result="$1"
              local exit_code="$2"
              if ! "$open_command" -g "${callback_base}&result=${result}&exit_code=${exit_code}"; then
                print -u2 -r -- "AI Token Meter: could not send the login result back to the app."
                report_status callback_failed
                return 1
              fi
              return 0
            }

            process_state() {
              /usr/bin/perl -e \(shellQuote(Self.boundedProcessStateProbeProgram)) "$process_state_executable" "$@" 2>/dev/null
            }

            owned_supervisor_state() {
              local observed_state query_status
              if observed_state="$(process_state --owned-query-completion -o stat= -p "$agy_pid")"; then
                print -r -- "$observed_state"
                return 0
              else
                query_status=$?
              fi
              # Only the complete, empty exit1 query may use kernel absence for our child.
              # Unknown query lifecycle, timeout and every other failure stay unsafe.
              [[ "$query_status" == 1 ]] || return 1
              /usr/bin/perl -MErrno=ESRCH -e 'my ($pid) = @ARGV; exit 1 unless $pid =~ /^[1-9][0-9]*$/; local $SIG{ALRM} = sub { exit 1 }; alarm 1; local $! = 0; my $matched = kill 0, $pid; my $errno = 0 + $!; exit 1 if $matched; exit($errno == ESRCH ? 0 : 1);' "$agy_pid"
            }

            guardian_stop_permission_valid() {
              /usr/bin/perl -e \(shellQuote(Self.guardianStopPermissionProbeProgram)) "$agy_pid" "$guardian_state_file" "$cleanup_ack_file"
            }

            owned_stop_confirmation_is_safe() {
              (( !status_publication_critical && !status_publication_failed && !cleanup_state_uncertain )) || return 1
              if (( agy_stop_requested || agy_wait_status >= 128 )); then guardian_stop_permission_valid; else return 0; fi
            }

            process_group_is_empty() {
              # ps exits nonzero for a nonexistent group. The kernel's ESRCH
              # is the positive absence proof; every other result stays unsafe.
              /usr/bin/perl -MErrno=ESRCH -e 'my ($pgid) = @ARGV; exit 1 unless $pgid =~ /^[1-9][0-9]*$/; my $matched = kill "-0", $pgid; exit 1 if $matched; exit($! == ESRCH ? 0 : 1);' "$agy_pid"
            }

            watchdog_is_running() {
              [[ -n "$watchdog_pid" ]] && kill -0 "$watchdog_pid" 2>/dev/null || return 1
              local watchdog_state
              watchdog_state="$(process_state -o stat= -p "$watchdog_pid")" || return 1
              [[ -n "$watchdog_state" && "$watchdog_state" != *Z* ]]
            }

            stop_watchdog() {
              [[ -n "$watchdog_pid" ]] || return 0
              # The watchdog is our unreaped direct child. Ignore a late timeout
              # signal once shutdown begins, then reap it with a bounded TERM/KILL.
              trap '' USR1
              if kill -0 "$watchdog_pid" 2>/dev/null; then
                kill -TERM "$watchdog_pid" 2>/dev/null || true
                for _ in {1..10}; do
                  if ! kill -0 "$watchdog_pid" 2>/dev/null; then break; fi
                  local watchdog_state
                  if watchdog_state="$(process_state -o stat= -p "$watchdog_pid")"; then
                    if [[ -z "$watchdog_state" || "$watchdog_state" == *Z* ]]; then break; fi
                  fi
                  /bin/sleep 0.1 || true
                done
                if kill -0 "$watchdog_pid" 2>/dev/null; then
                  kill -KILL "$watchdog_pid" 2>/dev/null || true
                fi
              fi
              wait "$watchdog_pid" 2>/dev/null || true
              watchdog_pid=""
            }

            stop_owned_processes() {
              cleanup_uncertain=0
              if (( status_publication_critical )); then guardian_state_preserved=1; return 1; fi
              if (( status_publication_failed )); then cleanup_state_uncertain=1; guardian_state_preserved=1; fi
              if [[ -n "$agy_pid" ]]; then
                if (( ! agy_child_reaped )); then
                  local supervisor_stopped=0
                  local identity_requested=0
                  for _ in {1..35}; do
                    local supervisor_state
                    if ! supervisor_state="$(owned_supervisor_state)"; then
                      cleanup_uncertain=1
                      break
                    fi
                    if [[ -z "$supervisor_state" || "$supervisor_state" == *Z* ]]; then supervisor_stopped=1; break; fi
                    if (( !identity_requested )) && [[ -r "$guardian_state_file" ]]; then
                      guardian_identity="$(<"$guardian_state_file")"
                      guardian_identity_pid="${guardian_identity%% *}"
                      if [[ "$guardian_identity_pid" == "$agy_pid" ]]; then
                        print -r -- "$guardian_identity" >| "$guardian_stop_file"
                        identity_requested=1
                        agy_stop_requested=1
                      else
                        report_status guardian_identity_mismatch
                        cleanup_uncertain=1
                        break
                      fi
                    fi
                    # .cancel and parent-loss are also observed by the actual
                    # owner. No identity means no arbitrary PID/PGID signal.
                    /bin/sleep 0.1 || true
                  done
                  if (( !supervisor_stopped )); then cleanup_uncertain=1; fi
                  if (( cleanup_uncertain )); then
                    guardian_state_preserved=1
                    report_status helper_stop_unconfirmed
                    print -u2 -r -- "AI Token Meter: Antigravity cleanup is unconfirmed; preserving guardian state."
                    return 1
                  fi
                  wait "$agy_pid" 2>/dev/null
                  agy_wait_status=$?
                  agy_child_reaped=1
                  if [[ -r "$status_file" && "$(<"$status_file")" == helper_stop_unconfirmed ]]; then
                    cleanup_uncertain=1
                  fi
                fi
                if ! owned_stop_confirmation_is_safe; then cleanup_uncertain=1; fi
                if ! process_group_is_empty; then
                  cleanup_uncertain=1
                fi
                if (( cleanup_uncertain )); then
                  guardian_state_preserved=1
                  report_status helper_stop_unconfirmed
                  print -u2 -r -- "AI Token Meter: the Antigravity process group may still be running; preserving guardian state."
                  return 1
                fi
              fi
              stop_watchdog
              if (( terminal_foreground_transferred )); then
                if restore_terminal_foreground; then
                  terminal_foreground_transferred=0
                else
                  print -u2 -r -- "AI Token Meter: could not restore the Terminal foreground process group."
                  report_status terminal_foreground_restore_failed
                  guardian_state_preserved=1
                  return 1
                fi
              fi
              if (( cleanup_state_uncertain )); then guardian_state_preserved=1; return 1; fi
              agy_pid=""
              agy_child_reaped=0
              watchdog_pid=""
              return 0
            }

            cleanup_watchdog() {
              if (( status_publication_critical )); then guardian_state_preserved=1; return 1; fi
              stop_watchdog
              if (( terminal_foreground_transferred )); then
                if restore_terminal_foreground; then
                  terminal_foreground_transferred=0
                else
                  print -u2 -r -- "AI Token Meter: could not restore the Terminal foreground process group during cleanup."
                  report_status terminal_foreground_restore_failed
                  guardian_state_preserved=1
                  return 1
                fi
              fi
              watchdog_pid=""
              rm -f -- "$watchdog_ready_file"
              if (( !guardian_state_preserved )); then
                rm -f -- "$process_group_file"
                rm -f -- "$guardian_state_file"
                rm -f -- "$guardian_stop_file"
              fi
              return 0
            }

            finalize_helper_exit() {
              trap '' HUP INT TERM USR1 USR2
              cleanup_watchdog || return 1
              trap - EXIT
              # Every successful owned-process shutdown clears agy_pid. An
              # unreaped/unknown guardian or failed foreground restore cannot ACK.
              (( !guardian_state_preserved && !terminal_foreground_transferred )) || return 1
              [[ -z "$agy_pid" && -z "$watchdog_pid" ]] || return 1
              publish_completed_failure || true
            }

            cancel_login() {
              (( !cancellation_in_progress )) || return
              cancellation_in_progress=1
              local exit_code="$1"
              deferred_app_cancel=0; deferred_cancel_exit=0
              trap '' HUP INT TERM USR1 USR2
              if stop_owned_processes && cleanup_watchdog; then
                trap - EXIT
                send_receipt cancelled "$exit_code"
              else
                cleanup_watchdog || true
                trap - EXIT
                report_status helper_stop_unconfirmed
              fi
              exit "$exit_code"
            }

            cancel_from_app() {
              (( !cancellation_in_progress )) || return
              cancellation_in_progress=1
              deferred_app_cancel=0; deferred_cancel_exit=0
              trap '' HUP INT TERM USR1 USR2
              if stop_owned_processes && cleanup_watchdog; then
                # All direct owners are reaped and EXIT cleanup is complete.
                # No supervisor or EXIT handler can publish a later response.
                trap - EXIT
                report_status helper_cancelled
              else
                cleanup_watchdog || true
                trap - EXIT
                report_status helper_stop_unconfirmed
              fi
              exit 130
            }

            request_terminal_cancel() {
              (( !cancellation_in_progress )) || return
              if (( launch_critical || status_publication_critical )); then
                if (( deferred_cancel_exit == 0 )); then deferred_cancel_exit="$1"; fi
                return
              fi
              if stop_requested; then cancel_from_app; else cancel_login "$1"; fi
            }

            request_app_cancel() {
              (( !cancellation_in_progress )) || return
              if (( launch_critical || status_publication_critical )); then deferred_app_cancel=1; return; fi
              cancel_from_app
            }

            finish_launch_claim() {
              launch_critical=0
              (( !cancellation_in_progress )) || return
              if (( deferred_app_cancel )) || stop_requested; then
                deferred_app_cancel=0
                cancel_from_app
              fi
              if (( deferred_cancel_exit )); then
                local exit_code="$deferred_cancel_exit"
                deferred_cancel_exit=0
                cancel_login "$exit_code"
              fi
            }

            mark_timeout() { timed_out=1; }

            trap 'request_terminal_cancel 129' HUP
            trap 'request_terminal_cancel 130' INT
            trap 'request_terminal_cancel 143' TERM
            trap 'mark_timeout' USR1
            trap 'request_app_cancel' USR2
            trap 'finalize_helper_exit' EXIT

            launch_critical=1
            \(shellQuote(watchdogExecutableURL.path)) -e 'use Fcntl qw(:DEFAULT); use Time::HiRes qw(time); my ($parent_pid, $seconds, $status_file, $cancel_request_file, $ready_file, $process_group_file, $launch_gate) = @ARGV; my $read_status = sub { my $request = ""; if (open my $request_handle, "<", $cancel_request_file) { $request = <$request_handle> // ""; close $request_handle; chomp $request; } return "cancelled" if $request eq "cancelled"; my $value = ""; if (open my $handle, "<", $status_file) { $value = <$handle> // ""; close $handle; chomp $value; } return $value; }; my $finish_deadline = sub { return if getppid() != $parent_pid; open my $gate, "+<", $launch_gate or do { my $state = $read_status->(); kill "USR2", $parent_pid if $state ne "process_state_unavailable" && $state ne "helper_stop_unconfirmed"; return; }; my $record_lock = pack("qqiss", 0, 0, 0, F_WRLCK, SEEK_SET); my $locked = 0; for (1..20) { if (defined(fcntl($gate, F_SETLK, $record_lock))) { $locked = 1; last; } my $state = $read_status->(); if ($state ne "ready") { close $gate; kill "USR2", $parent_pid if $state ne "process_state_unavailable" && $state ne "helper_stop_unconfirmed"; return; } select undef, undef, undef, 0.05; } if (!$locked) { close $gate; kill "USR1", $parent_pid if getppid() == $parent_pid; return; } my $state = $read_status->(); if ($state ne "ready") { kill "USR2", $parent_pid if $state ne "process_state_unavailable" && $state ne "helper_stop_unconfirmed"; } else { kill "USR1", $parent_pid if getppid() == $parent_pid; } close $gate; }; my $launch_deadline = time + 10; while (1) { exit 0 if getppid() != $parent_pid; my $state = $read_status->(); if ($state ne "ready") { kill "USR2", $parent_pid if $state ne "process_state_unavailable" && $state ne "helper_stop_unconfirmed"; exit 0; } if (!-e $ready_file) { open my $ready_handle, ">", $ready_file or die "watchdog ready file unavailable"; close $ready_handle; } last if -e $process_group_file; if (time >= $launch_deadline) { $finish_deadline->(); exit 0; } select undef, undef, undef, 0.1; } my $deadline = time + $seconds; while (time < $deadline) { exit 0 if getppid() != $parent_pid; my $state = $read_status->(); if ($state ne "ready") { kill "USR2", $parent_pid if $state ne "process_state_unavailable" && $state ne "helper_stop_unconfirmed"; exit 0; } select undef, undef, undef, 0.1; } $finish_deadline->();' "$$" \(loginTimeoutSeconds) "$status_file" "$cancel_request_file" "$watchdog_ready_file" "$process_group_file" "$launch_gate" &
            watchdog_pid=$!
            finish_launch_claim
            watchdog_ready=0
            for _ in {1..50}; do
              if [[ -f "$watchdog_ready_file" ]] && watchdog_is_running; then
                watchdog_ready=1
                break
              fi
              if ! kill -0 "$watchdog_pid" 2>/dev/null; then break; fi
              /bin/sleep 0.1 || true
            done
            if (( ! watchdog_ready )); then
              print -u2 -r -- "AI Token Meter: could not start the Antigravity login watchdog; the CLI was not started."
              report_status watchdog_start_failed
              stop_owned_processes
              exit 127
            fi

            if [[ ! -r "$status_file" ]] || [[ "$(<"$status_file")" != ready ]] || stop_requested; then
              if stop_requested; then cancel_from_app; fi
              stop_owned_processes || true
              exit 130
            fi

            exec_status_file=\(shellQuote(completionExecStatusPath))
            print -r -- "pending" >| "$exec_status_file"
            chmod 600 "$exec_status_file"
            launch_critical=1
            /usr/bin/perl -MFcntl=:DEFAULT -MPOSIX -MTime::HiRes=time,sleep -e \(shellQuote(supervisorProgram)) "$launch_gate" "$status_file" "$cancel_request_file" "$exec_status_file" "$process_group_file" "$guardian_state_file" "$guardian_stop_file" "$process_state_executable" "$$" \(shellQuote(executableURL.path)) </dev/tty >/dev/tty 2>/dev/tty &
            agy_pid=$!
            finish_launch_claim

            agy_exit_code=0
            while (( ! timed_out )); do
              if (( !terminal_foreground_transferred )) && [[ -r "$process_group_file" ]]; then
                process_group_id="$(<"$process_group_file")"
                terminal_foreground_transferred=1
                if [[ "$process_group_id" != "$agy_pid" ]] || ! set_terminal_foreground "$process_group_id"; then
                  print -u2 -r -- "AI Token Meter: could not give the Antigravity CLI the Terminal foreground."
                  report_status terminal_foreground_failed
                  stop_owned_processes
                  exit 127
                fi
                terminal_foreground_transferred=1
                for _ in {1..50}; do
                  exec_status="$(<"$exec_status_file")"
                  [[ "$exec_status" == exec_confirmed || "$exec_status" == exec_failed || "$exec_status" == launch_indeterminate ]] && break
                  /bin/sleep 0.02 || true
                done
                if [[ "$exec_status" == exec_confirmed ]]; then
                  print -r -- "AI Token Meter: Antigravity CLI process started; sign-in may still be in progress."
                elif [[ "$exec_status" == exec_failed ]]; then
                  print -u2 -r -- "AI Token Meter: the Antigravity CLI could not be started."
                elif [[ "$exec_status" == launch_indeterminate ]]; then
                  print -u2 -r -- "AI Token Meter: Antigravity CLI startup could not be confirmed."
                fi
              fi
              if [[ -r "$status_file" && ( "$(<"$status_file")" == process_state_unavailable || "$(<"$status_file")" == helper_stop_unconfirmed ) ]]; then
                guardian_state_preserved=1
                if [[ "$(<"$status_file")" == helper_stop_unconfirmed ]]; then
                  # Give our unreaped guardian its bounded owned cleanup before
                  # exiting can orphan a still-stopped pre-exec child.
                  stop_owned_processes || true
                fi
                print -u2 -r -- "AI Token Meter: process cleanup could not be confirmed; preserving guardian state."
                exit 127
              fi
              if ! agy_state="$(owned_supervisor_state)"; then
                print -u2 -r -- "AI Token Meter: could not confirm the Antigravity process state."
                if stop_owned_processes; then
                  report_status process_state_unavailable
                  send_receipt failure 127
                else
                  report_status helper_stop_unconfirmed
                fi
                exit 127
              fi
              if [[ -z "$agy_state" || "$agy_state" == *Z* ]]; then
                wait "$agy_pid"
                agy_exit_code=$?
                agy_wait_status=$agy_exit_code
                agy_child_reaped=1
                if stop_requested || [[ -r "$exec_status_file" && "$(<"$exec_status_file")" == pre_exec_cancelled ]]; then cancel_from_app; fi
                if [[ -r "$exec_status_file" && "$(<"$exec_status_file")" == launch_gate_timeout ]]; then timed_out=1; fi
                if ! owned_stop_confirmation_is_safe; then
                  guardian_state_preserved=1
                  report_status helper_stop_unconfirmed
                  exit 127
                fi
                if [[ -r "$status_file" && "$(<"$status_file")" == helper_stop_unconfirmed ]] || ! process_group_is_empty; then
                  guardian_state_preserved=1
                  report_status helper_stop_unconfirmed
                  print -u2 -r -- "AI Token Meter: process cleanup could not be confirmed; preserving guardian state."
                  exit 127
                fi
                break
              fi
              /bin/sleep 0.1 || true
            done
            # The supervisor remains the live process-group guardian until the
            # CLI and every same-group descendant have exited.
            if (( timed_out )); then
              # USR1 may interrupt wait before the watchdog's delayed KILL. Finish
              # process-group cleanup before stopping the watchdog.
              if stop_owned_processes; then
                timeout_cleanup_confirmed=1
              else
                timeout_cleanup_confirmed=0
              fi
            else
              if (( terminal_foreground_transferred )); then
                if restore_terminal_foreground; then
                  terminal_foreground_transferred=0
                else
                  print -u2 -r -- "AI Token Meter: could not restore the Terminal foreground process group."
                  report_status terminal_foreground_restore_failed
                  send_receipt failure 127
                  exit 127
                fi
              fi
              agy_pid=""
            fi
            stop_watchdog

            if (( timed_out )); then
              if (( timeout_cleanup_confirmed )); then
                print -u2 -r -- "AI Token Meter: Antigravity sign-in timed out and its login process was stopped."
                send_receipt timeout 124
              else
                print -u2 -r -- "AI Token Meter: Antigravity sign-in timed out; process cleanup is unconfirmed."
                report_status helper_stop_unconfirmed
              fi
              exit 124
            elif (( agy_exit_code == 0 )); then
              print -r -- "AI Token Meter: Antigravity CLI sign-in finished. Sending the result to the app."
              send_receipt success 0
              exit 0
            else
              print -u2 -r -- "AI Token Meter: Antigravity CLI sign-in failed with exit code $agy_exit_code."
              send_receipt failure "$agy_exit_code"
              exit "$agy_exit_code"
            fi
            """
        case .deepSeek, .code0:
            throw CLIAuthenticationScriptError.unsupportedProvider
        }

        let startupCleanup = provider == .gemini ? "rm -f -- \"$0\"" : ":"
        return """
        #!/bin/zsh
        set -u
        unsetopt BG_NICE
        \(startupCleanup)
        export PATH=\(shellQuote(executableURL.deletingLastPathComponent().path)):"${PATH:-/usr/bin:/bin:/usr/sbin:/sbin}"
        \(command)

        """
    }

    private func shellQuote(_ value: String) -> String {
        "'\(value.replacingOccurrences(of: "'", with: "'\\''"))'"
    }
}
