#[macro_export]
macro_rules! headless {
    ($($arg:tt)*) => {
        // You can add custom logic here before calling println!
        if std::env::var("HEADLESS").is_ok() {
            println!("{}: {}", std::thread::current().name().unwrap_or("Unknown"), format!($($arg)*));
        }
    };
}

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {
        let mut file = std::fs::OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open(std::env::var("LOG_FILE").unwrap())
        .unwrap();

        file.write_all(format!("{}: {}\n", chrono::Local::now().format("%H:%M:%S").to_string(), format!($($arg)*)).as_bytes()).unwrap();
    };
}