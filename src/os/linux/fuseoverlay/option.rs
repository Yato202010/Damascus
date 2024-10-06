use crate::{MOption, MountOption};

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
    /// Set st_nlink to the static value 1 for all directories
    StaticNLink,
    /// Disable ACL support in the FUSE file system
    NoAcl,
    // -o uidmapping=UID:MAPPED-UID:LEN[,UID2:MAPPED-UID2:LEN2] -o gidmapping=GID:MAPPED-GID:LEN[,GID2:MAPPED-GID2:LEN2] Specifies the dynamic UID/GID mapping used by fuse-
    //       overlayfs when reading/writing files to the system.
}

impl MOption for FuseOverlayFsOption {
    fn defaults() -> Vec<Self> {
        vec![]
    }

    fn incompatible(&self, other: &MountOption<Self>) -> bool {
        // TODO : find incompatible mount option and define compatibility matrix
        false
    }

    fn to_string(&self) -> String {
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
    }
}

impl Into<MountOption<FuseOverlayFsOption>> for FuseOverlayFsOption {
    fn into(self) -> MountOption<FuseOverlayFsOption> {
        MountOption::FsSpecific(self)
    }
}
