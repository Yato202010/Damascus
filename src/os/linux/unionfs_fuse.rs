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
        if lower.is_empty() {
            return Err(Error::other(
                "union fuse FileSystem needs at least 1 lower directory to work",
            ));
        }
        Ok(Self {
            lower,
            upper: upper.map(|x| x.into()),
            target: target.as_ref().as_cstring()?,
            options: UnionFsFuseOption::defaults(),
            id: None,
            drop,
        })
    }

    #[must_use = "initialised UnionFsFuse handle should be used"]
    /// Initialise a new readonly UnionFsFuse handle
    pub fn readonly(
        lower: impl IntoIterator<Item = impl AsRef<Path>>,
        target: impl AsRef<Path>,
    ) -> Result<UnionFsFuse> {
        let lower: Vec<PathBuf> = lower
            .into_iter()
            .map(|x| x.as_ref().to_path_buf())
            .collect();
        if lower.is_empty() {
            return Err(Error::other(
                "union fuse FileSystem needs at least 1 lower directory to work",
            ));
        }
        Ok(Self {
            lower,
            upper: None,
            target: target.as_ref().as_cstring()?,
            options: UnionFsFuseOption::defaults(),
            id: None,
            drop: true,
        })
    }

    #[must_use = "initialised UnionFsFuse handle should be used"]
    /// Initialise a new writable UnionFsFuse handle
    pub fn writable(
        lower: impl IntoIterator<Item = impl AsRef<Path>>,
        upper: impl AsRef<Path>,
        target: impl AsRef<Path>,
    ) -> Result<Self> {
        let lower: Vec<PathBuf> = lower
            .into_iter()
            .map(|x| x.as_ref().to_path_buf())
            .collect();
        if lower.is_empty() {
            return Err(Error::other(
                "union fuse FileSystem needs at least 1 lower directory to work",
            ));
        }
        Ok(Self {
            lower,
            upper: Some(upper.as_ref().to_path_buf()),
            target: target.as_ref().as_cstring()?,
            options: UnionFsFuseOption::defaults(),
            id: None,
            drop: true,
        })
    }
}

