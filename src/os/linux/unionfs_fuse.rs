/*
* implementation inspired by libmount crate
* https://github.com/tailhook/libmount/blob/master/src/overlay.rs
*/

pub mod option;

use cfg_if::cfg_if;
use option::UnionFsFuseOption;
use std::{
    ffi::CString,
    io,
    path::{Path, PathBuf},
    process::Command,
};

use tracing::{debug, error};

use crate::{AsCString, AsPath, Filesystem, MountOption, PartitionID, StackableFilesystem};

#[derive(Debug)]
/// Unionfs fuse filesystem handle
pub struct UnionFsFuse {
    lower: Vec<PathBuf>,
    upper: Option<PathBuf>,
    target: CString,
    options: Vec<MountOption<UnionFsFuseOption>>,
    id: Option<PartitionID>,
    drop: bool,
}

impl UnionFsFuse {
    #[must_use = "initialised UnionFsFuse handle should be used"]
    #[inline]
    /// Initialise a new UnionFsFuse handle
    pub fn new<'x, I, B, D>(
        lower: I,
        upper: Option<B>,
        target: D,
        drop: bool,
    ) -> Result<UnionFsFuse, io::Error>
    where
        I: Iterator<Item = &'x Path>,
        B: Into<PathBuf>,
        D: AsRef<Path>,
    {
        let lower: Vec<PathBuf> = lower.map(|x| x.to_path_buf()).collect();
        Ok(Self {
            lower,
            upper: upper.map(|x| x.into()),
            target: target.as_ref().as_cstring(),
            options: MountOption::defaults(),
            id: None,
            drop,
        })
    }

    #[must_use = "initialised UnionFsFuse handle should be used"]
    #[inline]
    /// Initialise a new readonly UnionFsFuse handle
    pub fn readonly<I, A, T>(lower: I, target: T) -> Result<UnionFsFuse, io::Error>
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
        Ok(Self {
            lower,
            upper: None,
            target: target.as_ref().as_cstring(),
            options: MountOption::defaults(),
            id: None,
            drop: true,
        })
    }

    #[must_use = "initialised UnionFsFuse handle should be used"]
    #[inline]
    /// Initialise a new writable UnionFsFuse handle
    pub fn writable<I, A, B, D>(lower: I, upper: B, target: D) -> Result<Self, io::Error>
    where
        I: Iterator<Item = A>,
        A: AsRef<Path>,
        B: AsRef<Path>,
        D: AsRef<Path>,
    {
        Ok(Self {
            lower: lower.map(|x| x.as_ref().to_path_buf()).collect(),
            upper: Some(upper.as_ref().to_path_buf()),
            target: target.as_ref().as_cstring(),
            options: MountOption::defaults(),
            id: None,
            drop: true,
        })
    }
}

impl Filesystem<UnionFsFuseOption> for UnionFsFuse {
    #[inline]
    fn mount(&mut self) -> Result<PathBuf, io::Error> {
        if matches!(self.id,Some(x) if x == PartitionID::try_from(self.target.as_path())?) {
            debug!("Damascus: partition already mounted");
            return Ok(PathBuf::from(&self.target.as_path()));
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
            CString::new("")?,
            CString::new("-o")?,
            CString::new(options)?,
            CString::new(layer_args)?,
            self.target.clone(),
        ];

        cfg_if!(
            if #[cfg(feature = "unionfs-fuse-vendored")] {
                use nix::{
                    sys::{
                        memfd::{memfd_create, MemFdCreateFlag},
                        wait::waitpid,
                    },
                    unistd::{fexecve, fork, write, ForkResult},
                };
                // init embedded unionfs fuse since it's not always packaged by distribution
                let byte = include_bytes!(concat!("../../../",env!("UNIONFS-FUSE-BIN")));
                let mem = memfd_create(
                    &CString::new("unionfs")?,
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
                        fexecve(mem.as_raw_fd(), args, &env)?;
                    }
                    Err(_) => {
                        return Err(
                            io::Error::new(
                                io::ErrorKind::PermissionDenied,
                                "Failed to mount vfs"
                            )
                        )
                    }
                }
            } else {
                let options: Vec<&str> = args.iter().skip(1).map(|x| x.to_str().unwrap()).collect();
                let output = Command::new("unionfs")
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

        self.id = Some(PartitionID::try_from(self.target.as_path())?);
        Ok(self.target.as_path().to_path_buf())
    }

    #[inline]
    fn unmount(&mut self) -> Result<(), io::Error> {
        if matches!(self.id,Some(x) if x == PartitionID::try_from(self.target.as_path())?) {
            let child = Command::new("fusermount")
                .args(["-z", "-u"])
                .arg(self.target.as_path())
                .spawn()?;
            let output = child.wait_with_output()?;
            match output.status.code() {
                Some(0) => {}
                Some(_) | None => {
                    error!("Damascus: unable to unmount {:?}", &self);
                    return Err(io::Error::new(
                        io::ErrorKind::PermissionDenied,
                        "Failed to unmount vfs",
                    ));
                }
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

    fn set_option(
        &mut self,
        option: impl Into<crate::MountOption<UnionFsFuseOption>>,
    ) -> Result<(), io::Error> {
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

    fn remove_option(
        &mut self,
        option: impl Into<crate::MountOption<UnionFsFuseOption>>,
    ) -> Result<(), io::Error> {
        let option = option.into();
        let idx = self.options.iter().position(|x| *x == option);
        if let Some(idx) = idx {
            let _ = self.options.remove(idx);
        }
        Ok(())
    }

    fn options(&self) -> &[crate::MountOption<UnionFsFuseOption>] {
        &self.options
    }
}

impl StackableFilesystem<UnionFsFuseOption> for UnionFsFuse {
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
        if self.id.is_some() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "upper layer cannot be change when the FileSystem is mounted",
            ));
        }
        self.upper = Some(upper);
        Ok(())
    }
}

impl Drop for UnionFsFuse {
    #[inline]
    fn drop(&mut self) {
        if self.drop {
            if let Err(err) = self.unmount() {
                error!(
                    "Damascus: unable to unmount unionfs fuse at {:?} because : {}",
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
        assert!(UnionFsFuse::is_available())
    }
}
