// Copyright 2025 Yato202010
//
// Licensed under the Apache License, Version 2.0 (the "License"); you may not use this file except in compliance with the License. You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software distributed under the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the License for the specific language governing permissions and limitations under the License.
use crate::PartitionID;
use std::{
    fmt::Display,
    io::Result,
    path::{Path, PathBuf},
    str::FromStr,
};

#[allow(dead_code)]
pub trait MountOption: FromStr + Display + Into<String> {
    fn defaults() -> Vec<String>;
}

/// Common trait for all filesystem handle
pub trait Filesystem {
    #[must_use = "Error on filesystem operation should be handled"]
    /// Request a handle to mount the filesystem
    fn mount(&mut self) -> Result<&mut Self>;

    #[must_use = "Error on filesystem operation should be handled"]
    /// Request a handle to unmount the filesystem
    fn unmount(&mut self) -> Result<&mut Self>;

    /// Retrieve whetever mount is scoped (unmount on drop)
    fn scoped(&self) -> bool;

    /// Set whetever mount is scoped (unmount on drop)
    fn set_scoped(&mut self, drop: bool) -> &mut Self;

    /// Retrieve the partition Identifier
    /// "dev id" on UNIX and "volume serial number" on Windows
    /// if the partition isn't mounted, it'll return None
    fn id(&self) -> Option<&PartitionID>;

    /// Retrieve the mount point as PathBuf
    fn target(&self) -> PathBuf;

    /// Set Target mount point
    fn set_target(&mut self, target: impl AsRef<Path>) -> Result<&mut Self>;

    /// Get if the filesystem is available
    fn is_available() -> bool;

    /// Check if the partition is mounted
    fn mounted(&self) -> bool {
        self.id()
            .is_some_and(|x| PartitionID::try_from(self.target().as_path()).is_ok_and(|y| &y == x))
    }

    /// Add option
    fn add_option(&mut self, option: impl Into<String>) -> Result<()>;

    /// Remove an option
    fn remove_option(&mut self, option: impl AsRef<str>) -> Result<()>;

    /// Add a collection of option
    fn append_options<I, T>(&mut self, options: I) -> Result<()>
    where
        I: IntoIterator<Item = T>,
        T: Into<String>,
    {
        for ele in options {
            self.add_option(ele)?;
        }
        Ok(())
    }

    /// List currently active option
    fn options(&self) -> &[String];
}

/// Common trait for all stackable/union/overlay filesystem handles
#[allow(dead_code)]
pub trait StackableFilesystem: Filesystem {
    /// Retrieve a list of lower layer
    fn lower(&self) -> Vec<&Path>;

    /// Set lower layer
    fn set_lower(&mut self, lower: impl IntoIterator<Item = impl AsRef<Path>>)
    -> Result<&mut Self>;

    /// Retrieve upper layer if set
    fn upper(&self) -> Option<&Path>;

    /// Set upper layer
    fn set_upper(&mut self, upper: impl Into<PathBuf>) -> Result<&mut Self>;
}

/// Common trait for all case-insensitive filesystem handles
#[allow(dead_code)]
pub trait CaseInsensitive: Filesystem {}

/// Common trait for all filesystem handles that can be recovered by using system information
/// ex: /etc/mtab on Linux, etc.
#[allow(dead_code)]
pub trait StateRecovery: Filesystem + Sized {
    /// Recover a filesystem handle from system information
    fn recover<P: AsRef<Path>>(path: P) -> Result<Self>;
}