impl Filesystem for UnionFsFuse {
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
        for (i, path) in self.lower.iter().enumerate() {
            layer_args.push_str(path.to_string_lossy().as_ref());
            if let Some(_upper) = &self.upper {
                layer_args.push_str("=ro:");
            } else if i < self.lower.len() - 1 {
                layer_args.push_str("=ro:");
            } else {
                layer_args.push_str("=ro");
            }
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
                unistd::{ForkResult, close, fexecve, fork, write},
            };
            use std::ffi::CStr;
            // init embedded unionfs fuse since it's not always packaged by distribution
            let byte = include_bytes!(concat!("../../../", env!("UNIONFS-FUSE-BIN")));
            let mem = memfd_create(CString::new("unionfs")?.as_c_str(), MFdFlags::empty())?;
            write(&mem, byte)?;
            let env: Vec<CString> = std::env::vars_os()
                .filter_map(|(k, v)| {
                    let s = format!("{}={}", k.to_string_lossy(), v.to_string_lossy());
                    CString::new(s).ok()
                })
                .collect();
            let env_refs: Vec<&CStr> = env.iter().map(|c| c.as_c_str()).collect();
            match unsafe { fork() } {
                Ok(ForkResult::Parent { child, .. }) => {
                    waitpid(child, None)?;
                    // Close the memfd in the parent to prevent leaking the file descriptor
                    let _ = close(mem);
                }
                Ok(ForkResult::Child) => {
                    fexecve(mem, args, &env_refs)?;
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
            let options: Vec<&str> = args
                .iter()
                .skip(1)
                .map(|x| {
                    x.to_str().map_err(|_| {
                        Error::new(
                            ErrorKind::InvalidInput,
                            "unionfs argument contains invalid UTF-8",
                        )
                    })
                })
                .collect::<std::result::Result<Vec<&str>, Error>>()?;
            let output = Command::new("unionfs")
                .args(options)
                .spawn()
                .map_err(|e| {
                    Error::new(
                        ErrorKind::PermissionDenied,
                        format!("Failed to spawn unionfs: {}", e),
                    )
                })?
                .wait_with_output()
                .map_err(|e| {
                    Error::new(
                        ErrorKind::PermissionDenied,
                        format!("Failed to wait for unionfs: {}", e),
                    )
                })?;
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

    fn scoped(&self) -> bool {
        self.drop
    }

    fn set_scoped(&mut self, drop: bool) -> &mut Self {
        self.drop = drop;
        self
    }

    fn id(&self) -> Option<&PartitionID> {
        self.id.as_ref()
    }

    fn target(&self) -> PathBuf {
        self.target.as_path().to_path_buf()
    }

    fn set_target(&mut self, target: impl AsRef<Path>) -> Result<&mut Self> {
        if self.id.is_some() {
            return Err(Error::other(
                "mount point cannot be changed when the FileSystem is mounted",
            ));
        }
        self.target = target.as_ref().as_cstring()?;
        Ok(self)
    }

    fn is_available() -> bool {
        #[cfg(feature = "unionfs-fuse-vendored")]
        {
            use std::process::Stdio;
            Command::new("fusermount")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .is_ok()
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
    fn lower(&self) -> Vec<&Path> {
        self.lower.iter().map(|x| x.as_path()).collect()
    }

    fn set_lower(
        &mut self,
        lower: impl IntoIterator<Item = impl AsRef<Path>>,
    ) -> Result<&mut Self> {
        if self.id.is_some() {
            return Err(Error::other(
                "lower layer cannot be changed when the FileSystem is mounted",
            ));
        }
        self.lower = lower
            .into_iter()
            .map(|x| x.as_ref().to_path_buf())
            .collect();
        Ok(self)
    }

    fn upper(&self) -> Option<&Path> {
        self.upper.as_deref()
    }

    fn set_upper(&mut self, upper: impl Into<PathBuf>) -> Result<&mut Self> {
        if self.id.is_some() {
            return Err(Error::other(
                "upper layer cannot be changed when the FileSystem is mounted",
            ));
        }
        self.upper = Some(upper.into());
        Ok(self)
    }
}

impl StateRecovery for UnionFsFuse {
    fn recover<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let proc_dir = std::path::Path::new("/proc");
        for entry in std::fs::read_dir(proc_dir)? {
            let entry = entry?;
            let pid_str = entry.file_name().to_string_lossy().to_string();
            if pid_str.parse::<u32>().is_err() {
                continue;
            }
            let cmdline_path = entry.path().join("cmdline");
            let cmdline = match std::fs::read_to_string(&cmdline_path) {
                Ok(s) => s,
                Err(_) => continue,
            };
            let args: Vec<&str> = cmdline.split('\0').filter(|s| !s.is_empty()).collect();
            if args.first().map(|s| s.contains("unionfs")).unwrap_or(false) {
                let mut iter = args.iter().skip(1); // skip the binary name
                let mut options = vec![];
                let mut lower = vec![];
                let mut upper = None;
                let mut target_path: Option<PathBuf> = None;
                while let Some(elem) = iter.next() {
                    if *elem == "-o" {
                        if let Some(opts) = iter.next() {
                            for part in opts.split(',') {
                                if !part.is_empty() {
                                    options.push(part.to_string());
                                }
                            }
                        }
                    } else {
                        let parts: Vec<&str> = if elem.contains(':') {
                            elem.split(':').collect()
                        } else {
                            vec![*elem]
                        };
                        for part in parts {
                            if let Some(path_part) = part.strip_suffix("=ro") {
                                lower.push(PathBuf::from(path_part));
                            } else if let Some(path_part) = part.strip_suffix("=rw") {
                                upper = Some(PathBuf::from(path_part));
                            } else {
                                target_path = Some(PathBuf::from(part));
                            }
                        }
                    }
                }
                if let Some(target) = target_path
                    && target == path
                {
                    let target_cstring = path.as_cstring()?;
                    return Ok(Self {
                        lower,
                        upper,
                        target: target_cstring,
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
        error!(
            "Damascus: unable to recover handle at {:?}\n{}",
            path, "no filesystem of type union fuse is mounted here"
        );
        Err(Error::new(
            ErrorKind::NotFound,
            "Failed to recover union fuse filesystem handle at: ".to_string()
                + &path.to_string_lossy(),
        ))
    }
}

impl Drop for UnionFsFuse {
    fn drop(&mut self) {
        if self.drop
            && let Err(err) = self.unmount()
        {
            error!(
                "Damascus: unable to unmount union fuse filesystem at {:?} because : {}",
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
