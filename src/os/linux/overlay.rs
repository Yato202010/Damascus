// Copyright 2025 Yato202010
//
// Licensed under the Apache License, Version 2.0 (the "License"); you may not use this file except in compliance with the License. You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software distributed under the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the License for the specific language governing permissions and limitations under the License.
/*
* Implementation inspired by libmount crate
* https://github.com/tailhook/libmount/blob/master/src/overlay.rs
*
*/

mod opt;
use crate::common::fs::MountOption;
pub use opt::*;

use nix::mount::{MntFlags, MsFlags, mount, umount2};
use std::{
    ffi::CString,
    io::{Error, ErrorKind, Result},
    ops::Deref,
    path::{Path, PathBuf},
};
use tracing::{debug, error};

use crate::{
    AsCString, AsPath, Filesystem, FsData, PartitionID, StackableFilesystem, StateRecovery,
    restore_fsdata,
};

#[derive(Debug)]
/// Kernel overlay filesystem handle
pub struct OverlayFs {
    lower: Vec<PathBuf>,
    upper: Option<PathBuf>,
    work: Option<PathBuf>,
    target: CString,
    options: Vec<String>,
    id: Option<PartitionID>,
    drop: bool,
}

impl OverlayFs {
    #[must_use = "initialised OverlayFs handle should be used"]
    #[inline]
    pub fn new(
        lower: impl IntoIterator<Item = impl AsRef<Path>>,
        upper: Option<impl Into<PathBuf>>,
        work: Option<impl Into<PathBuf>>,
        target: impl AsRef<Path>,
        drop: bool,
    ) -> Result<OverlayFs> {
        Ok(Self {
            lower: lower
                .into_iter()
                .map(|x| x.as_ref().to_path_buf())
                .collect(),
            upper: upper.map(|x| x.into()),
            work: work.map(|x| x.into()),
            target: target.as_ref().as_cstring(),
            options: OverlayFsOption::defaults(),
            id: None,
            drop,
        })
    }

    #[must_use = "initialised OverlayFs handle should be used"]
    #[inline]
    pub fn readonly(
        lower: impl IntoIterator<Item = impl AsRef<Path>>,
        target: impl AsRef<Path>,
    ) -> Result<OverlayFs> {
        let lower: Vec<PathBuf> = lower
            .into_iter()
            .map(|x| x.as_ref().to_path_buf())
            .collect();
        if lower.len() < 2 {
            return Err(Error::other(
                "overlay FileSystem need a least 2 lower directory to work",
            ));
        }
        Ok(OverlayFs {
            lower,
            upper: None,
            work: None,
            target: target.as_ref().as_cstring(),
            options: OverlayFsOption::defaults(),
            id: None,
            drop: true,
        })
    }

    #[must_use = "initialised OverlayFs handle should be used"]
    #[inline]
    pub fn writable(
        lower: impl IntoIterator<Item = impl AsRef<Path>>,
        upper: impl AsRef<Path>,
        work: impl AsRef<Path>,
        target: impl AsRef<Path>,
    ) -> Result<OverlayFs> {
        if PartitionID::try_from(upper.as_ref())? != PartitionID::try_from(work.as_ref())? {
            return Err(Error::other(
                "overlay FileSystem need the upper dir and the work dir to be on the same FileSystem",
            ));
        }
        Ok(OverlayFs {
            lower: lower
                .into_iter()
                .map(|x| x.as_ref().to_path_buf())
                .collect(),
            upper: Some(upper.as_ref().to_path_buf()),
            work: Some(work.as_ref().to_path_buf()),
            target: target.as_ref().as_cstring(),
            options: OverlayFsOption::defaults(),
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
                "overlay FileSystem need the upper dir and the work dir to be on the same FileSystem",
            ));
        }
        self.work = Some(work);
        Ok(())
    }
}

impl Filesystem for OverlayFs {
    #[inline]
    fn mount(&mut self) -> Result<&mut Self> {
        if !Self::is_available() {
            return Err(Error::new(
                ErrorKind::NotFound,
                "overlayfs is not available",
            ));
        }
        if self.mounted() {
            debug!("Damascus: partition already mounted");
            return Ok(self);
        }
        let flags = MsFlags::empty();
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
            options.push_str(&(",".to_string() + &mo.to_string()))
        }
        let mut args = options.as_bytes().to_vec();
        args.push(b'\0');
        let data = unsafe { CString::from_vec_with_nul_unchecked(args) };
        mount(
            Some(c"overlay"),
            &*self.target,
            Some(c"overlay"),
            flags,
            Some(data.as_bytes()),
        )
        .inspect_err(|_x| {
            dbg!(&self);
        })?;
        self.id = Some(
            PartitionID::try_from(self.target.as_path())
                .map_err(|_| Error::other("unable to get PartitionID"))?,
        );
        Ok(self)
    }

    #[inline]
    fn unmount(&mut self) -> Result<&mut Self> {
        if self.mounted() {
            umount2(self.target.as_c_str(), MntFlags::MNT_DETACH)?;
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
        if let Ok(res) = std::fs::read_to_string("/proc/filesystems") {
            res.contains("overlay")
        } else {
            false
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

impl StackableFilesystem for OverlayFs {
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
                "overlay FileSystem need the upper dir and the work dir to be on the same FileSystem",
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

impl StateRecovery for OverlayFs {
    fn recover<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let data: FsData = restore_fsdata(path)?.ok_or(Error::new(
            ErrorKind::NotFound,
            "OverlayFs not found at mount point : ".to_string() + &path.to_string_lossy(),
        ))?;
        let mut lower = vec![];
        let mut upper = None;
        let mut work = None;
        let target = path.as_cstring();
        let options = data
            .options()
            .iter()
            .filter_map(|x| {
                let (o, va) = if let Some(x) = x.split_once('=') {
                    x
                } else {
                    return Some(x.to_owned());
                };
                match o {
                    "lowerdir" => {
                        for path in va.split(':') {
                            lower.push(PathBuf::from(path))
                        }
                        return None;
                    }
                    "upperdir" => {
                        upper = Some(PathBuf::from(va));
                        return None;
                    }
                    "workdir" => {
                        work = Some(PathBuf::from(va));
                        return None;
                    }
                    _ => {}
                }
                Some(x.to_owned())
            })
            .collect();
        Ok(Self {
            lower,
            upper,
            work,
            target,
            options,
            id: Some(
                PartitionID::try_from(path)
                    .map_err(|_| Error::other("unable to get PartitionID"))?,
            ),
            drop: false,
        })
    }
}

impl Drop for OverlayFs {
    #[inline]
    fn drop(&mut self) {
        if self.drop
            && let Err(err) = self.unmount()
        {
            error!(
                "Damascus: unable to unmount overlay at {:?} because : {}",
                self.target, err
            )
        }
    }
}
