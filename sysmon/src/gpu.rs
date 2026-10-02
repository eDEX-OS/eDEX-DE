//! GPU statistics for every GPU in the system, from sysfs (amdgpu, i915, xe, nouveau,
//! virtio) and `nvidia-smi` for the proprietary NVIDIA driver.

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct GpuInfo {
    /// Marketing name from the PCI database (or nvidia-smi), else the driver name.
    pub name: String,
    pub driver: String,
    /// Busy percentage, where the driver reports it.
    pub busy_pct: Option<f32>,
    pub vram_used_bytes: Option<u64>,
    pub vram_total_bytes: Option<u64>,
    pub temp_c: Option<f32>,
    pub power_w: Option<f32>,
    pub clock_mhz: Option<u32>,
}

struct Card {
    /// /sys/class/drm/cardN
    path: PathBuf,
    device: PathBuf,
    driver: String,
    name: String,
    /// PCI bus id (0000:01:00.0) used to match nvidia-smi rows.
    bus_id: String,
}

pub struct GpuCollector {
    cards: Vec<Card>,
}

fn read(p: impl AsRef<Path>) -> Option<String> {
    fs::read_to_string(p).ok().map(|s| s.trim().to_string())
}

fn read_u64(p: impl AsRef<Path>) -> Option<u64> {
    read(p)?.parse().ok()
}

/// Look up "vendor device" names in the PCI id database.
fn pci_name(vendor: u16, device: u16) -> Option<String> {
    let db = ["/usr/share/hwdata/pci.ids", "/usr/share/misc/pci.ids"]
        .iter()
        .find_map(|p| fs::read_to_string(p).ok())?;
    pci_name_from(&db, vendor, device)
}

pub(crate) fn pci_name_from(db: &str, vendor: u16, device: u16) -> Option<String> {
    let v = format!("{vendor:04x}");
    let d = format!("\t{device:04x}");
    let mut in_vendor = false;
    let mut vendor_name = String::new();
    for line in db.lines() {
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        if !line.starts_with('\t') {
            if in_vendor {
                break;
            }
            if let Some(rest) = line.strip_prefix(&v) {
                in_vendor = true;
                vendor_name = rest.trim().to_string();
            }
            continue;
        }
        if in_vendor && line.starts_with(&d) && !line.starts_with("\t\t") {
            let name = line[d.len()..].trim();
            // "Navi 31 [Radeon RX 7900 XT/7900 XTX]" → the bracketed product name.
            let product = match (name.find('['), name.rfind(']')) {
                (Some(a), Some(b)) if b > a => &name[a + 1..b],
                _ => name,
            };
            let short_vendor = match vendor {
                0x1002 => "AMD",
                0x10de => "NVIDIA",
                0x8086 => "Intel",
                // "Red Hat, Inc." → "Red Hat"
                _ => vendor_name.split(',').next().unwrap_or("").trim(),
            };
            return Some(format!("{short_vendor} {product}").trim().to_string());
        }
    }
    None
}

fn hex(s: Option<String>) -> Option<u16> {
    u16::from_str_radix(s?.trim_start_matches("0x"), 16).ok()
}

/// First hwmon directory of a device.
fn hwmon(device: &Path) -> Option<PathBuf> {
    fs::read_dir(device.join("hwmon"))
        .ok()?
        .flatten()
        .map(|e| e.path())
        .next()
}

/// "1: 2100Mhz *" lines of a pp_dpm_* file → the active clock.
pub(crate) fn active_dpm_clock(s: &str) -> Option<u32> {
    s.lines()
        .find(|l| l.trim_end().ends_with('*'))
        .and_then(|l| {
            let mhz = l.split_whitespace().nth(1)?;
            mhz.trim_end_matches(|c: char| !c.is_ascii_digit())
                .parse()
                .ok()
        })
}

impl GpuCollector {
    pub fn new() -> Self {
        let mut cards = Vec::new();
        let mut entries: Vec<_> = fs::read_dir("/sys/class/drm")
            .map(|d| d.flatten().map(|e| e.path()).collect())
            .unwrap_or_default();
        entries.sort();
        for path in entries {
            let fname = path.file_name().unwrap_or_default().to_string_lossy();
            // cardN only, not connectors (card0-DP-1) or render nodes.
            if !fname.starts_with("card") || fname.contains('-') {
                continue;
            }
            let device = path.join("device");
            let driver = fs::read_link(device.join("driver"))
                .ok()
                .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
                .unwrap_or_default();
            if driver.is_empty() {
                continue;
            }
            let bus_id = fs::canonicalize(&device)
                .ok()
                .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
                .unwrap_or_default();
            let name = match (
                hex(read(device.join("vendor"))),
                hex(read(device.join("device"))),
            ) {
                (Some(v), Some(d)) => pci_name(v, d),
                _ => None,
            }
            .unwrap_or_else(|| match driver.as_str() {
                "virtio-pci" | "virtio_gpu" => "Virtio GPU".into(),
                d => d.to_uppercase(),
            });
            cards.push(Card {
                path,
                device,
                driver,
                name,
                bus_id,
            });
        }
        Self { cards }
    }

    pub fn is_empty(&self) -> bool {
        self.cards.is_empty()
    }

