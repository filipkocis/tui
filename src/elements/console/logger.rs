use log::{Level, LevelFilter, Metadata, Record};

struct ConsoleLogger;

impl log::Log for ConsoleLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        #[cfg(debug_assertions)]
        return metadata.level() <= Level::Trace;

        #[cfg(not(debug_assertions))]
        return metadata.level() <= Level::Info;
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let out = format!(
                "[{} {}] {}",
                record.level(),
                record.target(), // Shows the module path!
                record.args()
            );
            super::Console::log(out);
        }
    }

    fn flush(&self) {}
}

static LOGGER: ConsoleLogger = ConsoleLogger;

/// Initializes the logger for the console.
/// Has to be called before any logging occurs via the [log] crate.
/// # Note
/// This **has** to be called manually by the user, it does not happen automatically.
pub fn init() {
    log::set_logger(&LOGGER)
        .map(|()| {
            #[cfg(debug_assertions)]
            return log::set_max_level(LevelFilter::Trace);

            #[cfg(not(debug_assertions))]
            return log::set_max_level(LevelFilter::Info);
        })
        .expect("Failed to set logger");
}
