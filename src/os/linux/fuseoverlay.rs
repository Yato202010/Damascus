/*
* implementation inspired by libmount crate
* https://github.com/tailhook/libmount/blob/master/src/overlay.rs
*/

use std::{
    ffi::CString,
    io,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use crate::{
    common::fs::Filesystem,
    os::{AsCString, AsPath},
    PartitionID, StackableFilesystem,
};
use cfg_if::cfg_if;

use tracing::{debug, error};

#[derive(Debug, Clone)]
/// Fuse overlay filesystem handle
pub struct FuseOverlayFs {
    lower: Vec<PathBuf>,
    upper: Option<PathBuf>,
    work: Option<PathBuf>,
    target: CString,
    id: Option<PartitionID>,
    drop: bool,
}

impl FuseOverlayFs {
    #[must_use = "initialised FuseOverlayFs handle should be used"]
    #[inline]
    /// Initialise a new FuseOverlayFs handle
    pub fn new<'x, I, B, C, D>(
        lower: I,
        upper: Option<B>,
        work: Option<C>,
        target: D,
        drop: bool,
    ) -> Result<FuseOverlayFs, io::Error>
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

    #[must_use = "initialised FuseOverlayFs handle should be used"]
    #[inline]
    /// Initialise a new readonly FuseOverlayFs handle
    pub fn readonly<I, A, T>(lower: I, target: T) -> Result<FuseOverlayFs, io::Error>
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
        Ok(FuseOverlayFs {
            lower,
            upper: None,
            work: None,
            target: target.as_ref().as_cstring(),
            id: None,
            drop: true,
        })
    }

    #[must_use = "initialised FuseOverlayFs handle should be used"]
    #[inline]
    /// Initialise a new writable FuseOverlayFs handle
    pub fn writable<I, A, B, C, D>(
        lower: I,
        upper: B,
        work: C,
        target: D,
    ) -> Result<FuseOverlayFs, io::Error>
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
                "fuse-overlay FileSystem need the upper dir and the work dir to be on the same FileSystem",
            ));
        }
        Ok(FuseOverlayFs {
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
                "fuse-overlay FileSystem need the upper dir and the work dir to be on the same FileSystem",
            ));
        }
        self.work = Some(work);
        Ok(())
    }
}

impl Filesystem for FuseOverlayFs {
    #[inline]
    fn mount(&mut self) -> Result<PathBuf, io::Error> {
        if matches!(self.id,Some(x) if x == PartitionID::from(self.target.as_path())) {
            debug!("Damascus: partition already mounted");
            return Ok(PathBuf::from(&self.target.as_path()));
        }
        let mut options = String::new();
        options.push_str("lowerdir=");
        for (i, p) in self.lower.iter().enumerate() {
            if i != 0 {
                options.push(':')
            }
            options.push_str(p.to_str().unwrap());
        }
        if let (Some(u), Some(w)) = (self.upper.as_ref(), self.work.as_ref()) {
            options.push_str(",upperdir=");
            options.push_str(u.to_str().unwrap());
            options.push_str(",workdir=");
            options.push_str(w.to_str().unwrap());
        }
        let options = &[
            CString::new("")?,
            CString::new("-o")?,
            CString::new(options)?,
            self.target.clone(),
        ];

        cfg_if!(
            if #[cfg(feature = "fuse-overlayfs-vendored")] {
                use nix::{
                    sys::{
                        memfd::{memfd_create, MemFdCreateFlag},
                        wait::waitpid,
                    },
                    unistd::{fexecve, fork, write, ForkResult},
                };
                // init embeded fuse overlay version 1.10 or later since [ 1.7 , 1.9 ] doesn't support mounting on top
                // of the base directory
                let byte = include_bytes!("../../../vendor/fuse-overlayfs/fuse-overlayfs");
                let mem = memfd_create(
                    &CString::new("fuse-overlayfs")?,
                    MemFdCreateFlag::empty(),
                )?;
                write(&mem, byte)?;
                let env: Vec<CString> = vec![];
                match unsafe { fork() } {
                    Ok(ForkResult::Parent { child, .. }) => {
                        waitpid(child, None)?;
                    }
                    Ok(ForkResult::Child) => {
                        use std::os::fd::AsRawFd;
                        fexecve(mem.as_raw_fd(), options, &env)?;
                    }
                    Err(_) => {
                        return Err(
                            io::Error::new(
                                io::ErrorKind::PermissionDenied,
                                "Failed to unmount vfs"
                            )
                        )
                    }
                }
            } else {
                let options: Vec<&str> = options.iter().map(|x| x.to_str().unwrap()).collect();
                let output = Command::new("fuse-overlayfs")
                    .args(options)
                    .spawn()
                    .unwrap()
                    .wait_with_output()
                    .unwrap();
                if output.status.code().unwrap() != 0 {
                    error!(
                        "Damascus: unable to mount {:?}\n{}",
                        &self,
                        String::from_utf8(output.stderr).unwrap()
                    );
                    return Err(io::Error::new(
                        io::ErrorKind::PermissionDenied,
                        "Failed to mount vfs",
                    ));
                }
            }
        );

        self.id = Some(PartitionID::from(&self.target.as_path()));
        Ok(self.target.as_path().to_path_buf())
    }

    #[inline]
    fn unmount(&mut self) -> Result<(), io::Error> {
        if matches!(self.id,Some(x) if x == PartitionID::from(self.target.as_path())) {
            let child = Command::new("fusermount")
                .args([
                    "-z",
                    "-u",
                    self.target
                        .to_str()
                        .expect("Damascus: unable to convert CString to str"),
                ])
                .spawn()?;
            let output = child.wait_with_output()?;
            if output.status.code().unwrap() != 0 {
                error!("Damascus: unable to unmount {:?}", &self);
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "Failed to unmount vfs",
                ));
            }
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

    #[allow(unreachable_code)]
    fn is_availible() -> bool {
        #[cfg(feature = "fuse-overlayfs-vendored")]
        {
            return true;
        }
        Command::new("fuse-overlayfs")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok()
    }
}

impl StackableFilesystem for FuseOverlayFs {
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
                "fuse-overlay FileSystem need the upper dir and the work dir to be on the same FileSystem",
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

impl Drop for FuseOverlayFs {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn availibility() {
        assert!(FuseOverlayFs::is_availible())
    }
}
