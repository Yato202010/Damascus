/*
* Implementation inspired by libmount crate
* https://github.com/tailhook/libmount/blob/master/src/overlay.rs
*
*/

pub mod opt;

use nix::{
    mount::{mount, umount2, MntFlags, MsFlags},
    unistd::getuid,
};
use std::{
    ffi::{CStr, CString},
    io::{self, Error, Result},
    path::{Path, PathBuf},
};
use tracing::{debug, error};

use crate::{
    common::fs::{Filesystem, StateRecovery},
    os::{
        linux::overlay::opt::OverlayFsOption,
        linux::recover_state::{restore_fsdata, FsData},
        AsCString, AsPath, LinuxFilesystem, MountOption,
    },
    PartitionID, StackableFilesystem,
};

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
    ) -> Result<OverlayFs>
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
    pub fn readonly<I, A, T>(lower: I, target: T) -> Result<OverlayFs>
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
    pub fn writable<I, A, B, C, D>(lower: I, upper: B, work: C, target: D) -> Result<OverlayFs>
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
    pub fn set_work(&mut self, work: PathBuf) -> Result<()> {
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

impl Filesystem for OverlayFs {
    #[inline]
    fn mount(&mut self) -> Result<PathBuf> {
        if matches!(self.id,Some(x) if x == PartitionID::try_from(self.target.as_path())?) {
            debug!("Damascus: partition already mounted");
            return Ok(self.target.as_path().to_path_buf());
        }
        // let mut flags = MsFlags::MS_NOATIME.union(MsFlags::MS_NODIRATIME);
        let mut flags = MsFlags::empty();
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
        } else {
            flags = flags.union(MsFlags::MS_RDONLY);
        }
        if !getuid().is_root() {
            self.set_option(OverlayFsOption::UserXattr)?;
        }
        for mo in &self.options {
            options.push_str(&(",".to_string() + &mo.to_string()))
        }
        let mut args = options.as_bytes().to_vec();
        args.push(b'\0');
        mount(
            Some(unsafe { CStr::from_bytes_with_nul(b"overlay\0").unwrap_unchecked() }),
            &*self.target,
            Some(unsafe { CStr::from_bytes_with_nul(b"overlay\0").unwrap_unchecked() }),
            flags,
            Some(unsafe { CString::from_vec_with_nul_unchecked(args).as_bytes() }),
        )?;
        self.id = Some(PartitionID::try_from(self.target.as_path())?);
        Ok(self.target.as_path().to_path_buf())
    }

    #[inline]
    fn unmount(&mut self) -> Result<()> {
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
    fn target(&self) -> PathBuf {
        self.target.as_path().to_path_buf()
    }

    #[inline]
    fn set_target(&mut self, target: &dyn AsRef<Path>) -> Result<()> {
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
}

impl LinuxFilesystem<OverlayFsOption> for OverlayFs {
    fn set_option(&mut self, option: impl Into<MountOption<OverlayFsOption>>) -> Result<()> {
        let option = option.into();
        for (i, opt) in self.options.clone().iter().enumerate() {
            // If Option is already set with another value, overwrite it
            if matches!((opt,&option), (MountOption::FsSpecific(s), MountOption::FsSpecific(o)) if std::mem::discriminant(s) == std::mem::discriminant(o))
                | matches!((opt,&option), (s,o) if std::mem::discriminant(s) == std::mem::discriminant(o))
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

    fn remove_option(&mut self, option: impl Into<MountOption<OverlayFsOption>>) -> Result<()> {
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

impl StackableFilesystem for OverlayFs {
    #[inline]
    fn lower(&self) -> Vec<&Path> {
        self.lower.iter().map(|x| x.as_path()).collect()
    }

    #[inline]
    fn set_lower(&mut self, lower: Vec<PathBuf>) -> Result<()> {
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
    fn set_upper(&mut self, upper: PathBuf) -> Result<()> {
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

impl StateRecovery for OverlayFs {
    fn recover<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let data: FsData<OverlayFsOption> = restore_fsdata(path)?.ok_or(Error::new(
            io::ErrorKind::NotFound,
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
                if let MountOption::Other(str) = x {
                    let (o, va) = if let Some(x) = str.split_once('=') {
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
            id: Some(PartitionID::try_from(path)?),
            drop: false,
        })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn availability() {
        assert!(OverlayFs::is_available())
    }
}
