use derive_more::AsRef;
use serde::Serialize;

use crate::mpd::{FromMpd, LineHandled, errors::MpdError};

#[derive(Debug, Serialize, Default, PartialEq, AsRef, Clone, Copy)]
pub struct CurrentIndex(u32);

impl Bound<u32> for CurrentIndex {
    fn value(&self) -> &u32 {
        &self.0
    }

    fn set_value(&mut self, value: u32) -> &Self {
        self.0 = value;
        self
    }

    fn inc(&mut self) -> &Self {
        self.0 += 1;
        self
    }

    fn inc_by(&mut self, step: u32) -> &Self {
        self.0 = self.0.saturating_add(step);
        self
    }

    fn dec(&mut self) -> &Self {
        self.0 = self.0.saturating_sub(1);
        self
    }

    fn dec_by(&mut self, step: u32) -> &Self {
        self.0 = self.0.saturating_sub(step);
        self
    }
}

#[allow(dead_code)]
pub trait Bound<T> {
    fn value(&self) -> &u32;
    fn set_value(&mut self, value: u32) -> &Self;
    fn inc(&mut self) -> &Self;
    fn inc_by(&mut self, step: T) -> &Self;
    fn dec(&mut self) -> &Self;
    fn dec_by(&mut self, step: T) -> &Self;
}

impl FromMpd for CurrentIndex {
    fn next_internal(&mut self, key: &str, value: String) -> Result<LineHandled, MpdError> {
        if key == "id" {
            self.0 = value.parse()?;
            Ok(LineHandled::Yes)
        } else {
            Ok(LineHandled::No { value })
        }
    }
}
