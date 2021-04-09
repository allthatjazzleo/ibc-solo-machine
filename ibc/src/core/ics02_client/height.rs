use std::cmp::Ordering;

use cosmos_sdk_proto::ibc::core::client::v1::Height;

pub trait IHeight: Sized {
    fn new(revision_number: u64, revision_height: u64) -> Self;

    fn is_zero(&self) -> bool;

    fn checked_add(self, rhs: u64) -> Option<Self>;

    fn checked_sub(self, rhs: u64) -> Option<Self>;

    fn cmp(&self, other: &Self) -> Ordering;

    fn to_string(&self) -> String;
}

impl IHeight for Height {
    fn new(revision_number: u64, revision_height: u64) -> Self {
        Self {
            revision_number,
            revision_height,
        }
    }

    fn is_zero(&self) -> bool {
        self.revision_height == 0
    }

    fn checked_add(self, rhs: u64) -> Option<Self> {
        let revision_number = self.revision_number;
        let revision_height = self.revision_height.checked_add(rhs)?;

        Some(Self {
            revision_number,
            revision_height,
        })
    }

    fn checked_sub(self, rhs: u64) -> Option<Self> {
        let revision_number = self.revision_number;
        let revision_height = self.revision_height.checked_sub(rhs)?;

        Some(Self {
            revision_number,
            revision_height,
        })
    }

    fn cmp(&self, other: &Self) -> Ordering {
        match self.revision_number.cmp(&other.revision_number) {
            Ordering::Equal => self.revision_height.cmp(&other.revision_height),
            Ordering::Greater => Ordering::Greater,
            Ordering::Less => Ordering::Less,
        }
    }

    fn to_string(&self) -> String {
        format!("{}-{}", self.revision_number, self.revision_height)
    }
}
