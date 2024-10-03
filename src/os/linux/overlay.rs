/*
* implementation inspired by libmount crate
* https://github.com/tailhook/libmount/blob/master/src/overlay.rs
*
*/

pub mod option;

use std::{
    ffi::{CStr, CString},
    io,
    os::unix::prelude::OsStrExt,
    path::{Path, PathBuf},
};

use crate::{
    common::{fs::Filesystem, option::MountOption},
    os::{AsCString, AsPath},
    PartitionID, StackableFilesystem,
};
use nix::{
    mount::{mount, umount2, MntFlags, MsFlags},
    unistd::getuid,
};
use option::OverlayFsOption;
use tracing::{debug, error};

#[derive(Debug)]
/// Kernel overlay filesystem handle
pub struct OverlayFs {
    lower: Vec<PathBuf>,
    upper: Option<PathBuf>,
    work: Option<PathBuf>,
    target: CString,
    options: Vec<MountOption<OverlayFsOption>>,
    id: Option<PartitionID>,
    drop: bool,
}

impl OverlayFs {
    #[must_use = "initialised OverlayFs handle should be used"]
    #[inline]
    pub fn new<'x, I, B, C, D>(
        lower: I,
        upper: Option<B>,
        work: Option<C>,
        target: D,
        drop: bool,
    ) -> Result<OverlayFs, io::Error>
    where
        I: Iterator<Item = &'x Path>,
        B: Into<PathBuf>,
        C: Into<PathBuf>,
        D: AsRef<Path>,
    {
        Ok(Self {
            lower: lower.map(|x| x.to_path_buf()).collect(),
            upper: upper.map(|x| x.into()),
            work: work.map(|x| x.into()),
            target: target.as_ref().as_cstring(),
            options: MountOption::defaults(),
            id: None,
            drop,
        })
    }

    #[must_use = "initialised OverlayFs handle should be used"]
    #[inline]
    pub fn readonly<I, A, T>(lower: I, target: T) -> Result<OverlayFs, io::Error>
    where
        I: Iterator<Item = A>,
        A: AsRef<Path>,
        T: AsRef<Path>,
    {
        let lower: Vec<PathBuf> = lower.map(|x| x.as_ref().to_path_buf()).collect();
        if lower.len() < 2 {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "overlay FileSystem need a least 2 lower directory to work",
            ));
        }
        Ok(OverlayFs {
            lower,
            upper: None,
            work: None,
            target: target.as_ref().as_cstring(),
            options: MountOption::defaults(),
            id: None,
            drop: true,
        })
    }

    #[must_use = "initialised OverlayFs handle should be used"]
    #[inline]
    pub fn writable<I, A, B, C, D>(
        lower: I,
        upper: B,
        work: C,
        target: D,
    ) -> Result<OverlayFs, io::Error>
    where
        I: Iterator<Item = A>,
        A: AsRef<Path>,
        B: AsRef<Path>,
        C: AsRef<Path>,
        D: AsRef<Path>,
    {
        if PartitionID::try_from(upper.as_ref())? != PartitionID::try_from(work.as_ref())? {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "overlay FileSystem need the upper dir and the work dir to be on the same FileSystem",
            ));
        }
        Ok(OverlayFs {
            lower: lower.map(|x| x.as_ref().to_path_buf()).collect(),
            upper: Some(upper.as_ref().to_path_buf()),
            work: Some(work.as_ref().to_path_buf()),
            target: target.as_ref().as_cstring(),
            options: MountOption::defaults(),
            id: None,
            drop: true,
        })
    }

    #[inline]
    pub fn work(&self) -> Option<&PathBuf> {
        self.work.as_ref()
    }

    #[inline]
    pub fn set_work(&mut self, work: PathBuf) -> Result<(), io::Error> {
        if PartitionID::try_from(work.as_path())?
            != PartitionID::try_from(
                self.upper
                    .as_ref()
                    .ok_or(io::Error::new(
                        io::ErrorKind::NotFound,
                        "upper directory not set",
                    ))?
                    .as_path(),
            )?
        {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "overlay FileSystem need the upper dir and the work dir to be on the same FileSystem",
            ));
        }
        self.work = Some(work);
        Ok(())
    }
}

impl Filesystem<OverlayFsOption> for OverlayFs {
    #[inline]
    fn mount(&mut self) -> Result<PathBuf, io::Error> {
        if matches!(self.id,Some(x) if x == PartitionID::try_from(self.target.as_path())?) {
            debug!("Damascus: partition already mounted");
            return Ok(self.target.as_path().to_path_buf());
        }
        // let mut flags = MsFlags::MS_NOATIME.union(MsFlags::MS_NODIRATIME);
        let mut flags = MsFlags::empty();
        let mut options = Vec::new();
        options.extend(b"lowerdir=");
        for (i, p) in self.lower.iter().enumerate() {
            if i != 0 {
                options.push(b':')
            }
            _append_escape(&mut options, p);
        }
        if let (Some(u), Some(w)) = (self.upper.as_ref(), self.work.as_ref()) {
            options.extend(b",upperdir=");
            _append_escape(&mut options, u);
            options.extend(b",workdir=");
            _append_escape(&mut options, w);
        } else {
            flags = flags.union(MsFlags::MS_RDONLY);
        }
        if !getuid().is_root() {
            self.set_option(OverlayFsOption::UserXattr)?;
        }
        for mo in &self.options {
            options.extend((",".to_string() + &mo.to_string()).as_bytes())
        }
        mount(
            Some(unsafe { CStr::from_bytes_with_nul(b"overlay\0").unwrap_unchecked() }),
            &*self.target,
            Some(unsafe { CStr::from_bytes_with_nul(b"overlay\0").unwrap_unchecked() }),
            flags,
            Some(&*options),
        )?;
        self.id = Some(PartitionID::try_from(self.target.as_path())?);
        Ok(self.target.as_path().to_path_buf())
    }

