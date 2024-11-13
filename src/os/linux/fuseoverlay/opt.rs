use crate::{FsOption, MountOption};

use std::{fmt::Display, str::FromStr};
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FuseOverlayFsOption {
    // /// Use separate fuse device fd for each thread
    // CloneFd,
    /// The maximum number of idle worker threads allowed (default: -1)
    MaxIdleThread(isize),
    /// The maximum number of worker threads allowed (default: 10)
    MaxThread(usize),
    /// Allow access by all users
    AllowOther,
    /// Allow access by root
    AllowRoot,
    /// Every file and directory is owned by the root user (0:0).
    SquashToRoot,
    /// Every file and directory is owned by the specified uid.
    /// It has higher precedence over squash_to_root.
    SquashToUid(usize),
    /// Every file and directory is owned by the specified gid.
    /// It has higher precedence over squash_to_root.
    SquashToGid(usize),
    /// Set st_nlink to static value 1 for all directories
    StaticNLink,
    /// Disable ACL support in the FUSE file system
    NoAcl,
    // TODO : look into uidmapping and gidmapping
}

impl FsOption for FuseOverlayFsOption {
    fn defaults() -> Vec<Self> {
        vec![]
    }

    fn incompatible(&self, _other: &MountOption<Self>) -> bool {
        // TODO : find incompatible mount option and define compatibility matrix
        false
    }
}

impl FromStr for FuseOverlayFsOption {
    type Err = std::io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some((op, va)) = s.split_once('=') {
            match op {
                "max_idle_threads" => {
                    if let Ok(u) = va.parse() {
                        return Ok(Self::MaxIdleThread(u));
                    }
                }
                "max_threads" => {
                    if let Ok(u) = va.parse() {
                        return Ok(Self::MaxThread(u));
                    }
                }
                "squash_to_uid" => {
                    if let Ok(u) = va.parse() {
                        return Ok(Self::SquashToUid(u));
                    }
                }
                "squash_to_gid" => {
                    if let Ok(u) = va.parse() {
                        return Ok(Self::SquashToGid(u));
                    }
                }
                _ => {}
            };
        }

        Ok(match s {
            "allow_other" => Self::AllowOther,
            "allow_root" => Self::AllowRoot,
            "squash_to_root" => Self::SquashToRoot,
            "static_nlink" => Self::StaticNLink,
            "noacl" => Self::NoAcl,
            // "clone_fd" => Self::CloneFd,
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Unsupported,
                    "Unsupported mount option",
                ));
            }
        })
    }
}
impl Display for FuseOverlayFsOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                //FuseOverlayFsOption::CloneFd => "clone_fd".to_owned(),
                FuseOverlayFsOption::MaxIdleThread(x) => format!("max_idle_threads={}", x),
                FuseOverlayFsOption::MaxThread(x) => format!("max_threads={}", x),
                FuseOverlayFsOption::AllowOther => "allow_other".to_owned(),
                FuseOverlayFsOption::AllowRoot => "allow_root".to_owned(),
                FuseOverlayFsOption::SquashToRoot => "squash_to_root".to_owned(),
                FuseOverlayFsOption::SquashToUid(uid) => format!("squash_to_uid={}", uid),
                FuseOverlayFsOption::SquashToGid(gid) => format!("squash_to_gid={}", gid),
                FuseOverlayFsOption::StaticNLink => "static_nlink".to_owned(),
                FuseOverlayFsOption::NoAcl => "noacl".to_owned(),
            }
        )
    }
}

impl From<FuseOverlayFsOption> for MountOption<FuseOverlayFsOption> {
    fn from(val: FuseOverlayFsOption) -> Self {
        MountOption::FsSpecific(val)
    }
}
