use std::sync::RwLock;

pub static filename: RwLock<String> = RwLock::new(String::new());
pub static headless_flag: RwLock<bool> = RwLock::new(false);

#[macro_export]
macro_rules! headless {
    ($($arg:tt)*) => {
        // You can add custom logic here before calling println!
        // if core::macros::headless_flag.read().unwrap() {
        //     println!("{}: {}", std::thread::current().name().unwrap_or("Unknown"), format!($($arg)*));
        // }
    };
}

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {
        let mut file = std::fs::OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open(core::macros::filename.read().unwrap().to_string())
        .unwrap();

        file.write_all(format!("{}: {}\n", chrono::Local::now().format("%H:%M:%S").to_string(), format!($($arg)*)).as_bytes()).unwrap();
    };
}