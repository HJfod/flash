
use std::path::{PathBuf, Component};

pub trait Normalize {
    fn normalize(&self) -> Self;
}

impl Normalize for PathBuf {
    fn normalize(&self) -> Self {
        let mut res = Self::new();
        for comp in self.components() {
            if comp == Component::ParentDir {
                res.pop();
            }
            else {
                res.push(comp);
            }
        }
        res
    }
}
