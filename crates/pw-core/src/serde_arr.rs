//! `#[serde(with = "serde_arr")]` for fixed arrays of any length.

use std::marker::PhantomData;

use serde::de::{Error, SeqAccess, Visitor};
use serde::ser::SerializeTuple;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub fn serialize<S: Serializer, T: Serialize, const N: usize>(a: &[T; N], s: S) -> Result<S::Ok, S::Error> {
    let mut t = s.serialize_tuple(N)?;
    for x in a {
        t.serialize_element(x)?;
    }
    t.end()
}

pub fn deserialize<'de, D, T, const N: usize>(d: D) -> Result<[T; N], D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + Copy + Default,
{
    struct ArrVisitor<T, const N: usize>(PhantomData<T>);

    impl<'de, T: Deserialize<'de> + Copy + Default, const N: usize> Visitor<'de> for ArrVisitor<T, N> {
        type Value = [T; N];

        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(f, "an array of length {N}")
        }

        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<[T; N], A::Error> {
            let mut out = [T::default(); N];
            for (i, slot) in out.iter_mut().enumerate() {
                *slot = seq.next_element()?.ok_or_else(|| A::Error::invalid_length(i, &self))?;
            }
            Ok(out)
        }
    }

    d.deserialize_tuple(N, ArrVisitor::<T, N>(PhantomData))
}
