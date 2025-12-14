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
    use std::{ffi::CStr, io::Result, path::Path};

    use nix::libc::{getmntent, setmntent};

    use crate::AsPath;

    #[derive(Debug)]
    pub struct FsData {
        options: Vec<String>,
    }

    impl FsData {
        pub fn options(&self) -> &[String] {
            &self.options
        }
    }

    /// Retrieve filesystem data from system information
    pub(crate) fn restore_fsdata<P: AsRef<Path>>(path: P) -> Result<Option<FsData>> {
        let fd = unsafe { setmntent(c"/etc/mtab".as_ptr(), "r".as_ptr() as *const i8) };
        if fd.is_null() {
            return Err(std::io::Error::other("Cannot setmntent"));
        }

        let mut cont = true;
        while cont {
            let fs = unsafe { getmntent(fd).as_ref() };
            if let Some(fs) = fs {
                let target = unsafe { CStr::from_ptr(fs.mnt_dir) };
                if target.as_path() == path.as_ref() {
                    let opts = unsafe { CStr::from_ptr(fs.mnt_opts) };
                    let option = opts
                        .to_string_lossy()
                        .split(',')
                        .map(|x| x.to_string())
                        .collect();
                    return Ok(Some(FsData { options: option }));
                }
            } else {
                cont = false
            }
        }
        Ok(None)
    }
}
