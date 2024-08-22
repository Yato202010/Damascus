/*
* implementation inspired by libmount crate
* https://github.com/tailhook/libmount/blob/master/src/overlay.rs
*
*/

use std::{
    ffi::{CStr, CString},
    io,
    os::unix::prelude::OsStrExt,
    path::{Path, PathBuf},
};

use crate::{
    common::fs::Filesystem,
    os::{AsCString, AsPath},
    PartitionID, StackableFilesystem,
};
use nix::{
    mount::{mount, umount2, MntFlags, MsFlags},
    unistd::getuid,
};
use tracing::{debug, error};

#[derive(Debug)]
/// Kernel overlay filesystem handle
pub struct OverlayFs {
    lower: Vec<PathBuf>,
    upper: Option<PathBuf>,
    work: Option<PathBuf>,
    target: CString,
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
        if PartitionID::from(upper.as_ref()) != PartitionID::from(work.as_ref()) {
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
        if PartitionID::from(self.upper.clone().unwrap()) != PartitionID::from(&work) {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "overlay FileSystem need the upper dir and the work dir to be on the same FileSystem",
            ));
        }
        self.work = Some(work);
        Ok(())
    }
}

impl Filesystem for OverlayFs {
    #[inline]
    fn mount(&mut self) -> Result<PathBuf, io::Error> {
        if matches!(self.id,Some(x) if x == PartitionID::from(self.target.as_path())) {
            debug!("Damascus: partition already mounted");
            return Ok(self.target.as_path().to_path_buf());
        }
        let mut flags = MsFlags::MS_NOATIME.union(MsFlags::MS_NODIRATIME);
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
            options.extend(b",userxattr");
        }
        mount(
            Some(CStr::from_bytes_with_nul(b"overlay\0").unwrap()),
            &*self.target,
            Some(CStr::from_bytes_with_nul(b"overlay\0").unwrap()),
            flags,
            Some(&*options),
        )?;
        self.id = Some(PartitionID::from(&self.target.as_path()));
        Ok(self.target.as_path().to_path_buf())
    }

    #[inline]
    fn unmount(&mut self) -> Result<(), io::Error> {
        if matches!(self.id,Some(x) if x == PartitionID::from(self.target.as_path())) {
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

    fn is_availible() -> bool {
        std::fs::read_to_string("/proc/filesystems")
            .expect("Should have been able to read the file")
            .contains("overlay")
    }
}

impl StackableFilesystem for OverlayFs {
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
        if PartitionID::from(&upper) != PartitionID::from(self.work.clone().unwrap()) {
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
            // This is used as a argument separator
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
    fn availibility() {
        assert!(OverlayFs::is_availible())
    }
}
