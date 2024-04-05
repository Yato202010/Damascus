use std::path::Path;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
/// Representation of a partition unique identifier
pub enum Partition {
    #[cfg(target_family = "unix")]
    /// partition dev id
    Id(u64),
    #[cfg(target_os = "windows")]
    /// partition volume serial number
    Id(u32),
    Invalid,
}

impl<P> From<P> for Partition
where
    P: AsRef<Path>,
{
    #[inline]
    fn from(path: P) -> Partition {
        let path = path.as_ref();
        if !path.exists() {
            return Partition::Invalid;
        }

        cfg_if::cfg_if! {
            if #[cfg(target_family="unix")] {
                use std::os::unix::fs::MetadataExt;
                Partition::Id(std::fs::metadata(path)
                    .expect("unable to get metadata")
                    .dev())
            } else if #[cfg(target_os = "windows")] {
                unsafe {
                    extern crate windows as win;
                    use std::ptr;
                    use win::{
                        Win32::{
                            Storage::FileSystem::{
                                GetVolumeInformationByHandleW,
                                CreateFileA,
                                FILE_ATTRIBUTE_NORMAL,
                                OPEN_EXISTING,
                                FILE_SHARE_READ,
                                FILE_GENERIC_READ,
                                FILE_GENERIC_WRITE
                            },
                            Foundation::HANDLE
                        },
                        core::PCSTR
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
                    ).unwrap();
                    let lpvolumeserialnumber = ptr::null_mut();
                        GetVolumeInformationByHandleW(
                            handle,
                            None,
                            Some(lpvolumeserialnumber),
                            None,
                            None,
                            None
                        ).unwrap();
                    Partition::Id(*lpvolumeserialnumber)
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from() {
        cfg_if::cfg_if! {
            if #[cfg(target_family="unix")] {
                let _ = Partition::from("/tmp/");
            } else if #[cfg(target_os = "windows")] {
                let _ = Partition::from("C://User");
            }
        }
    }
}
