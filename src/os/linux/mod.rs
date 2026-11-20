// Copyright 2025 Yato202010
//
// Licensed under the Apache License, Version 2.0 (the "License"); you may not use this file except in compliance with the License. You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software distributed under the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the License for the specific language governing permissions and limitations under the License.
#[cfg(feature = "unionfs-fuse")]
pub mod unionfs_fuse;
#[cfg(feature = "unionfs-fuse")]
pub use unionfs_fuse::UnionFsFuse;
#[cfg(feature = "unionfs-fuse")]
pub use unionfs_fuse::UnionFsFuseOption;
#[cfg(feature = "fuse-overlayfs")]
pub mod fuseoverlay;
#[cfg(feature = "fuse-overlayfs")]
pub use fuseoverlay::FuseOverlayFs;
#[cfg(feature = "fuse-overlayfs")]
pub use fuseoverlay::FuseOverlayFsOption;
#[cfg(feature = "overlayfs")]
pub mod overlay;
#[cfg(feature = "overlayfs")]
pub use overlay::OverlayFs;
#[cfg(feature = "overlayfs")]
pub use overlay::OverlayFsOption;

#[allow(unused_imports)]
pub(crate) use recover_state::{FsData, restore_fsdata};

/// Provide utility to recover filesystem state from the information provided by the system
#[allow(dead_code)]
mod recover_state {
    use std::{io::Result, path::Path};

    #[derive(Debug)]
    pub struct FsData {
        options: Vec<String>,
    }

    impl FsData {
        pub fn options(&self) -> &[String] {
            &self.options
        }
    }

    /// Retrieve filesystem data from system information by reading /proc/self/mounts
    pub(crate) fn restore_fsdata<P: AsRef<Path>>(path: P) -> Result<Option<FsData>> {
        let path = path.as_ref();
        let content = std::fs::read_to_string("/proc/self/mounts")?;
        for line in content.lines() {
            let first_ws = match line.find(char::is_whitespace) {
                Some(pos) => pos,
                None => continue,
            };
            let mp_start = line[first_ws..]
                .find(|c: char| !c.is_whitespace())
                .map(|i| first_ws + i)
                .unwrap_or(first_ws + 1);
            let rest = &line[mp_start..];
            let mut fields_from_end = 0;
            let mut mp_end = rest.len();
            let bytes = rest.as_bytes();
            let mut i = bytes.len();
            while i > 0 && fields_from_end < 4 {
                if bytes[i - 1].is_ascii_whitespace() {
                    fields_from_end += 1;
                } else {
                    mp_end = i - 1;
                }
                i -= 1;
            }
            while mp_end > 0 && bytes[mp_end].is_ascii_whitespace() {
                mp_end -= 1;
            }
            let mountpoint_str = &rest[..=mp_end];
            let mountpoint = Path::new(mountpoint_str);
            if mountpoint == path {
                let after_mp = &rest[mp_end + 1..];
                let after_mp_fields: Vec<&str> = after_mp.split_whitespace().collect();
                let options = if after_mp_fields.len() >= 2 {
                    after_mp_fields[1]
                        .split(',')
                        .map(|x| x.to_string())
                        .collect()
                } else {
                    Vec::new()
                };
                return Ok(Some(FsData { options }));
            }
        }
        Ok(None)
    }
}
