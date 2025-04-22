pub mod wininfo {
    #[allow(unused)]
    pub fn window_title_from_pid(pid: u32) -> Option<String> {
        use std::process::Command;
        
        #[cfg(target_os="windows")] {
            use std::os::windows::process::CommandExt;
            const CREATENOWINDOW: u32 = 0x08000000;

            let cmd = format!("Get-Process | where {{ $_.Id -eq {} }} | select -ExpandProperty MainWindowTitle",pid);

            let output = Command::new("powershell")
                .creation_flags(CREATENOWINDOW)
                .args(["-Command",&cmd])
                .output()
                .expect("Failed to run process list command");

            let windowtitle= String::from_utf8_lossy(&output.stdout).trim().to_string();

            if !windowtitle.is_empty() {
                return Some(windowtitle)
            }

            None
        }
        
        #[cfg(target_os="linux")] {
            if !wmctrl_deps() {
                return None
            }
        
            let output = Command::new("wmctrl")
                .arg("-lp")
                .output()
                .expect("Failed to get output from \"wmctrl\"");
        
            let stdout = String::from_utf8_lossy(&output.stdout);
        
            for line in stdout.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
        
                if parts.len() < 5 {
                    continue;
                }
        
                if let Ok(line_pid) = parts[2].parse::<u32>() {
                    if line_pid == pid {
                        let window = parts[4..].join(" ");
                        return Some(window);
                    }
                }
            }
        
            None
        }
    }
}