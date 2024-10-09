use std::fmt::Display;

pub trait MOption: Sized + Clone + Display {
    fn defaults() -> Vec<Self>;
    fn incompatible(&self, other: &MountOption<Self>) -> bool;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MountOption<O: MOption> {
    RW,
    RO,
    FsSpecific(O),
}

impl<T: MOption> MountOption<T> {
    pub fn defaults() -> Vec<Self> {
        let mut v: Vec<MountOption<T>> = vec![];
        let mut r = T::defaults();
        v.extend(r.iter_mut().map(|x| MountOption::FsSpecific(x.clone())));
        v
    }

    pub fn incompatible(&self, other: &MountOption<T>) -> bool {
        match self {
            MountOption::FsSpecific(o) => o.incompatible(other),
            MountOption::RW if matches!(other, MountOption::RO) => true,
            _ => false,
        }
    }
}

impl<T: MOption> Display for MountOption<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            MountOption::FsSpecific(o) => o.to_string(),
            Self::RW => "rw".to_owned(),
            Self::RO => "ro".to_owned(),
        };
        write!(f, "{}", str)
    }
}
