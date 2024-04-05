#[cfg(all(
    target_os = "linux",
    any(feature = "overlayfs", feature = "fuse-overlayfs")
))]
mod linux;

#[cfg(all(target_os = "windows",any()))]
mod windows;

/// Mimic normal test output (hackishly).
macro_rules! run_tests {
    ( $($test_fn:ident),* ) => {{
        println!();

        use colored::*;
        $(
            print!("test test_mount::{} ... ", stringify!($test_fn));
            $test_fn();
            println!("{}","ok".green());
        )*

        println!();
    }}
}

#[cfg(target_os = "linux")]
fn main() {
    #[cfg(feature = "fuse-overlayfs")]
    use crate::linux::test_mount_fuse_overlay::{
        init_fuse_overlay_r, init_fuse_overlay_rw, mount_fuse_overlay_r, mount_fuse_overlay_rw,
        mount_fuse_overlay_rw_on_lower,
    };
    #[cfg(feature = "overlayfs")]
    use crate::linux::test_mount_overlay::{
        init_overlay_r, init_overlay_rw, mount_overlay_r, mount_overlay_rw,
        mount_overlay_rw_on_lower,
    };
    #[cfg(feature = "fuse-overlayfs")]
    run_tests!(
        init_fuse_overlay_r,
        init_fuse_overlay_rw,
        mount_fuse_overlay_r,
        mount_fuse_overlay_rw,
        mount_fuse_overlay_rw_on_lower
    );
    #[cfg(feature = "overlayfs")]
    run_tests!(
        init_overlay_r,
        init_overlay_rw,
        mount_overlay_r,
        mount_overlay_rw,
        mount_overlay_rw_on_lower
    );
}

#[cfg(target_os = "windows")]
fn main() {}
