use core::panic;
use std::{
    fs::{self, File},
    io::{Read, Write},
    os::unix::prelude::OpenOptionsExt,
    path::Path,
    process::Command,
    sync::Once,
};

use nix::{
    errno::Errno,
    sched::{unshare, CloneFlags},
    sys::stat::Mode,
    unistd::getuid,
};

#[allow(unused_imports)]
use crate::register_tests;

#[cfg(feature = "unionfs-fuse")]
pub mod unionfs_fuse;

#[cfg(feature = "fuse-overlayfs")]
pub mod fuse_overlayfs;

#[cfg(feature = "overlayfs")]
pub mod overlayfs;

pub fn register_test() {
    #[cfg(feature = "unionfs-fuse")]
    register_tests!(
        // WARN : currently this filesystem return Ernno::PermissionDenierd = 13 instead of
        // Ernno::EROFS = 30 which is clearly wrong but why !?
        unionfs_fuse::mount_unionfs_fuse_r,
        unionfs_fuse::mount_unionfs_fuse_rw,
        // WARN : mounting on top of lower dir is not permitted for now it freeze
        //unionfs_fuse::mount_unionfs_fuse_rw_on_lower
    );
    #[cfg(feature = "fuse-overlayfs")]
    register_tests!(
        fuse_overlayfs::mount_fuse_overlay_r,
        fuse_overlayfs::mount_fuse_overlay_rw,
        fuse_overlayfs::mount_fuse_overlay_rw_on_lower
    );
    #[cfg(feature = "overlayfs")]
    register_tests!(
        overlayfs::mount_overlay_r,
        overlayfs::mount_overlay_rw,
        overlayfs::mount_overlay_rw_on_lower,
        overlayfs::recover_overlay_ro_handle,
        overlayfs::recover_overlay_rw_handle
    );
}

fn write_test(path: &Path) {
    // Verify write.
    fs::OpenOptions::new()
        .create(true)
        .write(true)
        .mode((Mode::S_IRWXU | Mode::S_IRWXG | Mode::S_IRWXO).bits())
        .open(path)
        .map_err(|e| {
            panic!("open failed: {}", e);
        })
        .and_then(|mut f| f.write(SCRIPT_CONTENTS))
        .unwrap_or_else(|e| panic!("write failed: {}", e));
}

fn read_test(path: &Path) {
    // Verify read.
    let mut buf = Vec::new();
    File::open(path)
        .and_then(|mut f| f.read_to_end(&mut buf))
        .unwrap_or_else(|e| panic!("read failed: {}", e));
    assert_eq!(buf, SCRIPT_CONTENTS);
}

fn execute_test(path: &Path) {
    // Verify execute.
    assert_eq!(
        EXPECTED_STATUS,
        Command::new(path)
            .status()
            .unwrap_or_else(|e| panic!("exec failed: {}", e))
            .code()
            .unwrap_or_else(|| panic!("child killed by signal"))
    );
}

fn read_only_test(path: &Path) {
    // EROFS: Read-only file system
    let err = File::create(path);
    if err.is_ok() {
        panic!("filesystem is rw, ro was expected")
    }
    let raw_err = err.unwrap_err().raw_os_error().unwrap();
    assert_eq!(
        { Errno::EROFS as i32 },
        raw_err,
        "expected {} but got {}",
        Errno::EROFS,
        Errno::from_raw(raw_err)
    );
}

static SCRIPT_CONTENTS: &[u8] = b"#!/bin/sh
exit 23";

const EXPECTED_STATUS: i32 = 23;

static ONCE: Once = std::sync::Once::new();

fn setup_namespaces() -> Result<(), Errno> {
    let mut err = None;
    ONCE.call_once(|| {
        // Hold on to the uid in the parent namespace.
        let uid = getuid();

        if let Err(e) = unshare(CloneFlags::CLONE_NEWUSER.union(CloneFlags::CLONE_NEWNS)) {
            return err = Some(e);
        };
        // Map user as uid 1000.
        if let Err(e) = fs::OpenOptions::new()
            .write(true)
            .open("/proc/self/uid_map")
            .and_then(|mut f| f.write(format!("1000 {} 1\n", uid).as_bytes()))
        {
            err = Some(Errno::from_raw(e.raw_os_error().unwrap_or(-1)))
        }
    });
    if let Some(e) = err {
        Err(e)
    } else {
        Ok(())
    }
}
