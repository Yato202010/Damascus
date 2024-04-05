use crate::Partition;
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

    /// Retreive unmount_on_drop property
    fn unmount_on_drop(&self) -> bool;

    /// Set unmount_on_drop property
    fn set_unmount_on_drop(&mut self, drop: bool);

    /// Retreive the partition Identifier
    /// "dev id" on unix and "volume serial number" on windows
    /// if the partition is not mounted it will return None
    fn partition(&self) -> Option<&Partition>;

    /// Retreive the mount point as PathBuf
    fn target(&self) -> Result<PathBuf, io::Error>;

    /// Set Target mount point
    fn set_target(&mut self, target: &dyn AsRef<Path>) -> Result<(), io::Error>;

    /// Get if the filesystem is availible
    fn is_availible() -> bool
    where
        Self: Sized;

    /// Check if the partition is mounted
    fn mounted(&self) -> bool {
        self.partition().is_some()
    }
}

/// Common trait for all stackable/union/overlay filesystem handle
pub trait StackableFilesystem: Filesystem {
    /// Retreive list of lower layer
    fn lower(&self) -> Vec<&Path>;

    /// Set lower layer
    fn set_lower(&mut self, lower: Vec<PathBuf>) -> Result<(), io::Error>;

    /// Retreive upper layer if set
    fn upper(&self) -> Option<&Path>;

    /// Set upper layer
    fn set_upper(&mut self, upper: PathBuf) -> Result<(), io::Error>;
}
