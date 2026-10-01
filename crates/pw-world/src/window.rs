//! A list of rows addressed by a dense id that can forget its oldest rows.
//!
//! Some registries (information items, incidents, meetings) are append-only and referred to by id from many places. Left alone they
//! grow for ever. A `Window` keeps the same addressing (`window[id]`, `window.get(id)`) but can drop a prefix of old rows: an id that
//! has been forgotten answers `None` from `get` (every reader already copes with an id that has no row), and `len()` stays the
//! *next id to be issued*, so ids are never reused.
//!
//! **The saved bytes are exactly those of the `Vec<T>` it replaces.** The first retained row carries its own id (`Keyed::key`), which
//! is how the offset is recovered on load, so no save migration is needed. The window is never emptied (the newest row stays) so the
//! offset cannot be lost.

use std::ops::{Index, IndexMut};

use std::marker::PhantomData;

use pw_core::Id;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A row that knows its own id.
pub trait Keyed {
    fn key(&self) -> u32;
}

#[derive(Clone, Debug)]
pub struct Window<T, I = usize> {
    base: u32,
    rows: Vec<T>,
    _id: PhantomData<fn(I) -> I>,
}

impl<T, I> Default for Window<T, I> {
    fn default() -> Self {
        Self { base: 0, rows: Vec::new(), _id: PhantomData }
    }
}

impl<T, I: Id> Window<T, I> {
    /// The id the next pushed row gets.
    #[inline]
    pub fn next_id(&self) -> I {
        I::from_index(self.len())
    }

    /// Ids of the rows held, oldest first.
    pub fn ids(&self) -> impl DoubleEndedIterator<Item = I> + ExactSizeIterator + use<I, T> {
        (self.base as usize..self.len()).map(I::from_index)
    }

    pub fn iter_enumerated(&self) -> impl DoubleEndedIterator<Item = (I, &T)> + ExactSizeIterator {
        let base = self.base as usize;
        self.rows.iter().enumerate().map(move |(i, t)| (I::from_index(base + i), t))
    }

    /// Id of the oldest row still held (ids below it were forgotten).
    #[inline]
    pub fn base(&self) -> u32 {
        self.base
    }

    /// The next id to be issued (equals the number of rows ever pushed, forgotten or not).
    #[inline]
    pub fn len(&self) -> usize {
        self.base as usize + self.rows.len()
    }

    /// Rows currently held.
    #[inline]
    pub fn held(&self) -> usize {
        self.rows.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Was this id issued and then forgotten?
    #[inline]
    pub fn forgotten(&self, id: I) -> bool {
        id.index() < self.base as usize
    }

    #[inline]
    pub fn push(&mut self, row: T) -> I {
        let id = self.next_id();
        self.rows.push(row);
        id
    }

    #[inline]
    pub fn get(&self, id: I) -> Option<&T> {
        id.index().checked_sub(self.base as usize).and_then(|i| self.rows.get(i))
    }

    #[inline]
    pub fn get_mut(&mut self, id: I) -> Option<&mut T> {
        id.index().checked_sub(self.base as usize).and_then(|i| self.rows.get_mut(i))
    }

    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.rows.iter()
    }

    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, T> {
        self.rows.iter_mut()
    }

    pub fn last(&self) -> Option<&T> {
        self.rows.last()
    }

    pub fn as_slice(&self) -> &[T] {
        &self.rows
    }

    /// Forget the oldest rows while `old` says so, always keeping the newest row. Returns the new base.
    pub fn forget_front_while(&mut self, mut old: impl FnMut(&T) -> bool) -> u32 {
        let mut n = 0;
        while n + 1 < self.rows.len() && old(&self.rows[n]) {
            n += 1;
        }
        if n > 0 {
            self.rows.drain(..n);
            self.base += n as u32;
            if self.rows.capacity() > 2 * self.rows.len() + 1024 {
                self.rows.shrink_to_fit();
            }
        }
        self.base
    }
}

impl<T, I: Id> Index<I> for Window<T, I> {
    type Output = T;
    #[inline]
    fn index(&self, id: I) -> &T {
        match self.get(id) {
            Some(r) => r,
            None => panic!("row {} is not held (oldest held is {}, next id {})", id.index(), self.base, self.len()),
        }
    }
}

impl<T, I: Id> IndexMut<I> for Window<T, I> {
    #[inline]
    fn index_mut(&mut self, id: I) -> &mut T {
        let (base, len, ix) = (self.base, self.len(), id.index());
        match self.get_mut(id) {
            Some(r) => r,
            None => panic!("row {ix} is not held (oldest held is {base}, next id {len})"),
        }
    }
}

impl<'a, T, I> IntoIterator for &'a Window<T, I> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.rows.iter()
    }
}

impl<T: Serialize, I> Serialize for Window<T, I> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.rows.serialize(s)
    }
}

impl<'de, T: Deserialize<'de> + Keyed, I> Deserialize<'de> for Window<T, I> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let rows = Vec::<T>::deserialize(d)?;
        let base = rows.first().map_or(0, Keyed::key);
        Ok(Self { base, rows, _id: PhantomData })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
    struct Row {
        id: u32,
        v: u8,
    }

    impl Keyed for Row {
        fn key(&self) -> u32 {
            self.id
        }
    }

    #[test]
    fn ids_survive_forgetting_and_the_bytes_are_those_of_a_vec() {
        let mut w: Window<Row> = Window::default();
        for i in 0..10 {
            w.push(Row { id: i, v: i as u8 });
        }
        assert_eq!(bincode::serialize(&w).unwrap(), bincode::serialize(&w.as_slice().to_vec()).unwrap());
        w.forget_front_while(|r| r.id < 4);
        assert_eq!((w.len(), w.held(), w.base()), (10, 6, 4));
        assert!(w.get(3usize).is_none() && w.forgotten(3usize) && !w.forgotten(4usize));
        assert_eq!(w[4].v, 4);
        let back: Window<Row> = bincode::deserialize(&bincode::serialize(&w).unwrap()).unwrap();
        assert_eq!((back.len(), back.base()), (10, 4));
        assert_eq!(back[9], Row { id: 9, v: 9 });
        // The newest row is never forgotten.
        w.forget_front_while(|_| true);
        assert_eq!((w.len(), w.held()), (10, 1));
        w.push(Row { id: 10, v: 1 });
        assert_eq!(w[10].id, 10);
    }
}
