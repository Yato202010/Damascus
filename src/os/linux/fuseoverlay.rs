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
    AsCString, AsPath, Filesystem, PartitionID, StackableFilesystem, StateRecovery,
    common::fs::MountOption,
};

#[derive(Debug)]
/// Fuse overlay filesystem handle
pub struct FuseOverlayFs {
    lower: Vec<PathBuf>,
    upper: Option<PathBuf>,
    work: Option<PathBuf>,
    target: CString,
    options: Vec<String>,
    id: Option<PartitionID>,
    drop: bool,
}

impl FuseOverlayFs {
    #[must_use = "initialised FuseOverlayFs handle should be used"]
    #[inline]
    /// Initialise a new FuseOverlayFs handle
    pub fn new(
        lower: impl IntoIterator<Item = impl AsRef<Path>>,
        upper: Option<impl Into<PathBuf>>,
        work: Option<impl Into<PathBuf>>,
        target: impl AsRef<Path>,
        drop: bool,
    ) -> Result<FuseOverlayFs> {
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
            upper: upper.map(|x| x.into()),
            work: work.map(|x| x.into()),
            target: target.as_ref().as_cstring(),
            options: FuseOverlayFsOption::defaults(),
            id: None,
            drop,
        })
    }

    #[must_use = "initialised FuseOverlayFs handle should be used"]
    #[inline]
    /// Initialise a new readonly FuseOverlayFs handle
    pub fn readonly(
        lower: impl IntoIterator<Item = impl AsRef<Path>>,
        target: impl AsRef<Path>,
    ) -> Result<FuseOverlayFs> {
        let lower: Vec<PathBuf> = lower
            .into_iter()
            .map(|x| x.as_ref().to_path_buf())
            .collect();
        if lower.len() < 2 {
            return Err(Error::other(
                "overlay FileSystem need a least 2 lower directory to work",
            ));
        }
        Ok(FuseOverlayFs {
            lower,
            upper: None,
            work: None,
            target: target.as_ref().as_cstring(),
            options: FuseOverlayFsOption::defaults(),
            id: None,
            drop: true,
        })
    }

    #[must_use = "initialised FuseOverlayFs handle should be used"]
    #[inline]
    /// Initialise a new writable FuseOverlayFs handle
    pub fn writable(
        lower: impl IntoIterator<Item = impl AsRef<Path>>,
        upper: impl AsRef<Path>,
        work: impl AsRef<Path>,
        target: impl AsRef<Path>,
    ) -> Result<FuseOverlayFs> {
        if PartitionID::try_from(upper.as_ref())? != PartitionID::try_from(work.as_ref())? {
            return Err(Error::other(
                "fuse-overlay FileSystem need the upper dir and the work dir to be on the same FileSystem",
            ));
        }
        Ok(FuseOverlayFs {
            lower: lower
                .into_iter()
                .map(|x| x.as_ref().to_path_buf())
                .collect(),
            upper: Some(upper.as_ref().to_path_buf()),
            work: Some(work.as_ref().to_path_buf()),
            target: target.as_ref().as_cstring(),
            options: FuseOverlayFsOption::defaults(),
            id: None,
            drop: true,
        })
    }

    #[inline]
    pub fn work(&self) -> Option<&PathBuf> {
        self.work.as_ref()
    }

    #[inline]
    pub fn set_work(&mut self, work: PathBuf) -> Result<()> {
        if PartitionID::try_from(work.as_path())?
            != PartitionID::try_from(
                self.upper
                    .as_ref()
                    .ok_or(Error::new(ErrorKind::NotFound, "upper directory not set"))?
                    .as_path(),
            )?
        {
            return Err(Error::other(
                "fuse-overlay FileSystem need the upper dir and the work dir to be on the same FileSystem",
            ));
        }
        self.work = Some(work);
        Ok(())
    }
}

