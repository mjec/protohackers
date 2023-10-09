use std::sync::Mutex;

use log::{Level, Metadata, Record};

pub(crate) fn init() -> Result<(), log::SetLoggerError> {
    log::set_logger(&LOGGER)
        .map(|()| log::set_max_level(log::LevelFilter::Info))?;

    std::thread::spawn(|| {
        loop {
            std::thread::sleep(std::time::Duration::from_millis(LOG_AUTO_FLUSH_INTERVAL_MS));
            log::logger().flush();
        }
    });

    Ok(())
}

const LOG_AUTO_FLUSH_INTERVAL_MS: u64 = 200;

struct BufferedStderrLogger;

static LOGGER: BufferedStderrLogger = BufferedStderrLogger;
static LOG_BUFFER: Mutex<Vec<String>> = Mutex::new(Vec::new());

impl log::Log for BufferedStderrLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let mut result = format!("[{}] {}", record.level(), record.args());
            let mut collector = KVCollector::new();
            record.key_values().visit(&mut collector).expect("KVCollector cannot fail");
            result.push_str(collector.result.as_str());
            LOG_BUFFER.lock().unwrap().push(result);
        }
    }

    fn flush(&self) {
        let buf: Vec<String> = LOG_BUFFER.lock().unwrap().drain(..).collect();
        if !buf.is_empty() {
            eprintln!("{}", buf.join("\n"));
        }
    }
}

struct KVCollector {
    result: String,
}

impl KVCollector {
    fn new() -> Self {
        KVCollector {
            result: String::new(),
        }
    }
}

impl<'kvs> log::kv::Visitor<'kvs> for KVCollector {
    fn visit_pair(&mut self, key: log::kv::Key<'kvs>, value: log::kv::Value<'kvs>) -> Result<(), log::kv::Error> {
        self.result.push(' ');
        self.result.push_str(key.as_str());
        self.result.push('=');
        self.result.push_str(format!("{:?}", value).as_str());
        Ok(())
    }
}