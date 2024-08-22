use std::{
    fs::{self, File},
    io::{self, Read, Write},
    os::unix::prelude::OpenOptionsExt,
    path::Path,
    process::{self, Command},
    sync::Once,
};

use nix::{
    libc::EROFS,
    sched::{unshare, CloneFlags},
    sys::stat::Mode,
    unistd::getuid,
};

#[allow(unused_imports)]
use crate::register_tests;

#[cfg(feature = "fuse-overlayfs")]
pub mod fuse_overlayfs;

#[cfg(feature = "overlayfs")]
pub mod overlayfs;

pub fn register_test() {
    #[cfg(feature = "fuse-overlayfs")]
    use crate::linux::fuse_overlayfs::{
        init_fuse_overlay_r, init_fuse_overlay_rw, mount_fuse_overlay_r, mount_fuse_overlay_rw,
        mount_fuse_overlay_rw_on_lower,
    };
    #[cfg(feature = "overlayfs")]
    use crate::linux::overlayfs::{
        init_overlay_r, init_overlay_rw, mount_overlay_r, mount_overlay_rw,
        mount_overlay_rw_on_lower,
    };
    #[cfg(feature = "fuse-overlayfs")]
    register_tests!(
        init_fuse_overlay_r,
        init_fuse_overlay_rw,
        mount_fuse_overlay_r,
        mount_fuse_overlay_rw,
        mount_fuse_overlay_rw_on_lower
    );
    #[cfg(feature = "overlayfs")]
    register_tests!(
        init_overlay_r,
        init_overlay_rw,
        mount_overlay_r,
        mount_overlay_rw,
        mount_overlay_rw_on_lower
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
    assert_eq!(
        { EROFS },
        File::create(path).unwrap_err().raw_os_error().unwrap()
    );
}

static SCRIPT_CONTENTS: &[u8] = b"#!/bin/sh
exit 23";

const EXPECTED_STATUS: i32 = 23;

static ONCE: Once = std::sync::Once::new();

fn setup_namespaces() {
    ONCE.call_once(|| {
        // Hold on to the uid in the parent namespace.
        let uid = getuid();
        let stderr = io::stderr();
        let mut handle = stderr.lock();

        unshare(CloneFlags::CLONE_NEWUSER.union(CloneFlags::CLONE_NEWNS)).unwrap_or_else(|e| {
            writeln!(
                handle,
                "\nWarning: unshare failed: {}. Are unprivileged user namespaces available?",
                e
            )
            .unwrap();
            writeln!(handle, "mount is not being tested\n").unwrap();
            // Exit with success because not all systems support unprivileged user namespaces, and
            // that's not what we're testing for.
            process::exit(0);
        });

        // Map user as uid 1000.
        fs::OpenOptions::new()
            .write(true)
            .open("/proc/self/uid_map")
            .and_then(|mut f| f.write(format!("1000 {} 1\n", uid).as_bytes()))
            .unwrap_or_else(|e| panic!("could not write uid map: {}", e));
    });
}
