#[cfg(feature = "unionfs-fuse")]
mod unionfs_fuse;
#[cfg(feature = "unionfs-fuse")]
pub use unionfs_fuse::UnionFsFuse;
#[cfg(feature = "fuse-overlayfs")]
mod fuseoverlay;
#[cfg(feature = "fuse-overlayfs")]
pub use fuseoverlay::FuseOverlayFs;
#[cfg(feature = "overlayfs")]
mod overlay;
#[cfg(feature = "overlayfs")]
pub use overlay::{option::*, OverlayFs};

pub use option::{FsOption, LinuxFilesystem, MountOption};
/// Provide utility to recover filesystem state from the information provided by the system
mod recover_state {
    use std::ffi::{CStr, CString};
    use std::io::Result;
    use std::path::Path;
    use std::str::FromStr;

    use nix::libc::{getmntent, setmntent};

    use crate::{AsPath, MountOption};

    use super::option::FsOption;

    #[derive(Debug)]
    pub struct FsData<O: FsOption> {
        option: Vec<MountOption<O>>,
    }

    pub fn restore_fsdata<P: AsRef<Path>, O: FsOption>(path: P) -> Result<Option<FsData<O>>> {
        let fd = unsafe {
            let mtab = CString::new("/etc/mtab").unwrap();
            setmntent(mtab.as_ptr(), "r".as_ptr() as *const i8)
        };
        if fd.is_null() {
            panic!("cannot setmntent :{}", std::io::Error::last_os_error())
        }

        let mut cont = true;
        while cont {
            let fs = unsafe { getmntent(fd).as_ref() };
            if let Some(fs) = fs {
                let target = unsafe { CStr::from_ptr(fs.mnt_dir) };
                if target.as_path() == path.as_ref() {
                    dbg!(target);
                    let opts = dbg!(unsafe { CStr::from_ptr(fs.mnt_opts) });
                    let option = opts
                        .to_string_lossy()
                        .split(',')
                        .map(|x| MountOption::from_str(x))
                        .filter(|x| x.is_ok())
                        .map(|x| x.unwrap())
                        .collect();
                    return Ok(Some(FsData { option }));
                }
            } else {
                cont = false
            }
        }
        Ok(None)
    }

    /*#[cfg(test)]
    #[cfg(feature = "fuse-overlayfs")]
    mod tests {
        use crate::os::linux::{
            fuseoverlay::option::FuseOverlayFsOption, recover_state::restore_fsdata,
        };

        #[test]
        fn get_mounts() {
            dbg!(restore_fsdata::<&str, FuseOverlayFsOption>("/tmp/rstest/mount/").unwrap());
            todo!()
        }
    }*/
}

mod option {
    use std::{fmt::Display, io::Result, str::FromStr};

    pub trait LinuxFilesystem<O>
    where
        O: FsOption,
    {
        fn set_option(&mut self, option: impl Into<MountOption<O>>) -> Result<()>;

        fn remove_option(&mut self, option: impl Into<MountOption<O>>) -> Result<()>;

        fn options(&self) -> &[MountOption<O>];
    }

    pub trait FsOption: Sized + Clone + Display + FromStr {
        fn defaults() -> Vec<Self>;
        fn incompatible(&self, other: &MountOption<Self>) -> bool;
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum MountOption<O: FsOption> {
        RW,
        RO,
        Suid(bool),
        FsSpecific(O),
    }

    impl<T: FsOption> MountOption<T> {
        pub fn defaults() -> Vec<Self> {
            let mut v: Vec<MountOption<T>> = vec![];
            let mut r = T::defaults();
            v.extend(r.iter_mut().map(|x| MountOption::FsSpecific(x.clone())));
            v
        }

        pub fn incompatible(&self, other: &MountOption<T>) -> bool {
            match self {
                MountOption::FsSpecific(o) => o.incompatible(other),
                MountOption::RW if matches!(other, MountOption::RO) => true,
                _ => false,
            }
        }
    }

    impl<T: FsOption> FromStr for MountOption<T> {
        type Err = std::io::Error;

        fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
            Ok(match s {
                "rw" => MountOption::RW,
                "ro" => MountOption::RO,
                "suid" => MountOption::Suid(true),
                "nosuid" => MountOption::Suid(false),
                _ => {
                    let res = T::from_str(s)
                        .map(|x| MountOption::FsSpecific(x))
                        .map_err(|_e| {
                            std::io::Error::new(
                                std::io::ErrorKind::Unsupported,
                                "Unsupported mount option",
                            )
                        });
                    return res;
                }
            })
        }
    }

    impl<T: FsOption> Display for MountOption<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let str = match self {
                MountOption::FsSpecific(o) => o.to_string(),
                Self::RW => "rw".to_owned(),
                Self::RO => "ro".to_owned(),
                Self::Suid(b) => if *b { "suid" } else { "nosuid" }.to_owned(),
            };
            write!(f, "{}", str)
        }
    }
}
