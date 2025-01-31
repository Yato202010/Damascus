// Copyright 2025 Yato202010
//
// Licensed under the Apache License, Version 2.0 (the "License"); you may not use this file except in compliance with the License. You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software distributed under the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the License for the specific language governing permissions and limitations under the License.
use std::{
    fs::{self, File},
    io::{self, Read, Write},
    os::windows::prelude::OpenOptionsExt,
    path::Path,
    process::{self, Command},
    sync::Once,
};

pub fn register_test() {}

fn write_test(path: &Path) {
    // Verify write.
    fs::OpenOptions::new()
        .create(true)
        .write(true)
        .open(path)
        .or_else(|e| {
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
        5 as i32,
        File::create(path).unwrap_err().raw_os_error().unwrap()
    );
}

static SCRIPT_CONTENTS: &[u8] = b"echo hello world";

const EXPECTED_STATUS: i32 = 23;
