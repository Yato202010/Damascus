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
