// Copyright 2025 Yato202010
//
// Licensed under the Apache License, Version 2.0 (the "License"); you may not use this file except in compliance with the License. You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software distributed under the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the License for the specific language governing permissions and limitations under the License.
use std::path::Path;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
/// Representation of a partition unique identifier
pub struct PartitionID(
    /// Partition dev id
    #[cfg(target_family = "unix")]
    u64,
    /// Partition volume serial number
    #[cfg(target_os = "windows")]
    u32,
);

impl TryFrom<&Path> for PartitionID {
    type Error = std::io::Error;

    fn try_from(path: &Path) -> Result<Self, Self::Error> {
        if !path.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "path does not exist",
            ));
        }

        #[cfg(target_family = "unix")]
        {
            use std::os::unix::fs::MetadataExt;
            Ok(PartitionID(std::fs::metadata(path)?.dev()))
        }

        #[cfg(target_os = "windows")]
        {
            use crate::os::OsStrExt;
            let lpvolumeserialnumber = unsafe {
                extern crate windows as win;
                use std::ptr;
                use win::{
                    core::PCSTR,
                    Win32::{
                        Foundation::HANDLE,
                        Storage::FileSystem::{
                            CreateFileA, GetVolumeInformationByHandleW, FILE_ATTRIBUTE_NORMAL,
                            FILE_GENERIC_READ, FILE_GENERIC_WRITE, FILE_SHARE_READ, OPEN_EXISTING,
                        },
                    },
                };

                let path_str = path.as_os_str().as_bytes();
                let path_wstr = PCSTR::from_raw(path_str.as_ptr());

                let handle = CreateFileA(
                    path_wstr,
                    FILE_GENERIC_READ.0 | FILE_GENERIC_WRITE.0,
                    FILE_SHARE_READ,
                    None,
                    OPEN_EXISTING,
                    FILE_ATTRIBUTE_NORMAL,
                    HANDLE(0 as _),
                )?;
                let lpvolumeserialnumber = ptr::null_mut();
                GetVolumeInformationByHandleW(
                    handle,
                    None,
                    Some(lpvolumeserialnumber),
                    None,
                    None,
                    None,
                )?;
                (*lpvolumeserialnumber).into()
            };

            // TODO : move to safe alternative once into rust stable
            //
            // use std::os::windows::fs::MetadataExt;
            // let lpvolumeserialnumber = std::fs::metadata(path)?
            //     .volume_serial_number().unwrap_unchecked();
            Ok(PartitionID(lpvolumeserialnumber))
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn try_from() {
        #[cfg(target_family = "unix")]
        {
            let id = PartitionID::try_from(Path::new("/tmp/")).unwrap();
            assert_ne!(id, PartitionID(0));
        }
        #[cfg(target_os = "windows")]
        {
            let id = PartitionID::try_from(Path::new("C://User")).unwrap();
            assert_ne!(id, PartitionID(0));
        }
    }
}
