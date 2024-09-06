use crate::skip;

use super::{execute_test, read_only_test, read_test, setup_namespaces, write_test};
use damascus::{Filesystem, OverlayFs};
use nix::unistd::geteuid;
use std::fs::create_dir_all;
use temp_testdir::TempDir;

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
        skip!("rw mount can only be tested as root on tmpfs");
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
        skip!("rw mount can only be tested as root on tmpfs");
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
