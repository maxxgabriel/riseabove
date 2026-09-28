use std::marker::PhantomData;
use std::ops::{Deref, DerefMut, Index, IndexMut};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub trait Id: Copy {
    fn from_index(i: usize) -> Self;
    fn index(self) -> usize;
}

/// A dense vector indexed by a typed id. Derefs to a slice so rayon and the
/// standard slice API work directly; indexing with the wrong id type does not
/// compile.
pub struct IdVec<I, T> {
    items: Vec<T>,
    _id: PhantomData<fn(I) -> I>,
}

impl<I: Id, T> IdVec<I, T> {
    pub const fn new() -> Self {
        Self { items: Vec::new(), _id: PhantomData }
    }

    pub fn with_capacity(n: usize) -> Self {
        Self { items: Vec::with_capacity(n), _id: PhantomData }
    }

    pub fn from_vec(items: Vec<T>) -> Self {
        Self { items, _id: PhantomData }
    }

    #[inline]
    pub fn push(&mut self, item: T) -> I {
        let id = I::from_index(self.items.len());
        self.items.push(item);
        id
    }

    #[inline]
    pub fn next_id(&self) -> I {
        I::from_index(self.items.len())
    }

    #[inline]
    pub fn get(&self, id: I) -> Option<&T> {
        self.items.get(id.index())
    }

    #[inline]
    pub fn get_mut(&mut self, id: I) -> Option<&mut T> {
        self.items.get_mut(id.index())
    }

    pub fn ids(&self) -> impl DoubleEndedIterator<Item = I> + ExactSizeIterator + use<I, T> {
        (0..self.items.len()).map(I::from_index)
    }

    pub fn iter_enumerated(&self) -> impl DoubleEndedIterator<Item = (I, &T)> + ExactSizeIterator {
        self.items.iter().enumerate().map(|(i, t)| (I::from_index(i), t))
    }

    pub fn iter_enumerated_mut(&mut self) -> impl DoubleEndedIterator<Item = (I, &mut T)> + ExactSizeIterator {
        self.items.iter_mut().enumerate().map(|(i, t)| (I::from_index(i), t))
    }

    /// Two distinct mutable borrows.
    pub fn pair_mut(&mut self, a: I, b: I) -> (&mut T, &mut T) {
        let (a, b) = (a.index(), b.index());
        assert_ne!(a, b, "pair_mut requires distinct ids");
        if a < b {
            let (lo, hi) = self.items.split_at_mut(b);
            (&mut lo[a], &mut hi[0])
        } else {
            let (lo, hi) = self.items.split_at_mut(a);
            (&mut hi[0], &mut lo[b])
        }
    }

    pub fn into_vec(self) -> Vec<T> {
        self.items
    }
}

impl<I: Id, T> Default for IdVec<I, T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<I, T: Clone> Clone for IdVec<I, T> {
    fn clone(&self) -> Self {
        Self { items: self.items.clone(), _id: PhantomData }
    }
}

impl<I, T: std::fmt::Debug> std::fmt::Debug for IdVec<I, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.items.fmt(f)
    }
}

impl<I, T> Deref for IdVec<I, T> {
    type Target = [T];

    #[inline]
    fn deref(&self) -> &[T] {
        &self.items
    }
}

impl<I, T> DerefMut for IdVec<I, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut [T] {
        &mut self.items
    }
}

impl<I: Id, T> Index<I> for IdVec<I, T> {
    type Output = T;

    #[inline]
    fn index(&self, id: I) -> &T {
        &self.items[id.index()]
    }
}

impl<I: Id, T> IndexMut<I> for IdVec<I, T> {
    #[inline]
    fn index_mut(&mut self, id: I) -> &mut T {
        &mut self.items[id.index()]
    }
}

impl<I, T: Serialize> Serialize for IdVec<I, T> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.items.serialize(s)
    }
}

impl<'de, I, T: Deserialize<'de>> Deserialize<'de> for IdVec<I, T> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Ok(Self { items: Vec::deserialize(d)?, _id: PhantomData })
    }
}
