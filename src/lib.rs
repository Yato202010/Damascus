mod common;
mod os;
pub use common::{
    fs::{Filesystem, StackableFilesystem},
    utils::partition::Partition,
};

cfg_if::cfg_if! {
    if #[cfg(target_os = "linux")] {
        #[cfg(feature = "overlayfs")]
        pub use os::overlay::OverlayFs;
        #[cfg(feature = "fuse-overlayfs")]
        pub use os::fuseoverlay::FuseOverlayFs;
    } else if #[cfg(target_os = "windows")] {
    } else if #[cfg(target_os = "macos")] {
    }
}
