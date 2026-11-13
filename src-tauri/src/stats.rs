//! CPU, memory, disk and network throughput for the System widget.

use serde::Serialize;
use std::time::Instant;
use windows::core::HSTRING;
use windows::Win32::Foundation::FILETIME;
use windows::Win32::NetworkManagement::IpHelper::{FreeMibTable, GetIfTable2, MIB_IF_TABLE2};
use windows::Win32::NetworkManagement::Ndis::IfOperStatusUp;
use windows::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
use windows::Win32::System::Threading::GetSystemTimes;

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Stats {
    /// Percent of all cores, 0-100.
    pub cpu: f64,
    pub memory_used: u64,
    pub memory_total: u64,
    pub disk_free: u64,
    pub disk_total: u64,
    /// Bytes per second over physical network adapters.
    pub net_down: f64,
    pub net_up: f64,
}

/// Keeps the previous reading: CPU and network rates are differences between samples.
#[derive(Default)]
pub struct Sampler {
    last: Option<(Instant, u64, u64, u64, u64)>,
}

fn ticks(t: FILETIME) -> u64 {
    (u64::from(t.dwHighDateTime) << 32) | u64::from(t.dwLowDateTime)
}

impl Sampler {
    pub fn sample(&mut self) -> Stats {
        let now = Instant::now();
        let (mut idle, mut kernel, mut user) = (FILETIME::default(), FILETIME::default(), FILETIME::default());
        unsafe {
            let _ = GetSystemTimes(Some(&mut idle), Some(&mut kernel), Some(&mut user));
        }
        // Kernel time includes idle time.
        let (idle, busy_and_idle) = (ticks(idle), ticks(kernel) + ticks(user));
        let (down, up) = network_bytes();
        let mut stats = Stats::default();
        if let Some((then, idle0, total0, down0, up0)) = self.last {
            let seconds = now.duration_since(then).as_secs_f64().max(0.001);
            let total = busy_and_idle.saturating_sub(total0) as f64;
            if total > 0.0 {
                stats.cpu = (100.0 * (1.0 - idle.saturating_sub(idle0) as f64 / total)).clamp(0.0, 100.0);
            }
            stats.net_down = down.saturating_sub(down0) as f64 / seconds;
            stats.net_up = up.saturating_sub(up0) as f64 / seconds;
        }
        self.last = Some((now, idle, busy_and_idle, down, up));

        let mut memory = MEMORYSTATUSEX { dwLength: size_of::<MEMORYSTATUSEX>() as u32, ..Default::default() };
        if unsafe { GlobalMemoryStatusEx(&mut memory) }.is_ok() {
            stats.memory_total = memory.ullTotalPhys;
            stats.memory_used = memory.ullTotalPhys.saturating_sub(memory.ullAvailPhys);
        }
        let drive = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".into());
        let (mut free, mut total) = (0u64, 0u64);
        if unsafe { GetDiskFreeSpaceExW(&HSTRING::from(format!("{drive}\\")), Some(&mut free), Some(&mut total), None) }.is_ok() {
            stats.disk_free = free;
            stats.disk_total = total;
        }
        stats
    }
}

/// Total bytes received and sent by physical adapters that are up (virtual switches and
/// loopback would count the same traffic twice).
fn network_bytes() -> (u64, u64) {
    let mut table: *mut MIB_IF_TABLE2 = std::ptr::null_mut();
    unsafe {
        if GetIfTable2(&mut table).is_err() || table.is_null() {
            return (0, 0);
        }
        let rows = std::slice::from_raw_parts((*table).Table.as_ptr(), (*table).NumEntries as usize);
        let (mut down, mut up) = (0, 0);
        for row in rows {
            let hardware = row.InterfaceAndOperStatusFlags._bitfield & 1 != 0;
            if hardware && row.OperStatus == IfOperStatusUp {
                down += row.InOctets;
                up += row.OutOctets;
            }
        }
        FreeMibTable(table as *const _);
        (down, up)
    }
}
