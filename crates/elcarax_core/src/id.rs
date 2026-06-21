use std::cmp::Ordering as CmpOrdering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::num::NonZeroU64;
use std::sync::atomic::{AtomicU64, Ordering};

pub struct Id<T> {
    value: NonZeroU64,
    marker: PhantomData<fn() -> T>,
}

impl<T> Id<T> {
    pub fn new(value: u64) -> Option<Self> {
        NonZeroU64::new(value).map(|value| Self {
            value,
            marker: PhantomData,
        })
    }

    pub const fn from_non_zero(value: NonZeroU64) -> Self {
        Self {
            value,
            marker: PhantomData,
        }
    }

    pub const fn get(self) -> u64 {
        self.value.get()
    }
}

impl<T> Copy for Id<T> {}

impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<T> Eq for Id<T> {}

impl<T> PartialOrd for Id<T> {
    fn partial_cmp(&self, other: &Self) -> Option<CmpOrdering> {
        Some(self.cmp(other))
    }
}

impl<T> Ord for Id<T> {
    fn cmp(&self, other: &Self) -> CmpOrdering {
        self.value.cmp(&other.value)
    }
}

impl<T> Hash for Id<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.value.hash(state);
    }
}

impl<T> fmt::Debug for Id<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Id({})", self.value)
    }
}

pub struct IdGenerator<T> {
    next_value: AtomicU64,
    marker: PhantomData<fn() -> T>,
}

impl<T> IdGenerator<T> {
    pub const fn new() -> Self {
        Self {
            next_value: AtomicU64::new(1),
            marker: PhantomData,
        }
    }

    pub fn next_id(&self) -> Id<T> {
        let value = self.next_value.fetch_add(1, Ordering::Relaxed);
        match Id::new(value) {
            Some(id) => id,
            None => Id::from_non_zero(NonZeroU64::MIN),
        }
    }
}

impl<T> Default for IdGenerator<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    enum TestMarker {}

    #[test]
    fn generated_ids_are_monotonic() {
        let generator = IdGenerator::<TestMarker>::new();
        let first = generator.next_id();
        let second = generator.next_id();
        assert!(first < second);
    }
}
