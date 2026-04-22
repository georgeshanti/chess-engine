use std::{collections::BTreeMap, sync::{Mutex, RwLock}};

pub static FILENAME: RwLock<String> = RwLock::new(String::new());
pub static LAST_LOG_FILENAME: RwLock<String> = RwLock::new(String::new());
pub static ENABLE_LOG: RwLock<bool> = RwLock::new(true);
pub static LAST_LOG: Mutex<BTreeMap<String, String>> = Mutex::new(BTreeMap::new());

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
            (*crate::core::log::LAST_LOG.lock().unwrap()).insert(String::from(std::thread::current().name().unwrap_or("Unknown")), format!("{}: {}", chrono::Local::now().format("%H:%M:%S").to_string(), format!($($arg)*)));
    
            std::io::Write::write_all(&mut file, log_line.as_bytes()).unwrap();
        }
    };
}

#[macro_export]
macro_rules! dump_last_log {
    () => {
        let enable_logging: bool = *crate::core::log::ENABLE_LOG.read().unwrap();
        if enable_logging {
            let path = crate::core::log::LAST_LOG_FILENAME.read().unwrap().to_string();
            std::fs::create_dir_all(std::path::Path::new(&path).parent().unwrap().clone()).unwrap();
            let mut file = std::fs::OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .open(path)
            .unwrap();
            let b = crate::core::log::LAST_LOG.lock().unwrap();
            for (key, value) in b.clone().into_iter() {
                std::io::Write::write_all(&mut file, format!("{}: {}\n", key, value).as_bytes()).unwrap();
            }
        }
    };
}