    #[inline]
    fn unmount(&mut self) -> Result<(), io::Error> {
        if matches!(self.id,Some(x) if x == PartitionID::try_from(self.target.as_path())?) {
            umount2(self.target.as_c_str(), MntFlags::MNT_DETACH)?;
            self.id = None;
        }
        Ok(())
    }

    #[inline]
    fn unmount_on_drop(&self) -> bool {
        self.drop
    }

    #[inline]
    fn set_unmount_on_drop(&mut self, drop: bool) {
        self.drop = drop;
    }

    #[inline]
    fn id(&self) -> Option<&PartitionID> {
        self.id.as_ref()
    }

    #[inline]
    fn target(&self) -> Result<PathBuf, io::Error> {
        Ok(self.target.as_path().to_path_buf())
    }

    #[inline]
    fn set_target(&mut self, target: &dyn AsRef<Path>) -> Result<(), io::Error> {
        if self.id.is_some() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "mount point cannot be change when the FileSystem is mounted",
            ));
        }
        self.target = target.as_ref().as_cstring();
        Ok(())
    }

    fn is_available() -> bool {
        if let Ok(res) = std::fs::read_to_string("/proc/filesystems") {
            res.contains("overlay")
        } else {
            false
        }
    }

    fn set_option(
        &mut self,
        option: impl Into<MountOption<OverlayFsOption>>,
    ) -> Result<(), io::Error> {
        let option = option.into();
        for (i, opt) in self.options.clone().iter().enumerate() {
            // If Option is already set with another value, overwrite it
            if matches!((opt,&option), (MountOption::FsSpecific(s), MountOption::FsSpecific(o)) if std::mem::discriminant(s) == std::mem::discriminant(&o))
                | matches!((opt,&option), (s,o) if std::mem::discriminant(s) == std::mem::discriminant(&o))
            {
                self.options[i] = option.clone();
                return Ok(());
            }
            // Check option incompatibility
            if opt.incompatible(&option) {
                return Err(io::Error::new(
                    io::ErrorKind::Unsupported,
                    "Incompatible mount option combinaison",
                ));
            }
        }
        self.options.push(option);
        Ok(())
    }

    fn remove_option(
        &mut self,
        option: impl Into<MountOption<OverlayFsOption>>,
    ) -> Result<(), io::Error> {
        let option = option.into();
        let idx = self.options.iter().position(|x| *x == option);
        if let Some(idx) = idx {
            let _ = self.options.remove(idx);
        }
        Ok(())
    }

    fn options(&self) -> &[MountOption<OverlayFsOption>] {
        &self.options
    }
}

impl StackableFilesystem<OverlayFsOption> for OverlayFs {
    #[inline]
    fn lower(&self) -> Vec<&Path> {
        self.lower.iter().map(|x| x.as_path()).collect()
    }

    #[inline]
    fn set_lower(&mut self, lower: Vec<PathBuf>) -> Result<(), io::Error> {
        if self.id.is_some() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "upper layer cannot be change when the FileSystem is mounted",
            ));
        }
        self.lower = lower;
        Ok(())
    }

    #[inline]
    fn upper(&self) -> Option<&Path> {
        self.upper.as_deref()
    }

    #[inline]
    fn set_upper(&mut self, upper: PathBuf) -> Result<(), io::Error> {
        if PartitionID::try_from(upper.as_path())?
            != PartitionID::try_from(
                self.work
                    .as_ref()
                    .ok_or(io::Error::new(
                        io::ErrorKind::NotFound,
                        "work directory not set",
                    ))?
                    .as_path(),
            )?
        {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "overlay FileSystem need the upper dir and the work dir to be on the same FileSystem",
            ));
        } else if self.id.is_some() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "upper layer cannot be change when the FileSystem is mounted",
            ));
        }
        self.upper = Some(upper);
        Ok(())
    }
}

impl Drop for OverlayFs {
    #[inline]
    fn drop(&mut self) {
        if self.drop {
            if let Err(err) = self.unmount() {
                error!(
                    "Damascus: unable to unmount overlay at {:?} because : {}",
                    self.target, err
                )
            }
        }
    }
}

/*********************************************
* Copyright (c) 2016 The libmount Developers *
*********************************************/
#[inline]
fn _append_escape(dest: &mut Vec<u8>, path: &Path) {
    for &byte in path.as_os_str().as_bytes().iter() {
        match byte {
            // This is escape char
            b'\\' => {
                dest.push(b'\\');
                dest.push(b'\\');
            }
            // This is used as a path separator in lowerdir
            b':' => {
                dest.push(b'\\');
                dest.push(b':');
            }
            // This is used as an argument separator
            b',' => {
                dest.push(b'\\');
                dest.push(b',');
            }
            x => dest.push(x),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn availability() {
        assert!(OverlayFs::is_available())
    }
}
