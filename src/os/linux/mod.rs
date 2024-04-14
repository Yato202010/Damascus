#[cfg(any(feature = "fuse-overlayfs", feature = "fuse-overlayfs-vendored"))]
mod fuseoverlay;
#[cfg(any(feature = "fuse-overlayfs", feature = "fuse-overlayfs-vendored"))]
pub use fuseoverlay::FuseOverlayFs;
#[cfg(feature = "overlayfs")]
mod overlay;
#[cfg(feature = "overlayfs")]
pub use overlay::OverlayFs;
