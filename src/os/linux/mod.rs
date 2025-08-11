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
#[cfg(feature = "fuse-overlayfs")]
pub mod fuseoverlay;
#[cfg(feature = "fuse-overlayfs")]
pub use fuseoverlay::FuseOverlayFs;
#[cfg(feature = "overlayfs")]
pub mod overlay;
#[cfg(feature = "overlayfs")]
pub use overlay::OverlayFs;

#[allow(unused_imports)]
pub(crate) use option::set_option_helper;
pub use option::{FsOption, LinuxFilesystem, MountOption};
#[allow(unused_imports)]
pub(crate) use recover_state::{FsData, restore_fsdata};

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
    pub(crate) fn restore_fsdata<P: AsRef<Path>, O: FsOption>(
        path: P,
    ) -> Result<Option<FsData<O>>> {
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

    #[allow(dead_code)]
    pub(crate) fn set_option_helper<T, O>(
        options: &mut Vec<MountOption<T>>,
        option: O,
    ) -> Result<()>
    where
        T: FsOption + PartialEq,
        O: Into<MountOption<T>>,
    {
        let option = option.into();
        let mut idx = None;
        for (i, opt) in options.iter().enumerate() {
            if opt == &option {
                return Ok(());
            } else if matches!((opt,&option), (MountOption::FsSpecific(s), MountOption::FsSpecific(o)) if std::mem::discriminant(s) == std::mem::discriminant(o))
            {
                idx = Some(i);
            } else if opt.incompatible(&option) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Unsupported,
                    "Incompatible mount option combination",
                ));
            }
        }
        if let Some(idx) = idx {
            options[idx] = option;
        } else {
            options.push(option);
        }
        Ok(())
    }

    pub trait FsOption: Sized + Clone + Display + FromStr {
        /// Get defaults mount options for this filesystem
        fn defaults() -> Vec<Self>;
        /// Check if a mount option is incompatible
        fn incompatible(&self, other: &MountOption<Self>) -> bool;
    }

    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
    pub enum MountOption<O: FsOption> {
        /// Mount the filesystem read-write.
        RW,
        /// Mount the filesystem read-only.
        RO,
        /// Honor set-user-ID and set-group-ID bits or file capabilities when executing programs from this filesystem
        Suid(bool),
        FsSpecific(O),
        Other(String),
    }

    impl<T: FsOption> MountOption<T> {
        /// Get defaults mount options for this filesystem
        pub fn defaults() -> Vec<Self> {
            let mut v: Vec<MountOption<T>> = vec![];
            let mut r = T::defaults();
            v.extend(r.iter_mut().map(|x| MountOption::FsSpecific(x.clone())));
            v
        }

        /// Check if a mount option is incompatible
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
