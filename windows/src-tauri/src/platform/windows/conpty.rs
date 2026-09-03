#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConPtySize {
    pub columns: i16,
    pub rows: i16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConPtyError {
    UnsupportedPlatform,
    InvalidSize,
    CreateFailed,
    ResizeFailed,
    SpawnFailed,
    ProcessControlFailed,
    WriteFailed,
    ReadFailed,
    TimedOut,
    OutputLimitExceeded,
}

pub struct ConPty {
    #[cfg(windows)]
    handle: windows_sys::Win32::System::Console::HPCON,
    #[cfg(windows)]
    input_write: windows_sys::Win32::Foundation::HANDLE,
    #[cfg(windows)]
    output_read: windows_sys::Win32::Foundation::HANDLE,
}

#[cfg(any(windows, test))]
const CURSOR_POSITION_QUERY: &[u8] = b"\x1b[6n";
#[cfg(windows)]
const DEFAULT_CURSOR_POSITION: &[u8] = b"\x1b[1;1R";

impl ConPty {
    pub fn open(size: ConPtySize) -> Result<Self, ConPtyError> {
        validate_size(size)?;
        #[cfg(windows)]
        {
            open_windows(size)
        }
        #[cfg(not(windows))]
        {
            Err(ConPtyError::UnsupportedPlatform)
        }
    }

    pub fn resize(&mut self, size: ConPtySize) -> Result<(), ConPtyError> {
        validate_size(size)?;
        #[cfg(windows)]
        {
            use windows_sys::Win32::System::Console::{COORD, ResizePseudoConsole};
            let result = unsafe {
                ResizePseudoConsole(
                    self.handle,
                    COORD {
                        X: size.columns,
                        Y: size.rows,
                    },
                )
            };
            if result < 0 {
                return Err(ConPtyError::ResizeFailed);
            }
            Ok(())
        }
        #[cfg(not(windows))]
        {
            Err(ConPtyError::UnsupportedPlatform)
        }
    }

    pub fn spawn(
        &mut self,
        command: &super::process::CommandInvocation,
        working_directory: Option<&std::path::Path>,
        environment: &[(std::ffi::OsString, std::ffi::OsString)],
    ) -> Result<ConPtyChild, ConPtyError> {
        #[cfg(windows)]
        {
            self.spawn_windows(command, working_directory, environment)
        }
        #[cfg(not(windows))]
        {
            let _ = (command, working_directory, environment);
            Err(ConPtyError::UnsupportedPlatform)
        }
    }

    pub fn send_fixed_input(&mut self, input: &[u8]) -> Result<(), ConPtyError> {
        if input.len() > 4096 {
            return Err(ConPtyError::OutputLimitExceeded);
        }
        #[cfg(windows)]
        {
            self.write_all_windows(input)
        }
        #[cfg(not(windows))]
        {
            Err(ConPtyError::UnsupportedPlatform)
        }
    }

    pub fn read_until(
        &mut self,
        patterns: &[&str],
        timeout: std::time::Duration,
        max_output_bytes: usize,
    ) -> Result<String, ConPtyError> {
        #[cfg(windows)]
        {
            self.read_until_windows(patterns, timeout, max_output_bytes)
        }
        #[cfg(not(windows))]
        {
            let _ = (patterns, timeout, max_output_bytes);
            Err(ConPtyError::UnsupportedPlatform)
        }
    }
}

pub struct ConPtyChild {
    #[cfg(windows)]
    process: windows_sys::Win32::Foundation::HANDLE,
    #[cfg(windows)]
    job: windows_sys::Win32::Foundation::HANDLE,
}

impl ConPtyChild {
    pub fn wait(&mut self, timeout: std::time::Duration) -> Result<u32, ConPtyError> {
        #[cfg(windows)]
        {
            use windows_sys::Win32::Foundation::{WAIT_OBJECT_0, WAIT_TIMEOUT};
            use windows_sys::Win32::System::JobObjects::TerminateJobObject;
            use windows_sys::Win32::System::Threading::{GetExitCodeProcess, WaitForSingleObject};

            let milliseconds = timeout.as_millis().min(u128::from(u32::MAX - 1)) as u32;
            match unsafe { WaitForSingleObject(self.process, milliseconds) } {
                WAIT_OBJECT_0 => {
                    let mut exit_code = 0;
                    if unsafe { GetExitCodeProcess(self.process, &mut exit_code) } == 0 {
                        return Err(ConPtyError::ProcessControlFailed);
                    }
                    Ok(exit_code)
                }
                WAIT_TIMEOUT => {
                    unsafe { TerminateJobObject(self.job, 1) };
                    Err(ConPtyError::TimedOut)
                }
                _ => Err(ConPtyError::ProcessControlFailed),
            }
        }
        #[cfg(not(windows))]
        {
            let _ = timeout;
            Err(ConPtyError::UnsupportedPlatform)
        }
    }
}

