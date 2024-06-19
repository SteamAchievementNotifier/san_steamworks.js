use napi_derive::napi;

#[napi]
pub mod log {
    use fern::Dispatch;
    use log::{LevelFilter,error};
    use std::fs::File;

    #[napi]
    pub fn init_logger(app_data: String) -> String {
        let logfile = std::path::Path::new(&app_data).join("rust.log");

        let logger_config = Dispatch::new()
            .format(|out,message,record| {
                out.finish(format_args!(
                    "[{}]\n[{}]: {}\n{}\n",
                    chrono::Local::now().format("%d/%m/%Y %H:%M:%S"),
                    record.target(),
                    record.level(),
                    message
                ))
            })
            .level(LevelFilter::Debug)
            .chain(File::create(logfile).expect("Failed to create log file"))
            .apply();

        if let Err(err) = logger_config {
            eprintln!("Failed to initialise \"rust.log\": {}",err);
            return format!("Failed to initialise \"rust.log\": {}",err).to_string()
        }

        std::panic::set_hook(Box::new(|panic_info| {
            let location = panic_info.location().map_or_else(
                || "unknown location".to_string(),
                |loc| format!("{}:{}:{}",loc.file(),loc.line(),loc.column()),
            );

            let payload = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
                *s
            } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
                s.as_str()
            } else {
                "unknown panic payload"
            };

            error!("Panic occurred at \"{}\": {}",location,payload);
        }));

        "Initialised \"rust.log\" successfully".to_string()
    }

    #[napi]
    pub fn test_panic() {
        panic!("This is a test panic");
    }
}