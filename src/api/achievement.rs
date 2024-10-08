use napi_derive::napi;

#[napi]
pub mod achievement {
    use std::{thread::sleep,time::Duration};
    use log::error;

    const MAX: usize = 10;

    #[napi]
    pub fn is_activated(achievement: String) -> bool {
        let client = crate::client::get_client();
        client
            .user_stats()
            .achievement(&achievement)
            .get()
            .unwrap_or(false)
    }

    #[napi]
    pub fn unlock(achievement: String) -> bool {
        let client = crate::client::get_client();
        client
            .user_stats()
            .achievement(&achievement)
            .set()
            .and_then(|_| client.user_stats().store_stats())
            .is_ok()
    }

    #[napi]
    pub fn lock(achievement: String) -> bool {
        let client = crate::client::get_client();
        client
            .user_stats()
            .achievement(&achievement)
            .clear()
            .and_then(|_| client.user_stats().store_stats())
            .is_ok()
    }

    #[napi]
    pub fn get_achievement_display_attribute(achievement: String, key: String) -> String {
        let client = crate::client::get_client();
        let user_stats = client.user_stats();
        let achievement_data = user_stats.achievement(&achievement);
        let result = achievement_data
            .get_achievement_display_attribute(&key)
            .unwrap_or_default();

        if result.is_empty() {
            error!("Failed to get \"{}\" attribute for \"{}\"",&key,&achievement);
        }

        result.to_string()
    }

    #[napi]
    pub fn get_achievement_achieved_percent(achievement: String) -> f32 {
        let client = crate::client::get_client();

        for i in 0..MAX {
            match client
                .user_stats()
                .achievement(&achievement)
                .get_achievement_achieved_percent()
            {
                Ok(percent) => return percent,
                Err(_) => {
                    error!("{}/{}: Retrying attempt to fetch achievement percentage for {}",i,MAX,&achievement);
                    sleep(Duration::from_millis(250));
                }
            }
        }

        error!("{}/{} ATTEMPTS FAILED: Failed to fetch achievement percentage for {}",MAX,MAX,&achievement);
        0.0
    }

    #[napi(object)]
    pub struct Icon {
        pub handle: Vec<u8>,
        pub width: u32,
        pub height: u32
    }

    #[napi]
    pub fn get_achievement_icon(achievement: String) -> Option<Icon> {
        let client = crate::client::get_client();

        for i in 0..MAX {
            if let Some(icon) = client
                .user_stats()
                .achievement(&achievement)
                .get_achievement_icon()
            {
                return Some(Icon {
                    handle: icon.handle,
                    width: icon.width,
                    height: icon.height
                })
            } else {
                error!("{}/{}: Retrying attempt to fetch achievement icon for {}",i,MAX,&achievement);
                sleep(Duration::from_millis(250));
            }
        }

        error!("{}/{} ATTEMPTS FAILED: Failed to fetch achievement icon for {}",MAX,MAX,&achievement);
        Some(Icon {
            handle: vec![0],
            width: 0,
            height: 0
        })
    }

    #[napi]
    pub fn get_num_achievements() -> u32 {
        let client = crate::client::get_client();
            
        for i in 0..MAX {
            match client.user_stats().get_num_achievements() {
                Ok(num) => return num,
                Err(_) => {
                    error!("{}/{}: Retrying attempt to get number of achievements",i,MAX);
                    sleep(Duration::from_millis(250));
                }
            }
        }
    
        error!("{}/{} ATTEMPTS FAILED: Failed to get number of achievements",MAX,MAX);
        0
    }

    #[napi]
    pub fn get_achievement_names() -> Vec<String> {
        let client = crate::client::get_client();

        for i in 0..MAX {
            if let Some(names) = client
                .user_stats()
                .get_achievement_names()
            {
                return names
            } else {
                error!("{}/5: Retrying attempt to get achievement names",i);
                sleep(Duration::from_millis(250));
            }
        }

        error!("{}/{} ATTEMPTS FAILED: Failed to get achievement names",MAX,MAX);
        Vec::new()
    }
}
