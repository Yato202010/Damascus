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
///
/// NOTE : on windows this will return a u64 even if the FileSystem use u32
pub struct PartitionID(u64);

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
            use windows::Win32::Storage::FileSystem::GetVolumeInformationW;
            use windows_strings::HSTRING;

            let mut lpvolumeserialnumber = u32::MAX;
            unsafe {
                GetVolumeInformationW(
                    &HSTRING::from(path),
                    None,
                    Some(&mut lpvolumeserialnumber as *mut u32),
                    None,
                    None,
                    None,
                )?;
            }

            // TODO : move to safe alternative once into rust stable
            //
            // use std::os::windows::fs::MetadataExt;
            // let lpvolumeserialnumber = std::fs::metadata(path)?
            //     .volume_serial_number().unwrap_unchecked();
            Ok(PartitionID(lpvolumeserialnumber as u64))
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn try_from() {
        let temp_dir = std::env::temp_dir();
        dbg!(&temp_dir);
        let id = PartitionID::try_from(temp_dir.as_path()).unwrap();
        assert_ne!(id, PartitionID(0));
    }
}
