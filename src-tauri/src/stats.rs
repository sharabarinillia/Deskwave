//! CPU, memory, disk and network throughput for the System widget.

use serde::Serialize;

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

#[cfg(windows)]
pub use win::Sampler;
#[cfg(target_os = "macos")]
pub use mac::Sampler;

/// CPU percentage and network rates are differences between two readings.
fn rates(stats: &mut Stats, seconds: f64, idle: u64, total: u64, down: u64, up: u64) {
    if total > 0 {
        stats.cpu = (100.0 * (1.0 - idle as f64 / total as f64)).clamp(0.0, 100.0);
    }
    stats.net_down = down as f64 / seconds;
    stats.net_up = up as f64 / seconds;
}

#[cfg(windows)]
mod win {
    use super::{rates, Stats};
    use std::time::Instant;
    use windows::core::HSTRING;
    use windows::Win32::Foundation::FILETIME;
    use windows::Win32::NetworkManagement::IpHelper::{FreeMibTable, GetIfTable2, MIB_IF_TABLE2};
    use windows::Win32::NetworkManagement::Ndis::IfOperStatusUp;
    use windows::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
    use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
    use windows::Win32::System::Threading::GetSystemTimes;

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
                let total = busy_and_idle.saturating_sub(total0);
                rates(&mut stats, seconds, idle.saturating_sub(idle0), total, down.saturating_sub(down0), up.saturating_sub(up0));
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
}

#[cfg(target_os = "macos")]
mod mac {
    use super::{rates, Stats};
    use std::ffi::{c_void, CStr};
    use std::mem::{size_of, zeroed};
    use std::time::Instant;

    extern "C" {
        // Declared here because libc marks its binding deprecated in favour of another crate.
        fn mach_host_self() -> libc::mach_port_t;
    }

    #[derive(Default)]
    pub struct Sampler {
        host: Option<libc::mach_port_t>,
        last: Option<(Instant, u64, u64, u64, u64)>,
    }

    impl Sampler {
        pub fn sample(&mut self) -> Stats {
            let now = Instant::now();
            let host = *self.host.get_or_insert_with(|| unsafe { mach_host_self() });
            let (idle, total) = cpu_ticks(host);
            let (down, up) = network_bytes();
            let mut stats = Stats::default();
            if let Some((then, idle0, total0, down0, up0)) = self.last {
                let seconds = now.duration_since(then).as_secs_f64().max(0.001);
                rates(&mut stats, seconds, idle.saturating_sub(idle0), total.saturating_sub(total0),
                    down.saturating_sub(down0), up.saturating_sub(up0));
            }
            self.last = Some((now, idle, total, down, up));
            (stats.memory_used, stats.memory_total) = memory(host);
            (stats.disk_free, stats.disk_total) = disk();
            stats
        }
    }

    /// (idle, all) CPU ticks since boot, summed over cores.
    fn cpu_ticks(host: libc::mach_port_t) -> (u64, u64) {
        let mut info: libc::host_cpu_load_info = unsafe { zeroed() };
        let mut count = libc::HOST_CPU_LOAD_INFO_COUNT;
        let ok = unsafe {
            libc::host_statistics(host, libc::HOST_CPU_LOAD_INFO, &mut info as *mut _ as libc::host_info_t, &mut count)
        } == libc::KERN_SUCCESS;
        if !ok {
            return (0, 0);
        }
        let ticks = info.cpu_ticks.map(u64::from);
        (ticks[libc::CPU_STATE_IDLE as usize], ticks.iter().sum())
    }

    /// (used, total) bytes. Used counts what Activity Monitor calls app memory, wired and
    /// compressed memory.
    fn memory(host: libc::mach_port_t) -> (u64, u64) {
        let mut total = 0u64;
        let mut size = size_of::<u64>();
        let name = c"hw.memsize";
        unsafe {
            libc::sysctlbyname(name.as_ptr(), &mut total as *mut u64 as *mut c_void, &mut size, std::ptr::null_mut(), 0);
        }
        let mut vm: libc::vm_statistics64 = unsafe { zeroed() };
        let mut count = libc::HOST_VM_INFO64_COUNT;
        let ok = unsafe {
            libc::host_statistics64(host, libc::HOST_VM_INFO64, &mut vm as *mut _ as libc::host_info64_t, &mut count)
        } == libc::KERN_SUCCESS;
        if !ok {
            return (0, total);
        }
        let page = unsafe { libc::vm_page_size } as u64;
        let pages = u64::from(vm.internal_page_count.saturating_sub(vm.purgeable_count))
            + u64::from(vm.wire_count)
            + u64::from(vm.compressor_page_count);
        ((pages * page).min(total), total)
    }

    /// (free, total) bytes on the volume that holds the user's data.
    fn disk() -> (u64, u64) {
        for path in [c"/System/Volumes/Data", c"/"] {
            let mut fs: libc::statfs = unsafe { zeroed() };
            if unsafe { libc::statfs(path.as_ptr(), &mut fs) } == 0 {
                let block = u64::from(fs.f_bsize);
                return (fs.f_bavail * block, fs.f_blocks * block);
            }
        }
        (0, 0)
    }

    /// Bytes received and sent over Ethernet and Wi-Fi (`en*`), so VPN tunnels and bridges
    /// don't count the same traffic twice.
    fn network_bytes() -> (u64, u64) {
        let mut list: *mut libc::ifaddrs = std::ptr::null_mut();
        if unsafe { libc::getifaddrs(&mut list) } != 0 {
            return (0, 0);
        }
        let (mut down, mut up) = (0u64, 0u64);
        let mut cursor = list;
        while let Some(ifa) = unsafe { cursor.as_ref() } {
            cursor = ifa.ifa_next;
            let up_flags = ifa.ifa_flags as libc::c_int;
            let link = unsafe { ifa.ifa_addr.as_ref() }.is_some_and(|a| libc::c_int::from(a.sa_family) == libc::AF_LINK);
            let name = unsafe { CStr::from_ptr(ifa.ifa_name) }.to_bytes();
            if !link || ifa.ifa_data.is_null() || up_flags & libc::IFF_UP == 0 || !name.starts_with(b"en") {
                continue;
            }
            let data = unsafe { &*(ifa.ifa_data as *const libc::if_data) };
            down += u64::from(data.ifi_ibytes);
            up += u64::from(data.ifi_obytes);
        }
        unsafe { libc::freeifaddrs(list) };
        (down, up)
    }
}