fn validate_size(size: ConPtySize) -> Result<(), ConPtyError> {
    (size.columns > 0 && size.rows > 0)
        .then_some(())
        .ok_or(ConPtyError::InvalidSize)
}

#[cfg(windows)]
impl ConPty {
    fn write_all_windows(&mut self, input: &[u8]) -> Result<(), ConPtyError> {
        use windows_sys::Win32::Storage::FileSystem::WriteFile;

        let mut offset = 0;
        while offset < input.len() {
            let mut written = 0;
            let succeeded = unsafe {
                WriteFile(
                    self.input_write,
                    input[offset..].as_ptr().cast(),
                    (input.len() - offset) as u32,
                    &mut written,
                    std::ptr::null_mut(),
                )
            };
            if succeeded == 0 || written == 0 {
                return Err(ConPtyError::WriteFailed);
            }
            offset += written as usize;
        }
        Ok(())
    }

    fn spawn_windows(
        &mut self,
        command: &super::process::CommandInvocation,
        working_directory: Option<&std::path::Path>,
        environment: &[(std::ffi::OsString, std::ffi::OsString)],
    ) -> Result<ConPtyChild, ConPtyError> {
        use std::mem::{size_of, zeroed};
        use std::os::windows::ffi::OsStrExt;
        use std::ptr;
        use windows_sys::Win32::Foundation::CloseHandle;
        use windows_sys::Win32::System::JobObjects::{
            AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
            JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
            SetInformationJobObject, TerminateJobObject,
        };
        use windows_sys::Win32::System::Threading::{
            CREATE_SUSPENDED, CREATE_UNICODE_ENVIRONMENT, CreateProcessW,
            DeleteProcThreadAttributeList, EXTENDED_STARTUPINFO_PRESENT,
            InitializeProcThreadAttributeList, PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE,
            PROCESS_INFORMATION, ResumeThread, STARTUPINFOEXW, UpdateProcThreadAttribute,
        };

        let executable = command
            .executable
            .canonicalize()
            .map_err(|_| ConPtyError::SpawnFailed)?;
        if !executable
            .metadata()
            .is_ok_and(|metadata| metadata.is_file())
        {
            return Err(ConPtyError::SpawnFailed);
        }
        let mut application = executable.as_os_str().encode_wide().collect::<Vec<_>>();
        application.push(0);
        let mut command_line = windows_command_line(&executable, &command.arguments);
        let current_directory = working_directory.map(|directory| {
            let mut wide = directory.as_os_str().encode_wide().collect::<Vec<_>>();
            wide.push(0);
            wide
        });
        let environment_block = windows_environment_block(environment);

        let job = unsafe { CreateJobObjectW(ptr::null(), ptr::null()) };
        if job.is_null() {
            return Err(ConPtyError::ProcessControlFailed);
        }
        let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { zeroed() };
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        if unsafe {
            SetInformationJobObject(
                job,
                JobObjectExtendedLimitInformation,
                (&raw const limits).cast(),
                size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        } == 0
        {
            unsafe { CloseHandle(job) };
            return Err(ConPtyError::ProcessControlFailed);
        }

        let mut attribute_bytes = 0;
        unsafe { InitializeProcThreadAttributeList(ptr::null_mut(), 1, 0, &mut attribute_bytes) };
        if attribute_bytes == 0 {
            unsafe { CloseHandle(job) };
            return Err(ConPtyError::SpawnFailed);
        }
        let attribute_words = attribute_bytes.div_ceil(size_of::<usize>());
        let mut attribute_storage = vec![0_usize; attribute_words];
        let attribute_list = attribute_storage.as_mut_ptr().cast();
        if unsafe { InitializeProcThreadAttributeList(attribute_list, 1, 0, &mut attribute_bytes) }
            == 0
        {
            unsafe { CloseHandle(job) };
            return Err(ConPtyError::SpawnFailed);
        }
        let attribute_updated = unsafe {
            UpdateProcThreadAttribute(
                attribute_list,
                0,
                PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE as usize,
                self.handle as *const _,
                size_of::<windows_sys::Win32::System::Console::HPCON>(),
                ptr::null_mut(),
                ptr::null(),
            )
        };
        if attribute_updated == 0 {
            unsafe {
                DeleteProcThreadAttributeList(attribute_list);
                CloseHandle(job);
            }
            return Err(ConPtyError::SpawnFailed);
        }

        let mut startup: STARTUPINFOEXW = unsafe { zeroed() };
        startup.StartupInfo.cb = size_of::<STARTUPINFOEXW>() as u32;
        startup.lpAttributeList = attribute_list;
        let mut process: PROCESS_INFORMATION = unsafe { zeroed() };
        let created = unsafe {
            CreateProcessW(
                application.as_ptr(),
                command_line.as_mut_ptr(),
                ptr::null(),
                ptr::null(),
                0,
                EXTENDED_STARTUPINFO_PRESENT | CREATE_UNICODE_ENVIRONMENT | CREATE_SUSPENDED,
                environment_block.as_ptr().cast(),
                current_directory
                    .as_ref()
                    .map_or(ptr::null(), |wide| wide.as_ptr()),
                &startup.StartupInfo,
                &mut process,
            )
        };
        unsafe { DeleteProcThreadAttributeList(attribute_list) };
        if created == 0 {
            unsafe { CloseHandle(job) };
            return Err(ConPtyError::SpawnFailed);
        }
        if unsafe { AssignProcessToJobObject(job, process.hProcess) } == 0 {
            unsafe {
                TerminateJobObject(job, 1);
                CloseHandle(process.hThread);
                CloseHandle(process.hProcess);
                CloseHandle(job);
            }
            return Err(ConPtyError::ProcessControlFailed);
        }
        if unsafe { ResumeThread(process.hThread) } == u32::MAX {
            unsafe {
                TerminateJobObject(job, 1);
                CloseHandle(process.hThread);
                CloseHandle(process.hProcess);
                CloseHandle(job);
            }
            return Err(ConPtyError::ProcessControlFailed);
        }
        unsafe { CloseHandle(process.hThread) };
        Ok(ConPtyChild {
            process: process.hProcess,
            job,
        })
    }

    fn read_until_windows(
        &mut self,
        patterns: &[&str],
        timeout: std::time::Duration,
        max_output_bytes: usize,
    ) -> Result<String, ConPtyError> {
        use std::thread;
        use std::time::{Duration, Instant};
        use windows_sys::Win32::Storage::FileSystem::ReadFile;
        use windows_sys::Win32::System::Pipes::PeekNamedPipe;

        let deadline = Instant::now() + timeout;
        let mut output = Vec::new();
        let mut answered_cursor_queries = 0;
        loop {
            if Instant::now() >= deadline {
                return Err(ConPtyError::TimedOut);
            }
            let mut available = 0;
            if unsafe {
                PeekNamedPipe(
                    self.output_read,
                    std::ptr::null_mut(),
                    0,
                    std::ptr::null_mut(),
                    &mut available,
                    std::ptr::null_mut(),
                )
            } == 0
            {
                return Err(ConPtyError::ReadFailed);
            }
            if available == 0 {
                thread::sleep(Duration::from_millis(5));
                continue;
            }
            let read_size = usize::try_from(available).unwrap_or(4096).min(4096);
            if read_size > max_output_bytes.saturating_sub(output.len()) {
                return Err(ConPtyError::OutputLimitExceeded);
            }
            let mut chunk = vec![0_u8; read_size];
            let mut bytes_read = 0;
            if unsafe {
                ReadFile(
                    self.output_read,
                    chunk.as_mut_ptr().cast(),
                    chunk.len() as u32,
                    &mut bytes_read,
                    std::ptr::null_mut(),
                )
            } == 0
            {
                return Err(ConPtyError::ReadFailed);
            }
            output.extend_from_slice(&chunk[..bytes_read as usize]);
            let cursor_queries = count_subsequence(&output, CURSOR_POSITION_QUERY);
            while answered_cursor_queries < cursor_queries {
                self.write_all_windows(DEFAULT_CURSOR_POSITION)?;
                answered_cursor_queries += 1;
            }
            let normalized = super::process::normalize_output(&output);
            if patterns.iter().any(|pattern| normalized.contains(pattern)) {
                return Ok(normalized);
            }
        }
    }
}

#[cfg(any(windows, test))]
fn count_subsequence(haystack: &[u8], needle: &[u8]) -> usize {
    if needle.is_empty() {
        return 0;
    }
    haystack
        .windows(needle.len())
        .filter(|window| *window == needle)
        .count()
}

#[cfg(test)]
mod tests {
    use super::{CURSOR_POSITION_QUERY, count_subsequence};

