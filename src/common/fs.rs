use crate::PartitionID;
use std::{
    io,
    path::{Path, PathBuf},
};

/// Common trait for all filesystem handle
pub trait Filesystem {
    #[must_use = "Error on filesystem operation should be handled"]
    /// Request handle to mount the filesystem, returning a PathBuf pointing to the mount point
    fn mount(&mut self) -> Result<PathBuf, io::Error>;

    #[must_use = "Error on filesystem operation should be handled"]
    /// Request handle to unmount the filesystem
    fn unmount(&mut self) -> Result<(), io::Error>;

    /// Retrieve unmount_on_drop property
    fn unmount_on_drop(&self) -> bool;

    /// Set unmount_on_drop property
    fn set_unmount_on_drop(&mut self, drop: bool);

    /// Retrieve the partition Identifier
    /// "dev id" on UNIX and "volume serial number" on Windows
    /// if the partition is not mounted it will return None
    fn id(&self) -> Option<&PartitionID>;

    /// Retrieve the mount point as PathBuf
    fn target(&self) -> Result<PathBuf, io::Error>;

    /// Set Target mount point
    fn set_target(&mut self, target: &dyn AsRef<Path>) -> Result<(), io::Error>;

    /// Get if the filesystem is available
    fn is_available() -> bool
    where
        Self: Sized;

    /// Check if the partition is mounted
    fn mounted(&self) -> bool {
        self.id().is_some()
    }
}

/// Common trait for all stackable/union/overlay filesystem handle
#[allow(dead_code)]
pub trait StackableFilesystem: Filesystem {
    /// Retrieve list of lower layer
    fn lower(&self) -> Vec<&Path>;

    /// Set lower layer
    fn set_lower(&mut self, lower: Vec<PathBuf>) -> Result<(), io::Error>;

    /// Retrieve upper layer if set
    fn upper(&self) -> Option<&Path>;

    /// Set upper layer
    fn set_upper(&mut self, upper: PathBuf) -> Result<(), io::Error>;
}

/// Common trait for all case-insensitive filesystem handle
#[allow(dead_code)]
pub trait CaseInsensitive: Filesystem {}
