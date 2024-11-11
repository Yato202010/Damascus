#[cfg(feature = "unionfs-fuse")]
pub mod unionfs_fuse;
#[cfg(feature = "unionfs-fuse")]
pub use unionfs_fuse::UnionFsFuse;
#[cfg(feature = "fuse-overlayfs")]
pub mod fuseoverlay;
#[cfg(feature = "fuse-overlayfs")]
pub use fuseoverlay::FuseOverlayFs;
#[cfg(feature = "overlayfs")]
pub mod overlay;
#[cfg(feature = "overlayfs")]
pub use overlay::OverlayFs;

pub use option::{FsOption, LinuxFilesystem, MountOption};

/// Provide utility to recover filesystem state from the information provided by the system
#[allow(dead_code)]
mod recover_state {
    use std::{ffi::CStr, io::Result, path::Path, str::FromStr};

    use nix::libc::{getmntent, setmntent};

    use crate::{AsPath, MountOption};

    use super::option::FsOption;

    #[derive(Debug)]
    pub struct FsData<O: FsOption> {
        options: Vec<MountOption<O>>,
    }

    impl<O: FsOption> FsData<O> {
        pub fn options(&self) -> &[MountOption<O>] {
            &self.options
        }
    }

    /// Retrieve filesystem data from system information
    pub fn restore_fsdata<P: AsRef<Path>, O: FsOption>(path: P) -> Result<Option<FsData<O>>> {
        let fd = unsafe {
            let mtab = CStr::from_bytes_with_nul_unchecked(b"/etc/mtab\0");
            setmntent(mtab.as_ptr(), "r".as_ptr() as *const i8)
        };
        if fd.is_null() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Cannot setmntent",
            ));
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
                        .flat_map(MountOption::from_str)
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

mod option {
    use std::{fmt::Display, io::Result, str::FromStr};

    pub trait LinuxFilesystem<O>
    where
        O: FsOption,
    {
        /// Set option
        /// will send error if another incompatible option is present
        fn set_option(&mut self, option: impl Into<MountOption<O>>) -> Result<()>;

        /// Remove an option
        fn remove_option(&mut self, option: impl Into<MountOption<O>>) -> Result<()>;

        /// List currently active option
        fn options(&self) -> &[MountOption<O>];
    }

    pub trait FsOption: Sized + Clone + Display + FromStr {
        /// Get defaults mount option for this filesystem
        fn defaults() -> Vec<Self>;
        /// Check if mount option is incompatible
        fn incompatible(&self, other: &MountOption<Self>) -> bool;
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum MountOption<O: FsOption> {
        RW,
        RO,
        Suid(bool),
        FsSpecific(O),
        Other(String),
    }

    impl<T: FsOption> MountOption<T> {
        /// Get defaults mount option for this filesystem
        pub fn defaults() -> Vec<Self> {
            let mut v: Vec<MountOption<T>> = vec![];
            let mut r = T::defaults();
            v.extend(r.iter_mut().map(|x| MountOption::FsSpecific(x.clone())));
            v
        }

        /// Check if mount option is incompatible
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
                    let res = T::from_str(s).map_or(MountOption::Other(s.to_string()), |x| {
                        MountOption::FsSpecific(x)
                    });
                    return Ok(res);
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
                Self::Other(x) => x.to_owned(),
            };
            write!(f, "{}", str)
        }
    }
}