    #[test]
    fn cursor_queries_are_counted_across_surrounding_terminal_output() {
        let output = b"before\x1b[6nafter\x1b[6n";
        assert_eq!(count_subsequence(output, CURSOR_POSITION_QUERY), 2);
    }

    #[test]
    fn partial_cursor_query_is_not_treated_as_complete() {
        assert_eq!(count_subsequence(b"\x1b[6", CURSOR_POSITION_QUERY), 0);
    }
}

#[cfg(windows)]
fn windows_command_line(
    executable: &std::path::Path,
    arguments: &[std::ffi::OsString],
) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;

    let mut command_line = Vec::new();
    for argument in
        std::iter::once(executable.as_os_str()).chain(arguments.iter().map(AsRef::as_ref))
    {
        if !command_line.is_empty() {
            command_line.push(' ' as u16);
        }
        append_quoted_windows_argument(
            &mut command_line,
            &argument.encode_wide().collect::<Vec<_>>(),
        );
    }
    command_line.push(0);
    command_line
}

#[cfg(windows)]
fn append_quoted_windows_argument(output: &mut Vec<u16>, argument: &[u16]) {
    let needs_quotes = argument.is_empty()
        || argument
            .iter()
            .any(|character| matches!(*character, 0x09 | 0x20 | 0x22));
    if !needs_quotes {
        output.extend_from_slice(argument);
        return;
    }
    output.push('"' as u16);
    let mut backslashes = 0;
    for character in argument {
        if *character == '\\' as u16 {
            backslashes += 1;
        } else if *character == '"' as u16 {
            output.extend(std::iter::repeat_n('\\' as u16, backslashes * 2 + 1));
            output.push(*character);
            backslashes = 0;
        } else {
            output.extend(std::iter::repeat_n('\\' as u16, backslashes));
            output.push(*character);
            backslashes = 0;
        }
    }
    output.extend(std::iter::repeat_n('\\' as u16, backslashes * 2));
    output.push('"' as u16);
}

