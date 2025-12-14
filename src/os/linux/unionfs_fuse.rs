// Copyright 2025 Yato202010
//
// Licensed under the Apache License, Version 2.0 (the "License"); you may not use this file except in compliance with the License. You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software distributed under the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the License for the specific language governing permissions and limitations under the License.
/*
* implementation inspired by libmount crate
* https://github.com/tailhook/libmount/blob/master/src/overlay.rs
*/

mod opt;
pub use opt::*;

use std::{
    ffi::CString,
    io::{Error, ErrorKind, Result},
    ops::Deref,
    path::{Path, PathBuf},
    process::Command,
};
use tracing::{debug, error};

use crate::{
    AsCString, AsPath, Filesystem, PartitionID, StackableFilesystem, common::fs::MountOption,
};

#[derive(Debug)]
/// Unionfs fuse filesystem handle
pub struct UnionFsFuse {
    lower: Vec<PathBuf>,
    upper: Option<PathBuf>,
    target: CString,
    options: Vec<String>,
    id: Option<PartitionID>,
    drop: bool,
}

impl UnionFsFuse {
    #[must_use = "initialised UnionFsFuse handle should be used"]
    #[inline]
    /// Initialise a new UnionFsFuse handle
    pub fn new(
        lower: impl IntoIterator<Item = impl AsRef<Path>>,
        upper: Option<impl Into<PathBuf>>,
        target: impl AsRef<Path>,
        drop: bool,
    ) -> Result<UnionFsFuse> {
        let lower: Vec<PathBuf> = lower
            .into_iter()
            .map(|x| x.as_ref().to_path_buf())
            .collect();
        Ok(Self {
            lower,
            upper: upper.map(|x| x.into()),
            target: target.as_ref().as_cstring(),
            options: UnionFsFuseOption::defaults(),
            id: None,
            drop,
        })
    }

    #[must_use = "initialised UnionFsFuse handle should be used"]
    #[inline]
    /// Initialise a new readonly UnionFsFuse handle
    pub fn readonly(
        lower: impl IntoIterator<Item = impl AsRef<Path>>,
        target: impl AsRef<Path>,
    ) -> Result<UnionFsFuse> {
        let lower: Vec<PathBuf> = lower
            .into_iter()
            .map(|x| x.as_ref().to_path_buf())
            .collect();
        if lower.len() < 2 {
            return Err(Error::other(
                "overlay FileSystem need a least 2 lower directory to work",
            ));
        }
        Ok(Self {
            lower,
            upper: None,
            target: target.as_ref().as_cstring(),
            options: UnionFsFuseOption::defaults(),
            id: None,
            drop: true,
        })
    }

    #[must_use = "initialised UnionFsFuse handle should be used"]
    #[inline]
    /// Initialise a new writable UnionFsFuse handle
    pub fn writable(
        lower: impl IntoIterator<Item = impl AsRef<Path>>,
        upper: impl AsRef<Path>,
        target: impl AsRef<Path>,
    ) -> Result<Self> {
        Ok(Self {
            lower: lower
                .into_iter()
                .map(|x| x.as_ref().to_path_buf())
                .collect(),
            upper: Some(upper.as_ref().to_path_buf()),
            target: target.as_ref().as_cstring(),
            options: UnionFsFuseOption::defaults(),
            id: None,
            drop: true,
        })
    }
}

impl Filesystem for UnionFsFuse {
    #[inline]
    fn mount(&mut self) -> Result<&mut Self> {
        #[cfg(not(feature = "unionfs-fuse-vendored"))]
        if !Self::is_available() {
            return Err(Error::new(
                ErrorKind::NotFound,
                "unionfs-fuse is not available",
            ));
        }
        if self.mounted() {
            debug!("Damascus: partition already mounted");
            return Ok(self);
        }
        let mut layer_args: String = String::new();
        for path in &self.lower {
            layer_args.push_str(path.to_string_lossy().as_ref());
            layer_args.push_str("=ro:");
        }
        if let Some(upper) = &self.upper {
            layer_args.push_str(upper.to_string_lossy().as_ref());
            layer_args.push_str("=rw");
        }

        let mut options = String::new();
        for mo in &self.options {
            options.push_str(&(",".to_string() + &mo.to_string()))
        }

        let args = &[
            CString::new("unionfs")?,
            CString::new("-o")?,
            CString::new(options)?,
            CString::new(layer_args)?,
            self.target.clone(),
        ];

        #[cfg(feature = "unionfs-fuse-vendored")]
        {
            use nix::{
                sys::{
                    memfd::{MFdFlags, memfd_create},
                    wait::waitpid,
                },
                unistd::{ForkResult, fexecve, fork, write},
            };
            // init embedded unionfs fuse since it's not always packaged by distribution
            let byte = include_bytes!(concat!("../../../", env!("UNIONFS-FUSE-BIN")));
            let mem = memfd_create(CString::new("unionfs")?.as_c_str(), MFdFlags::empty())?;
            write(&mem, byte)?;
            let env: Vec<CString> = vec![];
            match unsafe { fork() } {
                Ok(ForkResult::Parent { child, .. }) => {
                    waitpid(child, None)?;
                }
                Ok(ForkResult::Child) => {
                    fexecve(mem, args, &env)?;
                }
                Err(_) => {
                    return Err(Error::new(
                        ErrorKind::PermissionDenied,
                        "Failed to mount vfs",
                    ));
                }
            }
        }
        #[cfg(not(feature = "unionfs-fuse-vendored"))]
        {
            let options: Vec<&str> = args.iter().skip(1).map(|x| x.to_str().unwrap()).collect();
            let output = Command::new("unionfs")
                .args(options)
                .spawn()
                .unwrap()
                .wait_with_output()
                .unwrap();
            if !output.status.success() {
                error!(
                    "Damascus: unable to mount {:?}\n{}",
                    &self,
                    String::from_utf8_lossy(&output.stderr)
                );
                return Err(Error::new(
                    ErrorKind::PermissionDenied,
                    "Failed to mount vfs",
                ));
            }
        };

        self.id = Some(
            PartitionID::try_from(self.target.as_path())
                .map_err(|_| Error::other("unable to get PartitionID"))?,
        );
        Ok(self)
    }

