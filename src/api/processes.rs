use napi_derive::napi;

#[cfg(target_os="windows")]
pub mod win32 {
    pub use std::{process::Command,os::windows::process::CommandExt};
    pub const CREATENOWINDOW: u32 = 0x08000000;
    pub use winreg::RegKey;
    pub use winreg::enums::HKEY_CURRENT_USER;
}

#[napi]
pub mod processes {
    fn get_install_dir_exes(appid: u32, dir: &std::path::Path) -> Vec<String> {    
        let mut install_dir_exes = Vec::new();
    
        match std::fs::read_dir(dir) {
            Ok(entries) => {
                for entry in entries.filter_map(|e| e.ok()) {
                    let path = entry.path();
                    if path.is_file() && path.extension().map_or(false, |ext| ext == "exe") {
                        install_dir_exes.push(path.file_name().unwrap().to_string_lossy().to_string());
                    } else if path.is_dir() {
                        install_dir_exes.extend(get_install_dir_exes(appid, &path));
                    }
                }
            }
            Err(err) => {
                eprint!("Failed to read {:?} directory: {}",dir,err);
            }
        }
    
        install_dir_exes
    }
    
    pub fn get_game_exes(appid: u32) -> Vec<String> {
        use steamworks::AppId;
    
        let client = crate::client::get_client();
        let installdir = client.apps().app_install_dir(AppId::from(appid));
    
        let mut exes = get_install_dir_exes(appid,&std::path::PathBuf::from(installdir));
    
        exes.push("SAM.Game.exe".to_string());
        exes
    }
    
    #[napi(object)]
    pub struct ProcessInfo {
        pub exe: String,
        pub pid: u32,
    }
    
    #[napi]
    pub fn get_game_processes(appid: u32) -> Vec<ProcessInfo> {
        use super::win32::{Command,CommandExt,CREATENOWINDOW};
        let mut processes = Vec::new();
    
        for exe_name in get_game_exes(appid) {
            let output = Command::new("tasklist")
                .creation_flags(CREATENOWINDOW)
                .arg("/FI")
                .arg(format!("IMAGENAME eq {}",exe_name))
                .output()
                .expect("Failed to execute command");

            let tasklist = String::from_utf8_lossy(&output.stdout);
    
            for line in tasklist.lines().skip(3) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(pid) = parts[1].parse::<u32>() {
                        processes.push(ProcessInfo {
                            exe: exe_name.clone(),
                            pid,
                        });
                    }
                }
            }
        }
    
        processes
    }

    use window_titles::{Connection, ConnectionTrait};
    use lazy_static::lazy_static;

    #[napi(object)]
    pub struct Info {
        pub title: String,
        pub name: String,
        pub pid: u32
    }

    lazy_static! {
        static ref CONNECTION: Connection = Connection::new().unwrap();
    }

    #[napi]
    pub fn get_process_info_from_pid(pid: u32) -> Info {
        CONNECTION
            .windows()
            .into_iter()
            .filter(|p| p.process().0 == pid)
            .nth(0)
            .map(|w| Info {
                title: w.name().unwrap(),
                name: w.process().name(),
                pid: w.process().0,
            })
            .unwrap_or_else(|| Info {
                title: "".to_string(),
                name: "".to_string(),
                pid
            })
    }

    #[napi]
    pub fn is_game_window_open(wintitle: String) -> bool {
        match CONNECTION
            .windows()
            .into_iter()
            .filter(|p| p.name().expect(&format!("Unable to filter window name value for {}",wintitle)) == wintitle)
            .nth(0)
            .and_then(|w| Some(w.name())) {
            Some(_) => {
                println!("Window is active: {}",wintitle);
                true
            },
            None => {
                eprintln!("Window is NOT active");
                false
            }
        }
    }
}
