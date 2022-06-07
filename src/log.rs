use std::fs::File;
use std::io::BufWriter;
use std::io::Write;
use std::sync::Mutex;
use std::sync::Once;
use std::cell::Cell;
use std::hint::unreachable_unchecked;
use std::path::{Path, PathBuf};

#[derive(thiserror::Error, Debug)]
pub enum LogHandleError {
    #[error("IO error")]
    Io(#[from] std::io::Error),
}

pub struct StaticLogger {
    pub a: Box<Logger>
}

#[derive(Debug)]
pub struct Logger {
    severity: Mutex<Severity>,
    log_path: Mutex<Option<PathBuf>>, // where to write the log file
    log_writer: Mutex<Option<BufWriter<File>>>, // internal cache for file writer, optional
}

/// Get a static reference to the logger. Lazy evaluated at runtime.
#[allow(non_snake_case)]
pub fn LOGGER() -> &'static StaticLogger {
    // Store the data, along with a lock guard to make sure static is set only once
    struct Stt {
        data: Cell<Option<StaticLogger>>,
        once: Once
    }

    // Static variable types must have Sync traits bound, force access to Stt to be thread safe
    unsafe impl Sync for Stt {}

    static SYNCHRONIZED_STT: Stt = Stt{
        data: Cell::new(None),
        once: Once::new()
    };

    SYNCHRONIZED_STT.once.call_once(|| {
        // Init static with a state at runtime (heap)
        SYNCHRONIZED_STT.data.set(Some(StaticLogger{ a: Box::new(Logger::new()) }));
    });

    // Get reference (deref to raw pointer)
    let v = unsafe { match *SYNCHRONIZED_STT.data.as_ptr() {
        Some(ref a) => a,
        None => unreachable_unchecked()
    }};

    return v;
}

impl Logger {
    pub fn new() -> Logger {
        // This never needs to be mutable since it's handled by mutex
        Logger {
            severity: Mutex::new(Severity::Debug),
            log_path: Mutex::new(None),
            log_writer: Mutex::new(None),
        }
    }

    /// Log to both stdout and file.
    fn log_message(&self, severity: Severity, message: &str) {
        let mut msg = LogMessage::new(&("").to_string(), message, severity);
        print!("{}", msg.formatted(true));
        self.log_message_to_file(&mut msg);
    }

    fn log_message_to_file(&self, log_message: &mut LogMessage) {
        self.set_log_writer_if_not_set();
        if let Ok(ref mut writer) = self.log_writer.lock() {
            if writer.is_some() {
                let formatted_message = log_message.formatted(false);
                if let Err(e) = writer.as_mut().unwrap().write(formatted_message.as_bytes()) {
                    self.remove_log_writer();
                    self.remove_log_path();
                    self.error(&format!("log file could not be written to: {e:?}"));
                }
            }
        }
    }

    fn set_log_writer_if_not_set(&self) {
        if !self.has_log_writer() {
            if let Some(path) = self.get_log_path() {
                let file = match self.open_log_file(&path, LogFileWriteType::Overwrite) {
                    Ok(f) => f,
                    Err(e) => {
                        print!("could not open log file: {:?}", e);
                        self.remove_log_path();
                        return;
                    }
                };

                let buf_writer = BufWriter::new(file);
                self.set_log_writer(buf_writer);
            }
        }
    }

    pub fn open_log_file<P: AsRef<Path>>(&self, path: P, mode: LogFileWriteType) -> Result<File, LogHandleError> {
        match mode {
            LogFileWriteType::Append => {
                if path.as_ref().exists() {
                    match File::options().append(true).open(path) {
                        Ok(file) => Ok(file),
                        Err(e) => Err(LogHandleError::Io(e))
                    }
                } else {
                    match File::create(path) {
                        Ok(file) => Ok(file),
                        Err(e) => Err(LogHandleError::Io(e))
                    }
                }
            },
            LogFileWriteType::Overwrite => {
                match File::create(path) {
                    Ok(file) => Ok(file),
                    Err(e) => Err(LogHandleError::Io(e))
                }
            }
        }
    }

    fn has_log_writer(&self) -> bool {
        if let Ok(lw) = self.log_writer.lock() {
            return lw.is_some();
        }

        false
    }