    #[inline]
    fn unmount(&mut self) -> Result<&mut Self> {
        if self.mounted() {
            let child = Command::new("fusermount")
                .args(["-z", "-u"])
                .arg(self.target.as_path())
                .spawn()?;
            let output = child.wait_with_output()?;
            if !output.status.success() {
                error!(
                    "Damascus: unable to unmount {:?}\n{}",
                    &self,
                    String::from_utf8_lossy(&output.stderr)
                );
                return Err(Error::new(
                    ErrorKind::PermissionDenied,
                    "Failed to unmount vfs",
                ));
            }
            self.id = None;
        }
        Ok(self)
    }

    #[inline]
    fn scoped(&self) -> bool {
        self.drop
    }

    #[inline]
    fn set_scoped(&mut self, drop: bool) -> &mut Self {
        self.drop = drop;
        self
    }

    #[inline]
    fn id(&self) -> Option<&PartitionID> {
        self.id.as_ref()
    }

    #[inline]
    fn target(&self) -> PathBuf {
        self.target.as_path().to_path_buf()
    }

    #[inline]
    fn set_target(&mut self, target: impl AsRef<Path>) -> Result<&mut Self> {
        if self.id.is_some() {
            return Err(Error::other(
                "mount point cannot be change when the FileSystem is mounted",
            ));
        }
        self.target = target.as_ref().as_cstring();
        Ok(self)
    }

    fn is_available() -> bool {
        #[cfg(feature = "unionfs-fuse-vendored")]
        {
            true
        }
        #[cfg(not(feature = "unionfs-fuse-vendored"))]
        {
            use std::process::Stdio;
            Command::new("unionfs")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .is_ok()
        }
    }

    fn add_option(&mut self, option: impl Into<String>) -> Result<()> {
        self.options.push(option.into());
        Ok(())
    }

    fn remove_option(&mut self, option: impl AsRef<str>) -> Result<()> {
        self.options.retain(|x| x.deref() != option.as_ref());
        Ok(())
    }

    fn options(&self) -> &[String] {
        self.options.deref()
    }
}

impl StackableFilesystem for UnionFsFuse {
    #[inline]
    fn lower(&self) -> Vec<&Path> {
        self.lower.iter().map(|x| x.as_path()).collect()
    }

    #[inline]
    fn set_lower(
        &mut self,
        lower: impl IntoIterator<Item = impl AsRef<Path>>,
    ) -> Result<&mut Self> {
        if self.id.is_some() {
            return Err(Error::other(
                "upper layer cannot be change when the FileSystem is mounted",
            ));
        }
        self.lower = lower
            .into_iter()
            .map(|x| x.as_ref().to_path_buf())
            .collect();
        Ok(self)
    }

    #[inline]
    fn upper(&self) -> Option<&Path> {
        self.upper.as_deref()
    }

    #[inline]
    fn set_upper(&mut self, upper: impl Into<PathBuf>) -> Result<&mut Self> {
        if self.id.is_some() {
            return Err(Error::other(
                "upper layer cannot be change when the FileSystem is mounted",
            ));
        }
        self.upper = Some(upper.into());
        Ok(self)
    }
}

impl Drop for UnionFsFuse {
    #[inline]
    fn drop(&mut self) {
        if self.drop
            && let Err(err) = self.unmount()
        {
            error!(
                "Damascus: unable to unmount unionfs fuse at {:?} because : {}",
                self.target, err
            )
        }
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "unionfs-fuse-vendored")]
    #[test]
    fn availability() {
        use super::{Filesystem, UnionFsFuse};
        assert!(UnionFsFuse::is_available())
    }
}
