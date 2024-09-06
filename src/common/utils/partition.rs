use std::{
    fs::File,
    io::{self, BufRead},
    path::Path,
};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
/// Representation of a partition unique identifier
pub enum PartitionID {
    #[cfg(target_family = "unix")]
    /// partition dev id
    Id(u64),
    #[cfg(target_os = "windows")]
    /// partition volume serial number
    Id(u32),
    Invalid,
}

impl<P> From<P> for PartitionID
where
    P: AsRef<Path>,
{
    /// Create a new PartitionID from a path
    #[inline]
    fn from(path: P) -> PartitionID {
        let path = path.as_ref();
        if !path.exists() {
            return PartitionID::Invalid;
        }

        #[cfg(target_family = "unix")]
        {
            use std::os::unix::fs::MetadataExt;
            PartitionID::Id(
                std::fs::metadata(path)
                    .expect("unable to get metadata")
                    .dev(),
            )
        }
        #[cfg(target_os = "windows")]
        {
            unsafe {
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

                let path_str = path.to_str().unwrap().as_bytes();
                let path_wstr = PCSTR::from_raw(path_str.as_ptr());

                let handle = CreateFileA(
                    path_wstr,
                    FILE_GENERIC_READ.0 | FILE_GENERIC_WRITE.0,
                    FILE_SHARE_READ,
                    None,
                    OPEN_EXISTING,
                    FILE_ATTRIBUTE_NORMAL,
                    HANDLE(0),
                )
                .unwrap();
                let lpvolumeserialnumber = ptr::null_mut();
                GetVolumeInformationByHandleW(
                    handle,
                    None,
                    Some(lpvolumeserialnumber),
                    None,
                    None,
                    None,
                )
                .unwrap();
                PartitionID::Id(*lpvolumeserialnumber)
            }

            // TODO : move to safe alternative once into rust stable
            //
            // use std::os::windows::fs::MetadataExt;
            // let dev = std::fs::metadata(path)
            //     .expect("unable to get metadata")
            //     .volume_serial_number()
            //     .unwrap("unable to get volume serial number");
            // Partition::Id(dev)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from() {
        #[cfg(target_family = "unix")]
        {
            let id = PartitionID::from("/tmp/");
            assert_ne!(id, PartitionID::Id(0));
        }
        #[cfg(target_os = "windows")]
        {
            let id = PartitionID::from("C://User");
            assert_ne!(id, PartitionID::Id(0));
        }
    }
}