#[cfg(windows)]
fn windows_environment_block(supplied: &[(std::ffi::OsString, std::ffi::OsString)]) -> Vec<u16> {
    use std::collections::BTreeMap;
    use std::os::windows::ffi::OsStrExt;

    let mut values = BTreeMap::new();
    for key in ["SystemRoot", "WINDIR", "TEMP", "TMP"] {
        if let Some(value) = std::env::var_os(key) {
            values.insert(
                key.to_ascii_uppercase(),
                (std::ffi::OsString::from(key), value),
            );
        }
    }
    for (key, value) in supplied {
        let normalized = key.to_string_lossy().to_ascii_uppercase();
        if !normalized.contains(['=', '\0']) && !value.to_string_lossy().contains('\0') {
            values.insert(normalized, (key.clone(), value.clone()));
        }
    }
    let mut block = Vec::new();
    for (_, (key, value)) in values {
        block.extend(key.encode_wide());
        block.push('=' as u16);
        block.extend(value.encode_wide());
        block.push(0);
    }
    block.push(0);
    if block.len() == 1 {
        block.push(0);
    }
    block
}

#[cfg(windows)]
fn open_windows(size: ConPtySize) -> Result<ConPty, ConPtyError> {
    use std::ptr;
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
    use windows_sys::Win32::System::Console::{COORD, CreatePseudoConsole, HPCON};
    use windows_sys::Win32::System::Pipes::CreatePipe;

    let mut input_read: HANDLE = ptr::null_mut();
    let mut input_write: HANDLE = ptr::null_mut();
    let mut output_read: HANDLE = ptr::null_mut();
    let mut output_write: HANDLE = ptr::null_mut();
    if unsafe { CreatePipe(&mut input_read, &mut input_write, ptr::null(), 0) } == 0 {
        return Err(ConPtyError::CreateFailed);
    }
    if unsafe { CreatePipe(&mut output_read, &mut output_write, ptr::null(), 0) } == 0 {
        unsafe {
            CloseHandle(input_read);
            CloseHandle(input_write);
        }
        return Err(ConPtyError::CreateFailed);
    }
    let mut handle: HPCON = 0;
    let result = unsafe {
        CreatePseudoConsole(
            COORD {
                X: size.columns,
                Y: size.rows,
            },
            input_read,
            output_write,
            0,
            &mut handle,
        )
    };
    unsafe {
        CloseHandle(input_read);
        CloseHandle(output_write);
    }
    if result < 0 {
        unsafe {
            CloseHandle(input_write);
            CloseHandle(output_read);
        }
        return Err(ConPtyError::CreateFailed);
    }
    Ok(ConPty {
        handle,
        input_write,
        output_read,
    })
}

#[cfg(windows)]
impl Drop for ConPty {
    fn drop(&mut self) {
        use windows_sys::Win32::Foundation::CloseHandle;
        use windows_sys::Win32::System::Console::ClosePseudoConsole;

        unsafe {
            ClosePseudoConsole(self.handle);
            CloseHandle(self.input_write);
            CloseHandle(self.output_read);
        }
    }
}

#[cfg(windows)]
impl Drop for ConPtyChild {
    fn drop(&mut self) {
        use windows_sys::Win32::Foundation::CloseHandle;
        use windows_sys::Win32::System::JobObjects::TerminateJobObject;

        unsafe {
            TerminateJobObject(self.job, 1);
            CloseHandle(self.process);
            CloseHandle(self.job);
        }
    }
}
