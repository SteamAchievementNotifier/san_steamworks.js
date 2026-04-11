use napi_derive::napi;

#[napi]
pub mod screenshots {
    use std::path::Path;
    use log::{info,error};

    #[napi]
    pub fn add_screenshot_to_library(filename: String,width: i32,height: i32) -> u32 {
        let client = crate::client::get_client();
        let filepath = Path::new(&filename);
        
        match client.screenshots().add_screenshot_to_library(filepath,None,width,height) {
            Ok(handle) => {
                info!("\"{}\" added to Steam Library successfully",filename);
                return handle
            },
            Err(err) => {
                error!("Unable to add \"{}\" to Steam Library: {}",filename,err);
                return 0
            }
        }
    }
}