    fn set_log_writer(&self, buf_writer: BufWriter<File>) {
        *self.log_writer.lock().unwrap() = Some(buf_writer);
    }

    fn remove_log_writer(&self) {
        *self.log_writer.lock().unwrap() = None;
    }

    pub fn set_log_path(&self, path: &str) -> Result<(), String> {
        let path_buf = PathBuf::from(path);
        self.remove_log_writer();

        // Create file if it doesn't exist
        if !path_buf.exists() && File::create(path).is_err() {
            return Err(("log file path specified does not exist!").to_owned());
        }

        if !path_buf.is_file() {
            return Err(("log file path specified is not a file!").to_owned())
        }

        *self.log_path.lock().unwrap() = Some(path_buf);

        Ok(())
    }

    pub fn get_log_path(&self) -> Option<PathBuf> {
        (*self.log_path.lock().unwrap()).as_ref().cloned()
    }

    pub fn remove_log_path(&self) {
        *self.log_path.lock().unwrap() = None;
        self.remove_log_writer();
    }

    pub fn set_severity(&self, severity: Severity) {
        *self.severity.lock().unwrap() = severity;
    }

    pub fn get_severity(&self) -> Severity {
        *self.severity.lock().unwrap()
    }

    pub fn debug(&self, message: &str) {
        if self.get_severity() <= Severity::Debug {
            self.log_message(Severity::Debug, message);
        }
    }

    pub fn info(&self, message: &str) {
        if self.get_severity() <= Severity::Info {
            self.log_message(Severity::Info, message);
        }
    }

    pub fn warn(&self, message: &str) {
        if self.get_severity() <= Severity::Warn {
            self.log_message(Severity::Warn, message);
        }
    }

    pub fn error(&self, message: &str) {
        if self.get_severity() <= Severity::Error {
            self.log_message(Severity::Error, message);
        }
    }

    pub fn fatal(&self, message: &str) {
        if self.get_severity() <= Severity::Fatal {
            self.log_message(Severity::Fatal, message);
        }
    }

    /// Clear I/O buffers before shutdown, needed for log files.
    pub fn flush(&self) -> std::io::Result<()> {
        if let Ok(ref mut writer) = self.log_writer.lock() {
            if writer.is_some() {
                writer.as_mut().unwrap().flush()
            } else {
                Ok(())
            }
        } else {
            Ok(())
        }
    }
}

pub struct LogMessage {
    colorized: Option<String>,
    non_colorized: Option<String>,
    prefix: String,
    severity_string: String,
    severity_color: ANSIColor,
    message: String
}

impl LogMessage {
    pub fn new(prefix: &str, message: &str, severity: Severity) -> LogMessage {
        LogMessage {
            colorized: None,
            non_colorized: None,
            prefix: prefix.to_string(),
            severity_string: format!("[{}]", severity),
            severity_color: severity.get_color(),
            message: message.to_string()
        }
    }

    pub fn formatted(&mut self, colorize: bool) -> String {
        if colorize {
            self.colorized()
        } else {
            self.non_colorized()
        }
    }

    fn colorized(&mut self) -> String {
        match self.colorized {
            Some(ref s) => s.clone(),
            None => {
                let severity_string = self.severity_color.colorize(&self.severity_string);
                self.colorized = Some(format!(
                    "{}{} {}\n",
                    self.prefix, severity_string, self.message
                ));

                self.colorized.clone().unwrap()
            }
        }
    }

