pub trait MOption: Sized + Clone {
    fn defaults() -> Vec<Self>;
    fn incompatible(&self, other: &MountOption<Self>) -> bool;
    fn to_string(&self) -> String;
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

    pub fn to_string(&self) -> String {
        match self {
            MountOption::FsSpecific(o) => o.to_string(),
            Self::RW => "rw".to_owned(),
            Self::RO => "ro".to_owned(),
        }
    }
}