impl Filesystem for FuseOverlayFs {
    #[inline]
    fn mount(&mut self) -> Result<&mut Self> {
        #[cfg(not(feature = "fuse-overlayfs-vendored"))]
        if !Self::is_available() {
            return Err(Error::new(
                ErrorKind::NotFound,
                "fuse-overlayfs is not available",
            ));
        }
        if self.mounted() {
            debug!("Damascus: partition already mounted");
            return Ok(self);
        }
        let mut options = String::new();
        options.push_str("lowerdir=");
        for (i, p) in self.lower.iter().enumerate() {
            if i != 0 {
                options.push(':')
            }
            options.push_str(p.to_string_lossy().as_ref());
        }
        if let (Some(u), Some(w)) = (self.upper.as_ref(), self.work.as_ref()) {
            options.push_str(",upperdir=");
            options.push_str(u.to_string_lossy().as_ref());
            options.push_str(",workdir=");
            options.push_str(w.to_string_lossy().as_ref());
        }

        for mo in &self.options {
            options.push_str(&(",".to_string() + mo))
        }

        let args = &[
            c"fuse-overlayfs",
            c"-o",
            &CString::new(options)?,
            self.target.as_c_str(),
        ];

        #[cfg(feature = "fuse-overlayfs-vendored")]
        {
            use nix::{
                sys::{
                    memfd::{MFdFlags, memfd_create},
                    wait::waitpid,
                },
                unistd::{ForkResult, fexecve, fork, write},
            };
            use std::ffi::CStr;

            // init embedded fuse overlay version 1.10 or later since [ 1.7, 1.9 ] doesn't support mounting on top
            // of the base directory
            let byte = include_bytes!(concat!("../../../", env!("FUSE-OVERLAYFS-BIN")));
            let mem = memfd_create(c"fuse-overlayfs", MFdFlags::empty())?;
            write(&mem, byte)?;
            let env: Vec<&CStr> = vec![];
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
        #[cfg(not(feature = "fuse-overlayfs-vendored"))]
        {
            let options: Vec<&str> = args.iter().skip(1).map(|x| x.to_str().unwrap()).collect();
            let output = Command::new("fuse-overlayfs")
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
        #[cfg(feature = "fuse-overlayfs-vendored")]
        {
            true
        }
        #[cfg(not(feature = "fuse-overlayfs-vendored"))]
        {
            use std::process::Stdio;
            Command::new("fuse-overlayfs")
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
        &self.options
    }
}

impl StackableFilesystem for FuseOverlayFs {
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
        let upper = upper.into();
        if PartitionID::try_from(upper.as_path())?
            != PartitionID::try_from(
                self.work
                    .as_ref()
                    .ok_or(Error::new(ErrorKind::NotFound, "work directory not set"))?
                    .as_path(),
            )?
        {
            return Err(Error::other(
                "fuse-overlay FileSystem need the upper dir and the work dir to be on the same FileSystem",
            ));
        } else if self.id.is_some() {
            return Err(Error::other(
                "upper layer cannot be change when the FileSystem is mounted",
            ));
        }
        self.upper = Some(upper);
        Ok(self)
    }
}

impl StateRecovery for FuseOverlayFs {
    fn recover<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let mut cmd = Command::new("ps");
        cmd.args(["--no-headers", "x", "-o", "args=", "-C", "fuse-overlayfs"]);
        let out = cmd.output()?;
        if !out.status.success() {
            error!(
                "Damascus: unable to recover handle at {:?}\n{}",
                path,
                String::from_utf8_lossy(&out.stderr)
            );
            return Err(Error::other("Failed to recover handle"));
        }
        for line in String::from_utf8_lossy(&out.stdout).lines() {
            if let Some(x) = line.strip_prefix("fuse-overlayfs") {
                let mut args = x.split(" ");
                let mut options = vec![];
                let mut lower = vec![];
                let mut upper = None;
                let mut work = None;
                while let Some(elem) = args.next() {
                    if elem == " " {
                        continue;
                    } else if elem == "-o" {
                        if let Some(elem) = args.next() {
                            let mut elem: Vec<String> = elem
                                .split(',')
                                .filter_map(|x| {
                                    if let Some(x) = x.strip_prefix("lowerdir=") {
                                        let mut x = x.split(':').map(PathBuf::from).collect();
                                        lower.append(&mut x);
                                        None
                                    } else if let Some(x) = x.strip_prefix("upperdir=") {
                                        upper = Some(PathBuf::from(x));
                                        None
                                    } else if let Some(x) = x.strip_prefix("workdir=") {
                                        work = Some(PathBuf::from(x));
                                        None
                                    } else {
                                        Some(x.to_string())
                                    }
                                })
                                .collect();
                            options.append(&mut elem);
                        }
                    } else if let Some(target) = Some(CString::new(elem)?)
                        && target.as_path() == path
                    {
                        return Ok(Self {
                            lower,
                            upper,
                            work,
                            target,
                            options,
                            id: Some(
                                PartitionID::try_from(path)
                                    .map_err(|_| Error::other("unable to get PartitionID"))?,
                            ),
                            drop: true,
                        });
                    }
                }
            }
        }
        error!(
            "Damascus: unable to recover handle at {:?}\n{}",
            path, "no filesystem of type fuse-overlayfs is mounted here"
        );
        Err(Error::new(ErrorKind::NotFound, "Failed to recover handle"))
    }
}

impl Drop for FuseOverlayFs {
    #[inline]
    fn drop(&mut self) {
        if self.drop
            && let Err(err) = self.unmount()
        {
            error!(
                "Damascus: unable to unmount fuse overlay at {:?} because : {}",
                self.target, err
            )
        }
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "fuse-overlayfs-vendored")]
    #[test]
    fn availability() {
        use super::{Filesystem, FuseOverlayFs};
        assert!(FuseOverlayFs::is_available())
    }
}
