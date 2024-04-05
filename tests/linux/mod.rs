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

#[cfg(feature = "fuse-overlayfs")]
pub mod test_mount_fuse_overlay {
    use super::{execute_test, read_only_test, read_test, write_test};
    use damascus::{Filesystem, FuseOverlayFs};
    use nix::unistd::geteuid;
    use std::{
        fs::create_dir_all,
        io::{self, Write},
    };
    use temp_testdir::TempDir;

    pub fn init_fuse_overlay_r() {
        let tmp = TempDir::default().to_path_buf();
        let _ = FuseOverlayFs::readonly([tmp.join("lower")].iter(), tmp.join("mount"));
    }

    pub fn init_fuse_overlay_rw() {
        let tmp = TempDir::default().to_path_buf();
        let _ = FuseOverlayFs::writable(
            [tmp.join("lower")].iter(),
            tmp.join("upper"),
            tmp.join("work"),
            tmp.join("mount"),
        );
    }

    pub fn mount_fuse_overlay_r() {
        if geteuid().is_root() {
            let stderr = io::stderr();
            let mut handle = stderr.lock();
            writeln!(handle, "\nWarning: fuse mount can't be tested as root\n").unwrap();
            return;
        }
        let tmp = TempDir::default().to_path_buf();
        let lower1 = tmp.join("lower1");
        let lower2 = tmp.join("lower2");
        let target = tmp.join("mount");
        let test = target.join("test");
        create_dir_all(&lower1).unwrap();
        create_dir_all(&lower2).unwrap();
        create_dir_all(&target).unwrap();
        let mut o = FuseOverlayFs::readonly([&lower1, &lower2].iter(), &target).unwrap();
        o.mount().unwrap();

        read_only_test(&test);
    }

    pub fn mount_fuse_overlay_rw() {
        if geteuid().is_root() {
            let stderr = io::stderr();
            let mut handle = stderr.lock();
            writeln!(handle, "\nWarning: fuse mount can't be tested as root\n").unwrap();
            return;
        }
        let tmp = TempDir::default().to_path_buf();
        let lower1 = tmp.join("lower1");
        let lower2 = tmp.join("lower2");
        let upper = tmp.join("upper");
        let work = tmp.join("work");
        let target = tmp.join("mount");
        let test = target.join("test");
        create_dir_all(&lower1).unwrap();
        create_dir_all(&lower2).unwrap();
        create_dir_all(&target).unwrap();
        create_dir_all(&upper).unwrap();
        create_dir_all(&work).unwrap();
        let mut o =
            FuseOverlayFs::writable([lower1, lower2].iter(), &upper, &work, &target).unwrap();
        o.mount().unwrap();

        write_test(&test);

        read_test(&test);

        execute_test(&test);
    }

    pub fn mount_fuse_overlay_rw_on_lower() {
        if geteuid().is_root() {
            let stderr = io::stderr();
            let mut handle = stderr.lock();
            writeln!(handle, "\nWarning: fuse mount can't be tested as root\n").unwrap();
            return;
        }
        let tmp = TempDir::default().to_path_buf();
        let lower1 = tmp.join("lower1");
        let lower2 = tmp.join("lower2");
        let upper = tmp.join("upper");
        let work = tmp.join("work");
        let target = lower1.clone();
        let test = target.join("test");
        create_dir_all(&lower1).unwrap();
        create_dir_all(&lower2).unwrap();
        create_dir_all(&target).unwrap();
        create_dir_all(&upper).unwrap();
        create_dir_all(&work).unwrap();
        let mut o = FuseOverlayFs::writable([lower1, lower2].iter(), upper, work, target).unwrap();
        o.mount().unwrap();

        write_test(&test);

        read_test(&test);

        execute_test(&test);
    }
}

#[cfg(feature = "overlayfs")]
pub mod test_mount_overlay {
    use super::{execute_test, read_only_test, read_test, setup_namespaces, write_test};
    use damascus::{Filesystem, OverlayFs};
    use nix::unistd::geteuid;
    use std::{
        fs::create_dir_all,
        io::{self, Write},
    };
    use temp_testdir::TempDir;

    pub fn init_overlay_r() {
        let tmp = TempDir::default().to_path_buf();
        let _ = OverlayFs::readonly([tmp.join("lower")].iter(), tmp.join("mount"));
    }

    pub fn init_overlay_rw() {
        let tmp = TempDir::default().to_path_buf();
        let _ = OverlayFs::writable(
            [tmp.join("lower")].iter(),
            tmp.join("upper"),
            tmp.join("work"),
            tmp.join("mount"),
        );
    }

    pub fn mount_overlay_r() {
        if !geteuid().is_root() {
            setup_namespaces();
        }
        let tmp = TempDir::default().to_path_buf();
        let lower1 = tmp.join("lower1");
        let lower2 = tmp.join("lower2");
        let target = tmp.join("mount");
        let test = target.join("test");
        create_dir_all(&lower1).unwrap();
        create_dir_all(&lower2).unwrap();
        create_dir_all(&target).unwrap();
        let mut o = OverlayFs::readonly([&lower1, &lower2].iter(), &target).unwrap();
        o.mount().unwrap();

        read_only_test(&test);
    }

    pub fn mount_overlay_rw() {
        if !geteuid().is_root() {
            let stderr = io::stderr();
            let mut handle = stderr.lock();
            writeln!(
                handle,
                "\nWarning: rw mount can only be tested as root on tmpfs\n"
            )
            .unwrap();
            return;
        }
        let tmp = TempDir::default().to_path_buf();
        let lower1 = tmp.join("lower1");
        let lower2 = tmp.join("lower2");
        let upper = tmp.join("upper");
        let work = tmp.join("work");
        let target = tmp.join("mount");
        let test = target.join("test");
        create_dir_all(&lower1).unwrap();
        create_dir_all(&lower2).unwrap();
        create_dir_all(&target).unwrap();
        create_dir_all(&upper).unwrap();
        create_dir_all(&work).unwrap();
        let mut o = OverlayFs::writable([lower1, lower2].iter(), &upper, &work, &target).unwrap();
        o.mount().unwrap();

        write_test(&test);

        read_test(&test);

        execute_test(&test);
    }

    pub fn mount_overlay_rw_on_lower() {
        if !geteuid().is_root() {
            let stderr = io::stderr();
            let mut handle = stderr.lock();
            writeln!(
                handle,
                "\nWarning: rw mount can only be tested as root on tmpfs\n"
            )
            .unwrap();
            return;
        }
        let tmp = TempDir::default().to_path_buf();
        let lower1 = tmp.join("lower1");
        let lower2 = tmp.join("lower2");
        let upper = tmp.join("upper");
        let work = tmp.join("work");
        let target = lower1.clone();
        let test = target.join("test");
        create_dir_all(&lower1).unwrap();
        create_dir_all(&lower2).unwrap();
        create_dir_all(&target).unwrap();
        create_dir_all(&upper).unwrap();
        create_dir_all(&work).unwrap();
        let mut o = OverlayFs::writable([lower1, lower2].iter(), upper, work, target).unwrap();
        o.mount().unwrap();

        write_test(&test);

        read_test(&test);

        execute_test(&test);
    }
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
