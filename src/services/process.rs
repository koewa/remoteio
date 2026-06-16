use procfs::process::all_processes;
use serde::Serialize;
use std::time::Duration;
use tokio::sync::broadcast::Sender;

#[derive(Debug, Clone, Serialize)]
pub struct ProcessInfo {
    pub pid: i32,
    pub name: String,
    pub state: String,
    pub uid: u32,
    pub rss_kb: u64,
    pub cpu_time_sec: f64,
    pub cmdline: String,
}

fn collect_processes() -> Vec<ProcessInfo> {
    let iter = match all_processes() {
        Ok(iter) => iter,
        Err(_) => return Vec::new(),
    };

    let page_size = procfs::page_size();
    let ticks = procfs::ticks_per_second() as f64;

    iter.filter_map(|proc_result| {
        let proc = proc_result.ok()?;
        let pid = proc.pid;

        let stat = proc.stat().ok()?;

        let name = stat.comm;
        let state = stat.state.to_string();
        let cpu_time_sec = (stat.utime + stat.stime) as f64 / ticks;
        let rss_kb = stat.rss * page_size / 1024;

        let status = proc.status().ok()?;
        let uid = status.ruid;

        let cmdline = proc.cmdline().unwrap_or_default().join(" ");

        Some(ProcessInfo {
            pid,
            name,
            state,
            uid,
            rss_kb,
            cpu_time_sec,
            cmdline,
        })
    })
    .collect()
}

pub fn setup_process_monitor(tx: Sender<String>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(2));
        loop {
            interval.tick().await;
            let processes = collect_processes();
            if let Ok(json) = serde_json::to_string(&processes) {
                let _ = tx.send(json);
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collect_processes_non_empty() {
        let procs = collect_processes();
        assert!(!procs.is_empty(), "should find at least init/systemd");
    }

    #[test]
    fn test_collect_processes_includes_pid_1() {
        let procs = collect_processes();
        assert!(procs.iter().any(|p| p.pid == 1), "pid 1 should exist");
    }

    #[test]
    fn test_collect_processes_fields() {
        let procs = collect_processes();
        for p in &procs {
            assert!(p.pid > 0, "pid should be positive");
            assert!(!p.name.is_empty(), "name should not be empty");
        }
    }

    #[test]
    fn test_process_serialization() {
        let ps = ProcessInfo {
            pid: 42,
            name: "test".into(),
            state: "R".into(),
            uid: 1000,
            rss_kb: 4096,
            cpu_time_sec: 1.5,
            cmdline: "/bin/test --foo".into(),
        };
        let json = serde_json::to_string(&ps).unwrap();
        assert!(json.contains("\"pid\":42"));
        assert!(json.contains("\"name\":\"test\""));
    }
}
