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

    #[allow(unused_mut)]
    fn get_install_dir_exes(input: String) -> Vec<String> {
        use glob::glob;
        use regex::Regex;
        use std::path::Path;

        let mut install_dir_exes = Vec::new();

        let ext: &str;

        #[cfg(target_os="windows")] {
            ext = ".exe";
        }

        #[cfg(target_os="linux")] {
            ext = "";
        }

        let regex = Regex::new(r#"^(.+?)(?:\s-[a-zA-Z]|$)"#).expect("Failed to create Regex");

        let executable_path = regex.captures(&input)
            .map(|captures| captures.get(1)
            .unwrap()
            .as_str()
            .to_string())
            .unwrap_or_else(|| input);

        let dir = Path::new(&executable_path)
            .to_str()
            .unwrap()
            .to_string();

        let pattern = format!("{}/**/*{}",dir,ext);

        for entry in glob(&pattern).expect("Failed to read glob pattern") {
            match entry {
                Ok(file) => {
                    #[cfg(target_os="linux")] {
                        use std::os::unix::fs::PermissionsExt;

                        let metadata = file.metadata().expect("Failed to get file metadata");

                        if let Some(file_name) = file.file_name() {
                            let file_name = file_name.to_string_lossy().to_string();
                            let has_valid_ext = file_name.find(".").is_none() || file_name.ends_with(".sh") || file_name.ends_with(".so") || file_name.ends_with(".exe");
                            let is_valid = file.is_file() && metadata.permissions().mode() & 0o111 != 0 && has_valid_ext;
        
                            if is_valid {
                                install_dir_exes.push(file_name);
                            }
                        }
                    }

                    #[cfg(target_os="windows")]
                    install_dir_exes.push(file.file_name().expect("Failed to get file name").to_string_lossy().to_string())
                },
                Err(err) => error!("Error while iterating over dir entries: {}",err)
            }
        }
    
        install_dir_exes
    }
    
    fn get_game_exes(appid: u32) -> Vec<String> {
        use steamworks::AppId;
    
        let client = crate::client::get_client();
        let installdir = client.apps().app_install_dir(AppId::from(appid));

        get_install_dir_exes(installdir)
    }
    
    #[napi(object)]
    pub struct ProcessInfo {
        pub pid: u32,
        pub exe: String
    }

    #[allow(unused)]
    #[napi]
    pub fn get_game_processes(appid: u32, linkedgame: Option<String>) -> Vec<ProcessInfo> {
        use std::process::Command;
        use serde_json::{from_str,Value,Error};

        let mut processes = Vec::new();

        let mut exes = match linkedgame {
            Some(game) => vec![game],
            None => get_game_exes(appid)
        };

        if cfg!(target_os="windows") {
            exes.push("SAM.Game.exe".to_string());
        }

        let output: std::process::Output;
        let cmd = if cfg!(target_os="windows") {
            "Get-WmiObject Win32_Process | Select ProcessName, ProcessId, ExecutablePath | ConvertTo-Json"
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
                .expect("Failed to execute \"GetWmiObject\" command");
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
            let regex = Regex::new(r#"^(\S+)\s+(\d+)\s+(.+)$"#).expect("Failed to create Regex");

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
                        
                        info!("ProcessName: {}, ProcessId: {}, Executable Path: {}",exename,pid,exe);

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
}