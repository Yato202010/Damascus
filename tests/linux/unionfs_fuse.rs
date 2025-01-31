// Copyright 2025 Yato202010
//
// Licensed under the Apache License, Version 2.0 (the "License"); you may not use this file except in compliance with the License. You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software distributed under the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the License for the specific language governing permissions and limitations under the License.
use crate::skip;

use super::{execute_test, read_only_test, read_test, write_test};
use damascus::{Filesystem, UnionFsFuse};
use nix::unistd::geteuid;
use std::fs::create_dir_all;
use temp_testdir::TempDir;

pub fn mount_unionfs_fuse_r() {
    if !UnionFsFuse::is_available() {
        skip!("UnionFsFuse is not available");
        return;
    }
    if geteuid().is_root() {
        skip!("fuse mount can't be tested as root");
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
    let mut o = UnionFsFuse::readonly([&lower1, &lower2].iter(), &target).unwrap();
    o.mount().unwrap();

    read_only_test(&test);
}

pub fn mount_unionfs_fuse_rw() {
    if !UnionFsFuse::is_available() {
        skip!("UnionFsFuse is not available");
        return;
    }
    if geteuid().is_root() {
        skip!("fuse mount can't be tested as root");
        return;
    }
    let tmp = TempDir::default().to_path_buf();
    let lower1 = tmp.join("lower1");
    let lower2 = tmp.join("lower2");
    let upper = tmp.join("upper");
    let target = tmp.join("mount");
    let test = target.join("test");
    create_dir_all(&lower1).unwrap();
    create_dir_all(&lower2).unwrap();
    create_dir_all(&target).unwrap();
    create_dir_all(&upper).unwrap();
    let mut o = UnionFsFuse::writable([lower1, lower2].iter(), &upper, &target).unwrap();
    o.mount().unwrap();

    write_test(&test);

    read_test(&test);

    execute_test(&test);
}

pub fn mount_unionfs_fuse_rw_on_lower() {
    if !UnionFsFuse::is_available() {
        skip!("UnionFsFuse is not available");
        return;
    }
    if geteuid().is_root() {
        skip!("fuse mount can't be tested as root");
        return;
    }
    let tmp = TempDir::default().to_path_buf();
    let lower1 = tmp.join("lower1");
    let lower2 = tmp.join("lower2");
    let upper = tmp.join("upper");
    let target = lower1.clone();
    let test = target.join("test");
    create_dir_all(&lower1).unwrap();
    create_dir_all(&lower2).unwrap();
    create_dir_all(&target).unwrap();
    create_dir_all(&upper).unwrap();
    let mut o = UnionFsFuse::writable([lower1, lower2].iter(), upper, target).unwrap();
    o.mount().unwrap();

    write_test(&test);

    read_test(&test);

    execute_test(&test);
}
