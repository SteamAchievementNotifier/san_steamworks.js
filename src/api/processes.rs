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
    use serde_json::{Map, Value};

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

    pub fn get_appinfo_exe(appid: u32, steampath: String) -> Option<Vec<String>> {
        use serde_json::{Value,Map};

        let mut executables: Vec<String> = Vec::new();

        let appinfo = appinfovdf::get_appinfo_for_appid(appid,&steampath)?;

        let launch = appinfo
            .get("config")?
            .get("launch")?
            .as_object()?;

        let mut linux_entry: Option<&Map<String,Value>> = None;
        let mut windows_entry: Option<&Map<String,Value>> = None;
        let mut fallback_entry: Option<&Map<String,Value>> = None;

        for entry in launch.values() {
            let entrymap = entry.as_object()?;
            let oslist = entrymap
                .get("config")
                .and_then(|value| value.as_object())
                .and_then(|config| config.get("oslist"))
                .and_then(|value| value.as_str());

            match oslist {
                Some("linux") => {
                    linux_entry.get_or_insert(entrymap);
                    break;
                },
                Some("windows") => {
                    windows_entry.get_or_insert(entrymap);
                },
                _ => {
                    fallback_entry.get_or_insert(entrymap);
                }
            }
        }

        // Get executables for all platforms to compare against running processes
        let windows_executable = get_entry_executable(windows_entry);
        if windows_executable.is_some() {
            executables.push(windows_executable?);
        }

        let linux_executable = get_entry_executable(linux_entry);
        if linux_executable.is_some() {
            executables.push(linux_executable?);
        }

        let fallback_executable = get_entry_executable(fallback_entry);
        if fallback_executable.is_some() {
            executables.push(fallback_executable?);
        }

        return Some(executables);
    }

    #[napi(object)]
    pub struct ProcessInfo {
        pub pid: u32,
        pub exe: String
    }

    #[allow(unused)]
    #[napi]
    pub fn get_game_processes(appid: u32, steampath: String, linkedgame: Option<String>) -> Vec<ProcessInfo> {
        use std::process::Command;
        use serde_json::{from_str,Value,Error};

        let mut processes = Vec::new();

        let mut exes = match linkedgame {
            Some(game) => vec![game],
            None => match get_appinfo_exe(appid, steampath) {
                Some(executables) => {
                    info!("Found executable entry \"{executables:?}\" in \"appinfo.vdf\" for AppID {appid}");
                    executables
                },
                None => {
                    error!("Unable to find valid executable entry in \"appinfo.vdf\" for AppID {appid}");
                    return Vec::new()
                }
            }
        };

        if cfg!(target_os="windows") {
            exes.push("SAM.Game.exe".to_string());
        }


        let json: Result<Value,Error>;

        #[cfg(target_os="windows")] {
            use super::win32::{CommandExt,CREATENOWINDOW};

            let output: std::process::Output;

            output = Command::new("powershell")
                .creation_flags(CREATENOWINDOW)
                .args(["-Command", "Get-CimInstance Win32_Process | Select ProcessName, ProcessId, ExecutablePath | ConvertTo-Json"])
                .output()
                .expect("Failed to run process list command");

            let stdout = String::from_utf8_lossy(&output.stdout);
            json = from_str(&stdout);
        }

        #[cfg(target_os="linux")] {
            let mut json_output = Vec::new();

            for proc in procfs::process::all_processes().unwrap() {
                match get_process_info(proc) {
                    Err(err) => {
                        // Skip process, most likely due to permissions
                        continue;
                    },
                    Ok(x) => json_output.push(x)
                };
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
                        let pid = process["ProcessId"]
                            .as_u64()
                            .unwrap_or(0) as u32;

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

    fn get_entry_executable(entry: Option<&Map<String,Value>>) -> Option<String> {
        use std::path::Path;

        let executable = entry?.get("executable")?.as_str()?;
        let executablepath = Path::new(executable);

        // Checks whether "config.launch[<entry>].workingdir" key exists
        // If so, also checks the "workingdir" value is not also specified in the "executable" value to prevent path duplication
        let value = if let Some(workingdir) = entry
            ?.get("workingdir")
            .and_then(|value| value.as_str())
        {
            let workingdirpath = Path::new(workingdir);

            // Normalise and compare paths in lowercase to prevent unintended mismatches
            let executable_lowercase = executable.replace("\\","/").to_lowercase();
            let workingdir_lowercase = workingdir.replace("\\","/").to_lowercase();

            // Use "executable" value if it already contains "workingdir" value
            if executable_lowercase.starts_with(&workingdir_lowercase) {
                executablepath.to_path_buf()
            // If these values differ, prepend "workingdir" value to "executable" value
            } else {
                workingdirpath.join(executablepath)
            }
        // If no "workingdir" key, return "executable" value as-is
        } else {
            executablepath.to_path_buf()
        };

        // Normalise the resulting path before returning
        value.to_str().map(|str| str.replace("\\","/").to_string())
    }

    #[cfg(target_os="linux")]
    fn get_process_info(process: Result<procfs::process::Process, procfs::ProcError>) -> Result<serde_json::Value, procfs::ProcError> {
        let proc = process?;
        let process_path = proc.exe()?;
        let process_id = proc.pid;
        let cmdline = proc.cmdline()?;

        // app is running through wine, get the real executable from the arguments
        let mut executable = process_path.file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        if executable == "wine-preloader" || executable == "wine64-preloader" {
            println!("Found wine process {}", &cmdline[0]);
            let normalized_path = cmdline[0].clone().replace("\\", "/");
            let full_path = std::path::PathBuf::from(normalized_path);
            let actual_name= full_path.file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .to_string();

            executable = actual_name;
        }

        return Ok(serde_json::json!({
            "ProcessName": executable,
            "ProcessId": process_id,
            "ExecutablePath": process_path
        }));
    }
}
