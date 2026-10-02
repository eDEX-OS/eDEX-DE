//! Live system readout for the greeter's SYSTEM panel, straight from /proc.

use std::collections::VecDeque;

#[derive(Clone, Debug, Default)]
pub struct Stats {
    pub kernel: String,
    pub cpu_model: String,
    pub cores: usize,
    pub mem_total_kb: u64,
    pub mem_used_kb: u64,
    pub uptime_secs: u64,
    /// CPU usage samples (0..1), oldest first.
    pub cpu_history: VecDeque<f32>,
    last_cpu: Option<(u64, u64)>,
}

const HISTORY: usize = 48;

fn read(path: &str) -> String {
    std::fs::read_to_string(path).unwrap_or_default()
}

/// (busy, total) jiffies from the aggregate `cpu` line of /proc/stat.
pub(crate) fn cpu_jiffies(stat: &str) -> Option<(u64, u64)> {
    let line = stat.lines().find(|l| l.starts_with("cpu "))?;
    let v: Vec<u64> = line
        .split_whitespace()
        .skip(1)
        .filter_map(|x| x.parse().ok())
        .collect();
    if v.len() < 4 {
        return None;
    }
    let total: u64 = v.iter().sum();
    let idle = v[3] + v.get(4).copied().unwrap_or(0);
    Some((total - idle, total))
}

pub(crate) fn meminfo_kb(text: &str, key: &str) -> Option<u64> {
    text.lines()
        .find(|l| l.starts_with(key) && l[key.len()..].starts_with(':'))?
        .split_whitespace()
        .nth(1)?
        .parse()
        .ok()
}

impl Stats {
    pub fn new() -> Self {
        let cpuinfo = read("/proc/cpuinfo");
        let cpu_model = cpuinfo
            .lines()
            .find(|l| l.starts_with("model name"))
            .and_then(|l| l.split_once(':'))
            .map(|(_, v)| v.trim().to_string())
            .unwrap_or_else(|| "unknown CPU".into());
        let cores = cpuinfo
            .lines()
            .filter(|l| l.starts_with("processor"))
            .count()
            .max(1);
        let mut s = Self {
            kernel: read("/proc/sys/kernel/osrelease").trim().to_string(),
            cpu_model,
            cores,
            ..Default::default()
        };
        s.refresh();
        s
    }

    pub fn refresh(&mut self) {
        let mem = read("/proc/meminfo");
        self.mem_total_kb = meminfo_kb(&mem, "MemTotal").unwrap_or(0);
        let avail = meminfo_kb(&mem, "MemAvailable").unwrap_or(self.mem_total_kb);
        self.mem_used_kb = self.mem_total_kb.saturating_sub(avail);
        self.uptime_secs = read("/proc/uptime")
            .split_whitespace()
            .next()
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0) as u64;
        if let Some((busy, total)) = cpu_jiffies(&read("/proc/stat")) {
            if let Some((b0, t0)) = self.last_cpu {
                let dt = total.saturating_sub(t0).max(1) as f32;
                let frac = (busy.saturating_sub(b0) as f32 / dt).clamp(0.0, 1.0);
                if self.cpu_history.len() >= HISTORY {
                    self.cpu_history.pop_front();
                }
                self.cpu_history.push_back(frac);
            }
            self.last_cpu = Some((busy, total));
        }
    }

    pub fn cpu_now(&self) -> f32 {
        self.cpu_history.back().copied().unwrap_or(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_proc_files() {
        assert_eq!(
            cpu_jiffies("cpu  100 0 50 800 50 0 0 0 0 0\ncpu0 1 2 3 4\n"),
            Some((150, 1000))
        );
        let mem = "MemTotal:       16000000 kB\nMemFree:  1000 kB\nMemAvailable:   12000000 kB\n";
        assert_eq!(meminfo_kb(mem, "MemTotal"), Some(16_000_000));
        assert_eq!(meminfo_kb(mem, "MemAvailable"), Some(12_000_000));
        assert_eq!(meminfo_kb(mem, "Mem"), None);
    }

    #[test]
    fn samples_this_machine() {
        let mut s = Stats::new();
        s.refresh();
        assert!(s.cores >= 1);
        assert!(!s.cpu_history.is_empty());
    }
}
