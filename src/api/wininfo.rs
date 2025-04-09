pub mod wininfo {
    #[allow(unused)]
    pub fn get_window_title(pid: u32) -> Option<String> {
        #[cfg(target_os="windows")] {
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