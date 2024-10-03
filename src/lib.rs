#![deny(clippy::unwrap_used)]
#![deny(clippy::panic)]
#![deny(clippy::expect_used)]

mod common;
mod os;
pub use common::{
    fs::{Filesystem, StackableFilesystem},
    option::*,
    utils::partition::PartitionID,
};

pub use os::*;
