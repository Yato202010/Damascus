use std::{
    ffi::{CStr, CString, OsStr},
    path::{Path, PathBuf},
};

cfg_if::cfg_if! {
    if #[cfg(target_os = "linux")] {
        mod linux;
        pub(crate) use std::os::unix::ffi::OsStrExt;
        #[allow(unused_imports)]
        pub use linux::*;
    } else if #[cfg(target_os = "windows")] {
        mod windows;
        #[allow(unused_imports)]
        pub use windows::*;
        pub(crate) trait OsStrExt {
            fn from_bytes(b: &[u8]) -> &Self;
            fn as_bytes(&self) -> &[u8];
        }

        impl OsStrExt for OsStr {
            #[allow(clippy::transmute_ptr_to_ptr)]
            fn from_bytes(b: &[u8]) -> &Self {
                use std::mem;
                unsafe { mem::transmute(b) }
            }
            fn as_bytes(&self) -> &[u8] {
                self.to_string_lossy().as_bytes()
            }
        }
    } else if #[cfg(target_os = "macos")] {
        mod macos;
        pub(crate) use std::os::unix::OsStrExt;
        #[allow(unused_imports)]
        pub use macos::*;
    }
}

pub trait AsPath {
    fn as_path(&self) -> &Path;
}

impl AsPath for CString {
    #[inline]
    fn as_path(&self) -> &Path {
        OsStr::from_bytes(self.to_bytes()).as_ref()
    }
}

impl AsPath for CStr {
    #[inline]
    fn as_path(&self) -> &Path {
        OsStr::from_bytes(self.to_bytes()).as_ref()
    }
}

pub trait AsCString {
    fn as_cstring(&self) -> CString;
}

impl AsCString for Path {
    #[inline]
    fn as_cstring(&self) -> CString {
        let int = {
            #[cfg(target_family = "unix")]
            {
                self.as_os_str().as_bytes()
            }
            #[cfg(target_os = "windows")]
            {
                self.to_string_lossy().to_string()
            }
        };

        // TODO : remove unwrap if possible
        #[allow(clippy::unwrap_used)]
        CString::new(int).unwrap()
    }
}

impl AsCString for PathBuf {
    #[inline]
    fn as_cstring(&self) -> CString {
        self.as_path().as_cstring()
    }
}
