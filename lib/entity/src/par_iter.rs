//! A rayon parallel iterator over entity references and entities.

use crate::EntityRef;
use rayon::iter::{plumbing, Enumerate, IndexedParallelIterator, ParallelIterator};
use rayon::slice;
use std::marker::PhantomData;

/// Parallel iterator over all keys.
pub struct ParIter<'a, K: EntityRef, V>
where
    K: Send,
    V: 'a + Sync,
{
    enumerate: Enumerate<slice::Iter<'a, V>>,
    unused: PhantomData<K>,
}

impl<'a, K: EntityRef, V> ParIter<'a, K, V>
where
    K: Send,
    V: Sync,
{
    /// Create a `ParIter` iterator that visits the `PrimaryMap` keys and values
    /// of `iter`.
    pub fn new(iter: slice::Iter<'a, V>) -> Self {
        Self {
            enumerate: iter.enumerate(),
            unused: PhantomData,
        }
    }
}

impl<'a, K: EntityRef, V> ParallelIterator for ParIter<'a, K, V>
where
    K: Send,
    V: Sync,
{
    type Item = (K, &'a V);

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: plumbing::UnindexedConsumer<Self::Item>,
    {
        self.enumerate
            .map(|(i, v)| (K::new(i), v))
            .drive_unindexed(consumer)
    }
}