    fn non_colorized(&mut self) -> String {
        match self.non_colorized {
            Some(ref s) => s.clone(),
            None => {
                self.non_colorized = Some(format!(
                    "{}{} {}\n",
                    self.prefix, self.severity_string, self.message
                ));

                self.non_colorized.clone().unwrap()
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum Severity {
    Debug = 0,
    Info,
    Warn,
    Error,
    Fatal,
    None
}

impl Severity {
    pub fn get_color(&self) -> ANSIColor {
        match self {
            Severity::Debug => ANSIColor::Cyan,
            Severity::Info =>  ANSIColor::Green,
            Severity::Warn =>  ANSIColor::Yellow,
            Severity::Error => ANSIColor::BrightRed,
            Severity::Fatal => ANSIColor::Red,
            Severity::None =>  ANSIColor::Reset
        }
    }
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Severity::Debug => write!(f, "DEBUG"),
            Severity::Info => write!(f, "INFO"),
            Severity::Warn => write!(f, "WARN"),
            Severity::Error => write!(f, "ERROR"),
            Severity::Fatal => write!(f, "FATAL"),
            Severity::None => write!(f, "NONE"),
        }
    }
}

pub enum LogFileWriteType {
    Append,
    Overwrite
}

#[allow(dead_code)]
pub enum ANSIColor {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    BrightBlack,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,
    Reset
}

impl ANSIColor {
    pub fn starter_sequence(&self) -> &str {
        match self {
            ANSIColor::Black =>         "\x1b[30m",
            ANSIColor::Red =>           "\x1b[31m",
            ANSIColor::Green =>         "\x1b[32m",
            ANSIColor::Yellow =>        "\x1b[33m",
            ANSIColor::Blue =>          "\x1b[34m",
            ANSIColor::Magenta =>       "\x1b[35m",
            ANSIColor::Cyan =>          "\x1b[36m",
            ANSIColor::White =>         "\x1b[37m",
            ANSIColor::BrightBlack =>   "\x1b[90m",
            ANSIColor::BrightRed =>     "\x1b[91m",
            ANSIColor::BrightGreen =>   "\x1b[92m",
            ANSIColor::BrightYellow =>  "\x1b[93m",
            ANSIColor::BrightBlue =>    "\x1b[94m",
            ANSIColor::BrightMagenta => "\x1b[95m",
            ANSIColor::BrightCyan =>    "\x1b[96m",
            ANSIColor::BrightWhite =>   "\x1b[97m",
            ANSIColor::Reset =>         "\x1b[0m",
        }
    }

    /// Add color to existing string.
    #[cfg(not(target_os = "windows"))]
    pub fn colorize(&self, string: &str) -> String {
        format!("{}{}{}", self.starter_sequence(), string, ANSIColor::Reset.starter_sequence())
    }

    /// Add color to existing string.
    #[cfg(target_os = "windows")]
    pub fn colorize(&self, string: &str) -> String {
        match enable_ansi_support() {
            Err(e) => {
                print!("error when enabling Windows ANSI escapes support, Windows error code: {:?}", e);
            },
            _ => {}
        }

        format!("{}{}{}", self.starter_sequence(), string, ANSIColor::Reset.starter_sequence())
    }
}

#[cfg(target_os = "windows")]
fn enable_ansi_support() -> Result<(), u32> {
    use std::ffi::OsStr;
    use std::iter::once;
    use std::os::windows::ffi::OsStrExt;
    use std::ptr::null_mut;
    use winapi::um::consoleapi::{GetConsoleMode, SetConsoleMode};
    use winapi::um::errhandlingapi::GetLastError;
    use winapi::um::fileapi::{CreateFileW, OPEN_EXISTING};
    use winapi::um::handleapi::INVALID_HANDLE_VALUE;
    use winapi::um::winnt::{FILE_SHARE_WRITE, GENERIC_READ, GENERIC_WRITE};

    const ENABLE_VIRTUAL_TERMINAL_PROCESSING: u32 = 0x0004;
    
    unsafe {
        let console_out_name: Vec<u16> = OsStr::new("CONOUT$").encode_wide().chain(once(0)).collect();
        let console_handle = CreateFileW(
            console_out_name.as_ptr(),
            GENERIC_READ | GENERIC_WRITE,
            FILE_SHARE_WRITE,
            null_mut(),
            OPEN_EXISTING,
            0,
            null_mut(),
        );

        if console_handle == INVALID_HANDLE_VALUE {
            return Err(GetLastError());
        }

        let mut console_mode: u32 = 0;

        if 0 == GetConsoleMode(console_handle, &mut console_mode) {
            return Err(GetLastError());
        }
        
        if console_mode & ENABLE_VIRTUAL_TERMINAL_PROCESSING == 0 {
            if 0 == SetConsoleMode(console_handle, console_mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING) {
                return Err(GetLastError());
            }
        }
    }
    
    Ok(())
}