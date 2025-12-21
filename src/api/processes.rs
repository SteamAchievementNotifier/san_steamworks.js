extern crate glob;
use napi_derive::napi;

#[cfg(target_os="windows")]
pub mod win32 {
    pub use std::os::windows::process::CommandExt;
    pub const CREATENOWINDOW: u32 = 0x08000000;
    pub use winreg::RegKey;
    pub use winreg::enums::HKEY_CURRENT_USER;
}

#[napi]
pub mod processes {
    use log::{info,error};

    #[napi]
    pub fn get_appinfo(steampath: String) -> serde_json::Value {
        use appinfovdf::get_appinfo;
        get_appinfo(&steampath)
    }

    #[napi]
    pub fn get_appinfo_for_appid(appid: u32,steampath: String) -> serde_json::Value {
        use appinfovdf::get_appinfo_for_appid;
        
        get_appinfo_for_appid(appid,&steampath).unwrap_or(serde_json::Value::Null)
    }

    fn get_appinfo_exe(appid: u32,steampath: String) -> Option<String> {
        use appinfovdf::get_appinfo_for_appid;
        use serde_json::Value::{Object,String};
        
        let appinfo = get_appinfo_for_appid(appid,&steampath);
        
        let platform = if cfg!(target_os="windows") {
            "windows"
        } else if cfg!(target_os="linux") {
            "linux"
        } else {
            return None;
        };

        let clone = appinfo?;
        let config = clone.get("config")?;
        let launch = config.get("launch")?.as_object()?;

        for entry in launch.values() {
            if let Object(entrymap) = entry {
                let os_match = if let Some(Object(config)) = entrymap.get("config") {
                    // Returns `false` if "oslist" key is present but does not match the current OS
                    if let Some(String(oslist)) = config.get("oslist") {
                        oslist == platform
                    // If outer "config" key exists but does not contain an inner "oslist" key, assume this entry is for all platforms and match
                    } else {
                        true
                    }
                // If no outer "config" key exists, also assume this entry is for all platforms and match
                } else {
                    true
                };

                if !os_match {
                    continue;
                }

                if let Some(String(exe)) = entrymap.get("executable") {
                    return Some(exe.clone());
                }
            }
        }

        None
    }
    
    #[napi(object)]
    pub struct ProcessInfo {
        pub pid: u32,
        pub exe: String
    }

    #[allow(unused)]
    #[napi]
    pub fn get_game_processes(appid: u32,steampath: String,linkedgame: Option<String>) -> Vec<ProcessInfo> {
        use std::process::Command;
        use serde_json::{from_str,Value,Error};

        let mut processes = Vec::new();

        let mut exes = match linkedgame {
            Some(game) => vec![game],
            None => match get_appinfo_exe(appid,steampath) {
                Some(exe) => {
                    info!("Found executable entry \"{}\" in \"appinfo.vdf\" for AppID {}",exe,appid);
                    vec![exe]
                },
                None => {
                    error!("Unable to find valid executable entry in \"appinfo.vdf\" for AppID {}",appid);
                    return Vec::new()
                }
            }
        };

        if cfg!(target_os="windows") {
            exes.push("SAM.Game.exe".to_string());
        }

        let output: std::process::Output;
        let cmd = if cfg!(target_os="windows") {
            "Get-CimInstance Win32_Process | Select ProcessName, ProcessId, ExecutablePath | ConvertTo-Json"
        } else if cfg!(target_os="linux") {
            "ps -eo comm,pid,cmd --no-headers"
        } else {
            "Unsupported platform"
        };

        #[cfg(target_os="windows")] {
            use super::win32::{CommandExt,CREATENOWINDOW};

            output = Command::new("powershell")
                .creation_flags(CREATENOWINDOW)
                .args(["-Command",cmd])
                .output()
                .expect("Failed to run process list command");
        }

        #[cfg(target_os="linux")] {
            output = Command::new("sh")
                .args(["-c",cmd])
                .output()
                .expect("Failed to execute \"ps\" command");
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        let json: Result<Value,Error>;

        #[cfg(target_os="windows")] {
            json = from_str(&stdout);
        }
        
        #[cfg(target_os="linux")] {
            use regex::Regex;

            let mut json_output = Vec::new();
            let regex = Regex::new(REGEX).expect("Failed to create Regex");

            for line in stdout.lines() {
                if let Some(captures) = regex.captures(line) {
                    let process_name = captures.get(1).map_or("<unknown>",|m| m.as_str().trim());
                    let process_id = captures.get(2).map_or("<unknown>",|m| m.as_str().trim());
                    let executable_path = captures.get(3).map_or("<unknown>",|m| m.as_str().trim());

                    let json_obj = serde_json::json!({
                        "ProcessName": process_name,
                        "ProcessId": process_id,
                        "ExecutablePath": executable_path
                    });

                    json_output.push(json_obj);
                }
            }

            let json_array = Value::Array(json_output).to_string();

            json = from_str(&json_array)
        }

        for exename in exes {
            match &json {
                Ok(value) => {
                    let stdoutprocesses = value
                        .as_array()
                        .expect("\"json\" is not an array")
                        .iter()
                        .filter(|p| {
                            if let Some(pname) = p["ProcessName"].as_str() {
                                exename.to_lowercase() == pname.to_lowercase()
                            } else {
                                false
                            }
                        })
                        .collect::<Vec<_>>();
    
                    for process in stdoutprocesses {
                        let pid = if cfg!(target_os="windows") {
                            process["ProcessId"]
                                .as_u64()
                                .unwrap_or(0) as u32
                        } else {
                            process["ProcessId"]
                                .as_str()
                                .unwrap_or("0")
                                .parse::<u32>()
                                .unwrap_or(0) as u32
                        };
    
                        let exe = process["ExecutablePath"]
                            .as_str()
                            .unwrap_or("")
                            .to_string();

                        info!("ProcessName: {}, ProcessId: {}, ExecutablePath: {}",exename,pid,exe);

                        processes.push(ProcessInfo {
                            pid,
                            exe
                        });
                    }
                }
                Err(err) => error!("No running process found for {}: {}",exename,err),
            }
        }

        processes
    }

    #[napi]
    pub fn is_process_running(pid: u32) -> bool {
        use process_alive::{state,State,Pid};

        state(Pid::from(pid)) == State::Alive
    }

    #[napi]
    pub fn get_window_title(pid: u32) -> String {
        match crate::api::wininfo::wininfo::window_title_from_pid(pid) {
            Some(windowtitle) => windowtitle,
            None => "".to_string()
        }
    }
}