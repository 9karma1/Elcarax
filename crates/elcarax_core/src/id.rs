use std::cmp::Ordering as CmpOrdering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::num::NonZeroU64;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

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

impl<T> Serialize for Id<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(self.value.get())
    }
}

impl<'de, T> Deserialize<'de> for Id<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = u64::deserialize(deserializer)?;
        Self::new(value).ok_or_else(|| serde::de::Error::custom("id must be non-zero"))
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

    /// Advances this generator past an identifier loaded from storage.
    ///
    /// Persisted identifiers are part of the generator's namespace. Without
    /// observing them, the next runtime-created value can collide with a
    /// loaded object even though the storage itself was valid.
    pub fn observe(&self, id: Id<T>) {
        let next = id.get().saturating_add(1);
        let mut current = self.next_value.load(Ordering::Relaxed);
        while current < next {
            match self.next_value.compare_exchange_weak(
                current,
                next,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => return,
                Err(observed) => current = observed,
            }
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

    #[test]
    fn observing_loaded_id_reserves_following_values() {
        let generator = IdGenerator::<TestMarker>::new();
        let loaded = match Id::new(100) {
            Some(id) => id,
            None => panic!("fixture id should be non-zero"),
        };
        generator.observe(loaded);
        assert_eq!(generator.next_id().get(), 101);
    }
}
