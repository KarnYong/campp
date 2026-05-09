use std::process::{Command, Stdio};

pub fn kill_existing_processes(process_name: &str) {
    #[cfg(windows)]
    {
        let _ = Command::new("taskkill")
            .args(["/F", "/IM", &format!("{}.exe", process_name)])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .output();

        std::thread::sleep(std::time::Duration::from_millis(500));
    }

    #[cfg(unix)]
    {
        let pkill_result = Command::new("pkill")
            .args(["-9", process_name])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .output();

        if pkill_result.is_err() {
            let _ = Command::new("killall")
                .args(["-9", process_name])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .output();
        }

        std::thread::sleep(std::time::Duration::from_millis(500));
    }
}