    pub fn sample(&self) -> Vec<GpuInfo> {
        let nvidia = if self.cards.iter().any(|c| c.driver == "nvidia") {
            nvidia_smi()
        } else {
            HashMap::new()
        };
        self.cards.iter().map(|c| sample_card(c, &nvidia)).collect()
    }
}

impl Default for GpuCollector {
    fn default() -> Self {
        Self::new()
    }
}

fn sample_card(c: &Card, nvidia: &HashMap<String, GpuInfo>) -> GpuInfo {
    if c.driver == "nvidia" {
        let key = c.bus_id.to_ascii_lowercase();
        if let Some(info) = nvidia.get(&key) {
            return info.clone();
        }
    }
    let d = &c.device;
    let mut g = GpuInfo {
        name: c.name.clone(),
        driver: c.driver.clone(),
        ..Default::default()
    };
    // amdgpu
    g.busy_pct = read_u64(d.join("gpu_busy_percent")).map(|v| v as f32);
    g.vram_used_bytes = read_u64(d.join("mem_info_vram_used"));
    g.vram_total_bytes = read_u64(d.join("mem_info_vram_total"));
    g.clock_mhz = read(d.join("pp_dpm_sclk")).and_then(|s| active_dpm_clock(&s));
    // i915 / xe
    if g.clock_mhz.is_none() {
        g.clock_mhz = read(c.path.join("gt_act_freq_mhz"))
            .or_else(|| read(d.join("tile0/gt0/freq0/act_freq")))
            .and_then(|s| s.parse().ok())
            .filter(|&f: &u32| f > 0);
    }
    if let Some(hw) = hwmon(d) {
        g.temp_c = read_u64(hw.join("temp1_input")).map(|m| m as f32 / 1000.0);
        g.power_w = read_u64(hw.join("power1_average"))
            .or_else(|| read_u64(hw.join("power1_input")))
            .map(|uw| uw as f32 / 1_000_000.0);
    }
    g
}

/// One row per GPU keyed by lower-case PCI bus id.
fn nvidia_smi() -> HashMap<String, GpuInfo> {
    let out = Command::new("nvidia-smi")
        .args([
            "--query-gpu=pci.bus_id,name,utilization.gpu,memory.used,memory.total,temperature.gpu,power.draw,clocks.gr",
            "--format=csv,noheader,nounits",
        ])
        .output();
    let Ok(out) = out else {
        return HashMap::new();
    };
    parse_nvidia_smi(&String::from_utf8_lossy(&out.stdout))
}

pub(crate) fn parse_nvidia_smi(s: &str) -> HashMap<String, GpuInfo> {
    let num = |v: &str| v.trim().parse::<f32>().ok();
    s.lines()
        .filter_map(|l| {
            let f: Vec<&str> = l.split(',').map(str::trim).collect();
            if f.len() < 8 {
                return None;
            }
            // nvidia-smi prints 00000000:01:00.0; sysfs uses 0000:01:00.0.
            let bus = f[0].to_ascii_lowercase();
            let bus = if bus.len() > 12 {
                bus[bus.len() - 12..].to_string()
            } else {
                bus
            };
            let mib = |v: &str| num(v).map(|m| (m as u64) * 1024 * 1024);
            Some((
                bus,
                GpuInfo {
                    name: f[1].to_string(),
                    driver: "nvidia".into(),
                    busy_pct: num(f[2]),
                    vram_used_bytes: mib(f[3]),
                    vram_total_bytes: mib(f[4]),
                    temp_c: num(f[5]),
                    power_w: num(f[6]),
                    clock_mhz: num(f[7]).map(|c| c as u32),
                },
            ))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_gpus_from_the_pci_database() {
        let db = "# comment\n1002  Advanced Micro Devices, Inc. [AMD/ATI]\n\t744c  Navi 31 [Radeon RX 7900 XT/7900 XTX]\n\t\t1002 0e3b  sub\n10de  NVIDIA Corporation\n\t2684  AD102 [GeForce RTX 4090]\n";
        assert_eq!(
            pci_name_from(db, 0x1002, 0x744c).as_deref(),
            Some("AMD Radeon RX 7900 XT/7900 XTX")
        );
        assert_eq!(
            pci_name_from(db, 0x10de, 0x2684).as_deref(),
            Some("NVIDIA GeForce RTX 4090")
        );
        assert_eq!(pci_name_from(db, 0x10de, 0x1111), None);
    }

    #[test]
    fn reads_active_amd_clock() {
        assert_eq!(
            active_dpm_clock("0: 500Mhz\n1: 1800Mhz *\n2: 2500Mhz\n"),
            Some(1800)
        );
        assert_eq!(active_dpm_clock("0: 500Mhz\n"), None);
    }

    #[test]
    fn parses_nvidia_smi_rows() {
        let rows = parse_nvidia_smi(
            "00000000:01:00.0, NVIDIA GeForce RTX 4090, 37, 2048, 24564, 52, 85.40, 2520\n",
        );
        let g = &rows["0000:01:00.0"];
        assert_eq!(g.busy_pct, Some(37.0));
        assert_eq!(g.vram_total_bytes, Some(24564 * 1024 * 1024));
        assert_eq!(g.temp_c, Some(52.0));
        assert_eq!(g.clock_mhz, Some(2520));
    }

    #[test]
    fn collector_runs_anywhere() {
        let c = GpuCollector::new();
        assert_eq!(c.sample().len(), c.cards.len());
    }
}
