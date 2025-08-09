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
            use windows::Win32::Storage::FileSystem::GetVolumeInformationA;

            let mut lpvolumeserialnumber = u32::MAX;

            // change current working directory to avoid to use lproothpathname which i can't make
            // it work
            let current_dir = std::env::current_dir()?;
            std::env::set_current_dir(path)?;

            unsafe {
                GetVolumeInformationA(
                    None,
                    None,
                    Some(&mut lpvolumeserialnumber as *mut u32),
                    None,
                    None,
                    None,
                )?;
            }

            // restore current working dir
            std::env::set_current_dir(current_dir)?;

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

    use std::fs::create_dir_all;

    use super::*;

    #[test]
    fn try_from() {
        let temp_dir = std::env::temp_dir();
        dbg!(&temp_dir);
        #[cfg(target_os = "windows")]
        {
            create_dir_all(&temp_dir).unwrap();
        }
        let id = PartitionID::try_from(temp_dir.as_path()).unwrap();
        #[cfg(target_os = "linux")]
        {
            assert_ne!(id, PartitionID(0));
        }
        #[cfg(target_os = "windows")]
        {
            assert_ne!(id, PartitionID(0 >> 16));
        }
    }
}
