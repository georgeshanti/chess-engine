use std::sync::RwLock;

pub static FILENAME: RwLock<String> = RwLock::new(String::new());
pub static ENABLE_LOG: RwLock<bool> = RwLock::new(true);

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {
        let enable_logging: bool = *crate::core::log::ENABLE_LOG.read().unwrap();
        if enable_logging {
            let path = crate::core::log::FILENAME.read().unwrap().to_string();
            std::fs::create_dir_all(std::path::Path::new(&path).parent().unwrap().clone()).unwrap();
            let mut file = std::fs::OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .open(path)
            .unwrap();
    
            let log_line = format!("{}: {}: {}\n", chrono::Local::now().format("%H:%M:%S").to_string(), std::thread::current().name().unwrap_or("Unknown"), format!($($arg)*));
    
            std::io::Write::write_all(&mut file, log_line.as_bytes()).unwrap();
        }
    };
}