// Copyright 2025 Yato202010
//
// Licensed under the Apache License, Version 2.0 (the "License"); you may not use this file except in compliance with the License. You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software distributed under the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the License for the specific language governing permissions and limitations under the License.
use std::{
    ffi::{CStr, CString, OsStr},
    path::{Path, PathBuf},
};

/* Linux */
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
#[allow(unused_imports)]
pub use linux::*;
#[cfg(target_family = "unix")]
pub(crate) use std::os::unix::ffi::OsStrExt;

/* Windows */
#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
#[allow(unused_imports)]
pub use windows::*;
#[cfg(target_os = "windows")]
#[allow(dead_code)]
pub(crate) trait OsStrExt {
    fn from_bytes(b: &[u8]) -> &Self;
    fn as_bytes(&self) -> &[u8];
}
#[cfg(target_os = "windows")]
impl OsStrExt for OsStr {
    fn from_bytes(b: &[u8]) -> &Self {
        // On Windows, OsStr is UTF-16 encoded. The bytes passed here are UTF-8
        // (from CStr/CString), so we convert via lossy string to get a valid OsStr.
        // This avoids the unsound transmute that assumed UTF-8 == UTF-16.
        use std::borrow::Cow;
        match std::ffi::OsString::from_encoded_bytes(b.to_vec()) {
            Ok(os) => Cow::Owned(os).as_ref(),
            Err(_) => {
                // Fallback: treat as UTF-8 lossy
                let s = String::from_utf8_lossy(b);
                Cow::Borrowed(OsStr::new(s.as_ref())).as_ref()
            }
        }
    }
    fn as_bytes(&self) -> &[u8] {
        self.as_encoded_bytes()
    }
}

/* Macos */
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
#[allow(unused_imports)]
pub use macos::*;

#[allow(dead_code)]
pub(crate) trait AsPath {
    fn as_path(&self) -> &Path;
}

impl AsPath for CString {
    fn as_path(&self) -> &Path {
        OsStr::from_bytes(self.to_bytes()).as_ref()
    }
}

impl AsPath for CStr {
    fn as_path(&self) -> &Path {
        OsStr::from_bytes(self.to_bytes()).as_ref()
    }
}

#[allow(dead_code)]
pub(crate) trait AsCString {
    fn as_cstring(&self) -> std::io::Result<CString>;
}

impl AsCString for Path {
    fn as_cstring(&self) -> std::io::Result<CString> {
        let int = {
            #[cfg(target_family = "unix")]
            {
                self.as_os_str().as_bytes()
            }
            #[cfg(target_os = "windows")]
            {
                self.to_string_lossy().to_string()
            }
        };

        CString::new(int).map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "path contains null bytes")
        })
    }
}

impl AsCString for PathBuf {
    fn as_cstring(&self) -> std::io::Result<CString> {
        self.as_path().as_cstring()
    }
}
