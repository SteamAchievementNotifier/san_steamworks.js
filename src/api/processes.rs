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
    #[allow(unused_mut)]
    fn get_install_dir_exes(dir: String) -> Vec<String> {
        use glob::glob;

        let mut install_dir_exes = Vec::new();

        #[cfg(target_os="windows")]
        let ext = ".exe";

        #[cfg(target_os="linux")]
        let ext = "";

        let pattern = format!("{}/**/*{}",dir,ext);

        for entry in glob(&pattern).expect("Failed to read glob pattern") {
            match entry {
                Ok(file) => {
                    #[cfg(target_os="linux")] {
                        use std::os::unix::fs::PermissionsExt;

                        let metadata = std::fs::metadata(&file).expect("Failed to get file metadata");

                        if metadata.permissions().mode() & 0o111 == 0 {
                            install_dir_exes.push(file.file_name().expect("Failed to get file name").to_string_lossy().to_string())
                        }
                    }

                    #[cfg(target_os="windows")]
                    install_dir_exes.push(file.file_name().expect("Failed to get file name").to_string_lossy().to_string())
                },
                Err(err) => println!("Error while iterating over dir entries: {}",err)
            }
        }
    
        install_dir_exes
    }
    
    pub fn get_game_exes(appid: u32) -> Vec<String> {
        use steamworks::AppId;
    
        let client = crate::client::get_client();
        let installdir = client.apps().app_install_dir(AppId::from(appid));
    
        let mut exes = get_install_dir_exes(installdir);
    
        if cfg!(target_os="windows") {
            exes.push("SAM.Game.exe".to_string());
        }

        exes
    }
    
    #[napi(object)]
    pub struct ProcessInfo {
        pub pid: u32,
        pub exe: String
    }

    #[napi]
    pub fn get_game_processes(appid: u32, linkedgame: Option<String>) -> Vec<ProcessInfo> {
        use std::process::Command;
        use serde_json::{from_str,Value,Error};

        let mut processes = Vec::new();
        get_game_exes(appid);

        let exes = match linkedgame {
            Some(game) => vec![game,"SAM.Game.exe".to_string()],
            None => get_game_exes(appid)
        };

        let output: std::process::Output;
        let cmd = if cfg!(target_os="windows") {
            "Get-WmiObject Win32_Process | Select ProcessName, ProcessId, ExecutablePath | ConvertTo-Json"
        } else if cfg!(target_os="linux") {
            "ps -eo comm,pid,cmd | awk 'NR>1 {print \"{\\n\\t\\\"ProcessName\\\": \\\"\" $1 \"\\\",\\n\\t\\\"ProcessId\\\": \" $2 \",\\n\\t\\\"ExecutablePath\\\": \\\"\" $3 \"\\\"\\n},\"}' | sed '$ s/,$//'"
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
        let json: Result<Value,Error> = from_str(&stdout);

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
                        let pid = process["ProcessId"]
                            .as_u64()
                            .unwrap_or(0) as u32;
    
                        let exe = process["ExecutablePath"]
                            .as_str()
                            .unwrap_or("")
                            .to_string();
                        
                        println!("ProcessName: {}, ProcessId: {}, Executable Path: {}", exename, pid, exe);

                        processes.push(ProcessInfo {
                            pid,
                            exe
                        });
                    }
                }
                Err(err) => println!("No running process found for {}: {}", exename, err),
            }
        }

        processes
    }

    // use window_titles::{Connection, ConnectionTrait};
    // use lazy_static::lazy_static;

    // #[napi(object)]
    // pub struct Info {
    //     pub title: String,
    //     pub name: String,
    //     pub pid: u32
    // }

    // lazy_static! {
    //     static ref CONNECTION: Connection = Connection::new().unwrap();
    // }

    // #[napi]
    // pub fn get_process_info_from_pid(pid: u32) -> Info {
    //     CONNECTION
    //         .windows()
    //         .into_iter()
    //         .filter(|p| p.process().0 == pid)
    //         .nth(0)
    //         .map(|w| Info {
    //             title: w.name().unwrap(),
    //             name: w.process().name(),
    //             pid: w.process().0,
    //         })
    //         .unwrap_or_else(|| Info {
    //             title: "".to_string(),
    //             name: "".to_string(),
    //             pid
    //         })
    // }

    // #[napi]
    // pub fn is_game_window_open(wintitle: String) -> bool {
    //     match CONNECTION
    //         .windows()
    //         .into_iter()
    //         .filter(|p| p.name().expect(&format!("Unable to filter window name value for {}",wintitle)) == wintitle)
    //         .nth(0)
    //         .and_then(|w| Some(w.name())) {
    //         Some(_) => {
    //             println!("Window is active: {}",wintitle);
    //             true
    //         },
    //         None => {
    //             eprintln!("Window is NOT active");
    //             false
    //         }
    //     }
    // }
}
