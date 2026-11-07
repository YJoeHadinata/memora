use std::ffi::CStr;

use crate::models::{DiskVolume, StorageSnapshot};
use crate::system::storage::StorageCollector;

const ATTR_BIT_MAP_COUNT: u16 = 5;
const ATTR_VOL_INFO: u32 = 0x8000_0000;
const ATTR_VOL_SPACEUSED: u32 = 0x0080_0000;
const HIDDEN_MOUNTS: &[&str] = &[
    "/System/Volumes/Hardware",
    "/System/Volumes/xarts",
    "/System/Volumes/iSCPreboot",
    "/System/Volumes/Update",
];

fn query_volume_space_used(mount_point: &CStr) -> Option<u64> {
    let mut attrlist = libc::attrlist {
        bitmapcount: ATTR_BIT_MAP_COUNT,
        reserved: 0,
        commonattr: 0,
        volattr: ATTR_VOL_INFO | ATTR_VOL_SPACEUSED,
        dirattr: 0,
        fileattr: 0,
        forkattr: 0,
    };

    let mut buf = [0u8; 12];
    let ret = unsafe {
        libc::getattrlist(
            mount_point.as_ptr(),
            &mut attrlist as *mut libc::attrlist as *mut libc::c_void,
            buf.as_mut_ptr() as *mut libc::c_void,
            buf.len(),
            0,
        )
    };

    if ret == 0 && u32::from_ne_bytes([buf[0], buf[1], buf[2], buf[3]]) >= 12 {
        return Some(u64::from_ne_bytes([
            buf[4], buf[5], buf[6], buf[7], buf[8], buf[9], buf[10], buf[11],
        ]));
    }
    None
}

pub struct MacosStorageCollector;

impl MacosStorageCollector {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MacosStorageCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl StorageCollector for MacosStorageCollector {
    fn collect_storage(&self) -> Result<StorageSnapshot, String> {
        let mut mntbuf: *mut libc::statfs = std::ptr::null_mut();
        let count = unsafe { libc::getmntinfo(&mut mntbuf, libc::MNT_NOWAIT) };
        if count <= 0 || mntbuf.is_null() {
            return Err("Failed to retrieve mounted volumes from getmntinfo".to_string());
        }

        let mut volumes = Vec::new();
        let entries = unsafe { std::slice::from_raw_parts(mntbuf, count as usize) };

        for stat in entries {
            let mnt_on_cstr = unsafe { CStr::from_ptr(stat.f_mntonname.as_ptr()) };
            let mnt_on = mnt_on_cstr.to_string_lossy().to_string();
            let mnt_from = unsafe { CStr::from_ptr(stat.f_mntfromname.as_ptr()) }
                .to_string_lossy()
                .to_string();
            let fstype = unsafe { CStr::from_ptr(stat.f_fstypename.as_ptr()) }
                .to_string_lossy()
                .to_string();

            if fstype == "devfs"
                || fstype == "autofs"
                || mnt_on.starts_with("/dev")
                || HIDDEN_MOUNTS.contains(&mnt_on.as_str())
            {
                continue;
            }

            let bsize = stat.f_bsize as u64;
            let total_bytes = stat.f_blocks * bsize;
            if total_bytes == 0 {
                continue;
            }
            let free_bytes = stat.f_bfree * bsize;
            let available_bytes = stat.f_bavail * bsize;
            let used_bytes = query_volume_space_used(mnt_on_cstr)
                .unwrap_or_else(|| total_bytes.saturating_sub(free_bytes));

            let is_system = matches!(
                mnt_on.as_str(),
                "/" | "/System/Volumes/Data" | "/System/Volumes/VM"
            );
            let is_removable = mnt_on.starts_with("/Volumes/") && mnt_on != "/Volumes/Recovery";

            let name = match mnt_on.as_str() {
                "/System/Volumes/Data" => "Macintosh HD - Data".to_string(),
                "/" => "Macintosh HD - System".to_string(),
                "/System/Volumes/VM" => "macOS VM (Swap Volume)".to_string(),
                "/System/Volumes/Preboot" => "macOS Preboot".to_string(),
                _ => mnt_on
                    .strip_prefix("/Volumes/")
                    .map_or_else(|| mnt_on.clone(), String::from),
            };

            volumes.push(DiskVolume {
                name,
                mount_point: mnt_on,
                device_path: mnt_from,
                filesystem_type: fstype,
                total_bytes,
                used_bytes,
                available_bytes,
                is_system,
                is_removable,
            });
        }

        volumes.sort_by_key(|v| match v.mount_point.as_str() {
            "/System/Volumes/Data" => 0,
            _ if v.is_removable => 1,
            "/" => 2,
            "/System/Volumes/VM" => 3,
            _ => 4,
        });

        let primary_volume = volumes
            .iter()
            .find(|v| v.mount_point == "/System/Volumes/Data")
            .cloned()
            .or_else(|| volumes.iter().find(|v| v.mount_point == "/").cloned());

        let swap_headroom_bytes = primary_volume.as_ref().map_or(0, |v| v.available_bytes);
        let swap_headroom_healthy = swap_headroom_bytes > 20 * 1024 * 1024 * 1024;

        Ok(StorageSnapshot {
            volumes,
            primary_volume,
            swap_headroom_bytes,
            swap_headroom_healthy,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_macos_storage_collection() {
        let collector = MacosStorageCollector::new();
        let snapshot = collector.collect_storage().expect("Should collect storage");
        assert!(!snapshot.volumes.is_empty());
        let primary = snapshot
            .primary_volume
            .expect("Should identify primary volume");
        assert!(primary.total_bytes > 0 && primary.used_bytes > 0);
    }
}
