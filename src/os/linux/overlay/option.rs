use crate::{MOption, MountOption};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RedirectDir {
    On,
    Follow,
    NoFollow,
    Off,
}

impl Into<MountOption<OverlayFsOption>> for RedirectDir {
    fn into(self) -> MountOption<OverlayFsOption> {
        MountOption::FsSpecific(OverlayFsOption::RedirectDir(self))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FsVerity {
    On,
    Require,
    Off,
}

impl Into<MountOption<OverlayFsOption>> for FsVerity {
    fn into(self) -> MountOption<OverlayFsOption> {
        MountOption::FsSpecific(OverlayFsOption::FsVerity(self))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Xino {
    On,
    Auto,
    Off,
}

impl Into<MountOption<OverlayFsOption>> for Xino {
    fn into(self) -> MountOption<OverlayFsOption> {
        MountOption::FsSpecific(OverlayFsOption::Xino(self))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OverlayFsOption {
    /// ### On
    /// Redirects are enabled.
    /// ### Follow
    /// Redirects are not created, but followed.
    /// ### NoFollow
    /// Redirects are not created and not followed.
    /// ### Off
    /// If “redirect_always_follow” is enabled in the kernel/module config, this “off” translates to “follow”, otherwise it translates to “nofollow”.
    RedirectDir(RedirectDir),
    /// When the “metacopy” feature is enabled, overlayfs will only copy up metadata (as opposed to whole file)
    Metacopy(bool),
    /// ### On  
    /// Whenever a metacopy files specifies an expected digest, the corresponding data file must match the specified digest. When generating a metacopy file the verity digest will be set in it based on the source file (if it has one).
    /// ### Require
    /// Same as “on”, but additionally all metacopy files must specify a digest (or EIO is returned on open). This means metadata copy up will only be used if the data file has fs-verity enabled, otherwise a full copy-up is used.
    /// ###• Off
    /// The metacopy digest is never generated or used. This is the default if verity option is not specified.
    FsVerity(FsVerity),
    /// Inode index. If this feature is disabled and a file with multiple hard links is copied up, then this will "break" the link. Changes will not be propagated to other names referring to the same inode.
    Index(bool),
    /// Can be used to replace UUID of the underlying filesystem in file handles with null, and effectively disable UUID checks. This can be useful in case the
    /// underlying disk is copied and the UUID of this copy is changed. This is only applicable if
    /// all lower/upper/work directories are on the same filesystems,
    /// otherwise it will fallback to normal behaviour.
    Uuid(bool),
    /// The "xino" feature composes a unique object identifier from the real object st_ino and an underlying fsid index. The "xino" feature uses the high inode number
    /// bits for fsid, because the underlying filesystems rarely use the high inode number bits. In case the underlying inode number does overflow into the high xino
    /// bits, overlay filesystem will fall back to the non xino behavior for that inode.
    ///
    /// For a detailed description of the effect of this option please refer to https://docs.kernel.org/filesystems/overlayfs.html
    Xino(Xino),
    /// Use the "user.overlay." xattr namespace instead of "trusted.overlay.". This is useful for unprivileged mounting of overlayfs.
    UserXattr,
    /// Volatile mounts are not guaranteed to survive a crash. It is strongly recommended that volatile mounts are only used if data written to the overlay can be
    /// recreated without significant effort.
    Volatile,
    // TODO : check doc and incompatibility
    // NfsExport,
}

impl MOption for OverlayFsOption {
    fn defaults() -> Vec<Self> {
        vec![
            OverlayFsOption::RedirectDir(RedirectDir::On),
            OverlayFsOption::Index(true),
            OverlayFsOption::Xino(Xino::On),
        ]
    }

    fn incompatible(&self, other: &MountOption<Self>) -> bool {
        let incompat_matrix = [|s: &OverlayFsOption, o: &MountOption<OverlayFsOption>| {
            matches!(s, OverlayFsOption::UserXattr)
                && matches!(o, MountOption::FsSpecific(OverlayFsOption::FsVerity(_)))
        }];

        for incompat in incompat_matrix {
            if incompat(self, other) {
                return true;
            }
        }
        false
    }

    fn to_string(&self) -> String {
        match self {
            OverlayFsOption::RedirectDir(o) => match o {
                RedirectDir::On => "redirect_dir=on",
                RedirectDir::Follow => "redirect_dir=follow",
                RedirectDir::NoFollow => "redirect_dir=nofollow",
                RedirectDir::Off => "redirect_dir=off",
            },
            OverlayFsOption::Metacopy(o) => match *o {
                true => "metacopy=on",
                false => "metacopy=off",
            },
            OverlayFsOption::FsVerity(o) => match o {
                FsVerity::On => "verity=on",
                FsVerity::Require => "verity=required",
                FsVerity::Off => "verity=off",
            },
            OverlayFsOption::Index(o) => match o {
                true => "index=on",
                false => "index=off",
            },
            OverlayFsOption::Uuid(_) => todo!(),
            OverlayFsOption::Xino(o) => match o {
                Xino::On => "xino=on",
                Xino::Auto => "xino=auto",
                Xino::Off => "xino=off",
            },
            OverlayFsOption::UserXattr => "userxattr",
            OverlayFsOption::Volatile => "volatile",
        }
        .to_string()
    }
}

impl Into<MountOption<OverlayFsOption>> for OverlayFsOption {
    fn into(self) -> MountOption<OverlayFsOption> {
        MountOption::FsSpecific(self)
    }
}
