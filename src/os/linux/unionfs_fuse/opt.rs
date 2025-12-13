// Copyright 2025 Yato202010
//
// Licensed under the Apache License, Version 2.0 (the "License"); you may not use this file except in compliance with the License. You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software distributed under the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the License for the specific language governing permissions and limitations under the License.
use std::{fmt::Display, path::PathBuf, str::FromStr};

use crate::common::fs::MountOption;

#[non_exhaustive]
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
    /// Enable direct-io flag for the fuse subsystem
    DirectIo,
}

impl MountOption for UnionFsFuseOption {
    fn defaults() -> Vec<String> {
        vec![
            UnionFsFuseOption::Cow.to_string(),
            UnionFsFuseOption::HideMetaFiles.to_string(),
        ]
    }
}

impl From<UnionFsFuseOption> for String {
    fn from(val: UnionFsFuseOption) -> Self {
        val.to_string()
    }
}

impl FromStr for UnionFsFuseOption {
    type Err = std::io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some((op, va)) = s.split_once('=') {
            match op {
                "max_files" => {
                    if let Ok(u) = va.parse() {
                        return Ok(Self::MaxFile(u));
                    }
                }
                "chroot" => {
                    return Ok(Self::Chroot(PathBuf::from(va)));
                }
                _ => {}
            };
        }

        Ok(match s {
            "cow" => Self::Cow,
            "preserve_branch" => Self::PreserveBranch,
            "hide_meta_files" => Self::HideMetaFiles,
            "relaxed_permissions" => Self::RelaxedPermission,
            "statfs_omit_ro" => UnionFsFuseOption::StatfsOmitRo,
            "direct_io" => Self::DirectIo,
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Unsupported,
                    "Unsupported mount option",
                ));
            }
        })
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
