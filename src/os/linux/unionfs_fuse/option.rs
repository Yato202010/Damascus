use std::{fmt::Display, path::PathBuf};

use crate::{MOption, MountOption};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnionFsFuseOption {
    /// Chroot into this path. Use this if you want to have a union of "/"
    Chroot(PathBuf),
    /// Enable copy-on-write
    Cow,
    /// Preserve branch when moving files, creating directories as needed
    PreserveBranch,
    /// ".unionfs" is a secret directory not visible by readdir(), and so are .fuse_hidden* files
    HideMetaFiles,
    /// Increase the maximum number of open files
    MaxFile(usize),
    /// Disable permissions checks, but only if running neither as UID=0 or GID=0
    RelaxedPermission,
    /// Do not count blocks of ro-branches
    StatfsOmitRo,
    /// Enable direct-io flag for fuse subsystem
    DirectIo,
}

impl MOption for UnionFsFuseOption {
    fn defaults() -> Vec<Self> {
        vec![UnionFsFuseOption::Cow, UnionFsFuseOption::HideMetaFiles]
    }

    fn incompatible(&self, other: &MountOption<Self>) -> bool {
        // TODO : find incompatible mount option and define compatibility matrix
        false
    }
}

impl Display for UnionFsFuseOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                UnionFsFuseOption::Chroot(x) => "chroot=".to_string() + &x.to_string_lossy(),
                UnionFsFuseOption::Cow => "cow".to_owned(),
                UnionFsFuseOption::PreserveBranch => "preserve_branch".to_owned(),
                UnionFsFuseOption::HideMetaFiles => "hide_meta_files".to_owned(),
                UnionFsFuseOption::MaxFile(x) => format!("max_files={}", x),
                UnionFsFuseOption::RelaxedPermission => "relaxed_permissions".to_owned(),
                UnionFsFuseOption::StatfsOmitRo => "statfs_omit_ro".to_owned(),
                UnionFsFuseOption::DirectIo => "direct_io".to_owned(),
            }
        )
    }
}

impl From<UnionFsFuseOption> for MountOption<UnionFsFuseOption> {
    fn from(val: UnionFsFuseOption) -> Self {
        MountOption::FsSpecific(val)
    }
}
