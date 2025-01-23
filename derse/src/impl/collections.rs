use crate::*;
use std::cmp::Eq;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap, HashSet, LinkedList, VecDeque};
use std::hash::Hash;

macro_rules! seq_se_rev_impl {
    (
        $ty:ident <T $(: $tbound1:ident $(+ $tbound2:ident)*)*>
    ) => {
        impl<T> Serialize for $ty<T>
        where
            T: Serialize,
        {
            #[inline]
            fn serialize_to<S: Serializer>(&self, serializer: &mut S) -> Result<()> {
                for item in self.iter().rev() {
                    item.serialize_to(serializer)?;
                }
                VarInt64(self.len() as u64).serialize_to(serializer)
            }
        }
    };
}

seq_se_rev_impl! {
    Vec<T>
}

seq_se_rev_impl! {
    VecDeque<T>
}

seq_se_rev_impl! {
    LinkedList<T>
}

seq_se_rev_impl! {
    BinaryHeap<T: Ord>
}

macro_rules! seq_se_for_impl {
    (
        $ty:ident <T $(: $tbound1:ident $(+ $tbound2:ident)*)*>
    ) => {
        impl<T> Serialize for $ty<T>
        where
            T: Serialize,
        {
            #[inline]
            fn serialize_to<S: Serializer>(&self, serializer: &mut S) -> Result<()> {
                for item in self.iter() {
                    item.serialize_to(serializer)?;
                }
                VarInt64(self.len() as u64).serialize_to(serializer)
            }
        }
    };
}

seq_se_for_impl! {
    BTreeSet<T: Ord>
}

seq_se_for_impl! {
    HashSet<T: Eq + Hash>
}

macro_rules! seq_de_impl {
    (
        $ty:ident <T $(: $tbound1:ident $(+ $tbound2:ident)*)*>
    ) => {
        impl<'a, T> Deserialize<'a> for $ty<T>
        where
            T: Deserialize<'a> $(+ $tbound1 $(+ $tbound2)*)*,
        {
            fn deserialize_from<D: Deserializer<'a>>(buf: &mut D) -> Result<Self>
            where
                Self: Sized,
            {
                let len = VarInt64::deserialize_from(buf)?.0 as usize;
                (0..len).map(|_| T::deserialize_from(buf)).collect::<Result<Self>>()
            }
        }
    };
}

seq_de_impl! {
    Vec<T>
}

seq_de_impl! {
    VecDeque<T>
}

seq_de_impl! {
    LinkedList<T>
}

seq_de_impl! {
    BinaryHeap<T: Ord>
}

seq_de_impl! {
    BTreeSet<T: Ord>
}

seq_de_impl! {
    HashSet<T: Eq + Hash>
}

macro_rules! map_impl {
    (
        $ty:ident <K $(: $kbound1:ident $(+ $kbound2:ident)*)*, V>
    ) => {
        impl<K, V> Serialize for $ty<K, V>
        where
            K: Serialize,
            V: Serialize,
        {
            #[inline]
            fn serialize_to<S: Serializer>(&self, serializer: &mut S) -> Result<()> {
                for item in self.iter() {
                    item.serialize_to(serializer)?;
                }
                VarInt64(self.len() as u64).serialize_to(serializer)
            }
        }

        impl<'a, K, V> Deserialize<'a> for $ty<K, V>
        where
            Self: Sized,
            K: Deserialize<'a> $(+ $kbound1 $(+ $kbound2)*)*,
            V: Deserialize<'a>,
        {
            #[inline]
            fn deserialize_from<D: Deserializer<'a>>(buf: &mut D) -> Result<Self>
            where
                Self: Sized,
            {
                let len = VarInt64::deserialize_from(buf)?.0 as usize;
                (0..len).map(|_| <(K, V)>::deserialize_from(buf)).collect::<Result<Self>>()
            }
        }
    }
}

map_impl! {
    BTreeMap<K: Ord, V>
}

map_impl! {
    HashMap<K: Eq + Hash, V>
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collections() {
        {
            let ser = vec!["hello", "world", "!"];
            let bytes: DownwardBytes = ser.serialize().unwrap();
            let der = Vec::<String>::deserialize(&bytes[..]).unwrap();
            assert_eq!(ser, der);

            assert!(Vec::<u8>::deserialize([128].as_ref()).is_err());
            assert!(Vec::<u8>::deserialize([1].as_ref()).is_err());
            assert!(Vec::<u8>::deserialize([0].as_ref()).unwrap().is_empty());
        }

        {
            let ser = Some("hello".to_string());
            let bytes: DownwardBytes = ser.serialize().unwrap();
            assert_eq!(bytes.len(), 1 + 1 + 5);
            let der = Option::<String>::deserialize(&bytes[..]).unwrap();
            assert_eq!(ser, der);
            let der = Vec::<String>::deserialize(&bytes[..]).unwrap();
            assert_eq!(ser.as_ref(), der.first());

            let ser = None;
            let bytes: DownwardBytes = ser.serialize().unwrap();
            assert_eq!(bytes.len(), 1);
            let der = Option::<String>::deserialize(&bytes[..]).unwrap();
            assert_eq!(ser, der);
            let der = Vec::<String>::deserialize(&bytes[..]).unwrap();
            assert_eq!(ser.as_ref(), der.first());

            assert!(Option::<String>::deserialize([128].as_ref()).is_err());
            assert!(Option::<String>::deserialize([1].as_ref()).is_err());
            assert!(Option::<String>::deserialize([0].as_ref())
                .unwrap()
                .is_none());
        }

        {
            let ser: HashSet<String> = "hello world !".split(' ').map(|s| s.to_owned()).collect();
            let bytes: DownwardBytes = ser.serialize().unwrap();
            let der = HashSet::<String>::deserialize(&bytes[..]).unwrap();
            assert_eq!(ser, der);

            assert!(HashSet::<u8>::deserialize([128].as_ref()).is_err());
            assert!(HashSet::<u8>::deserialize([1].as_ref()).is_err());
            assert!(HashSet::<u8>::deserialize([0].as_ref()).unwrap().is_empty());
        }

        {
            let ser: HashMap<String, u32> = (0..10).map(|i| (i.to_string(), i)).collect();
            let bytes: DownwardBytes = ser.serialize().unwrap();
            let der = HashMap::<String, u32>::deserialize(&bytes[..]).unwrap();
            assert_eq!(ser, der);

            let mut der = Vec::<(String, u32)>::deserialize(&bytes[..]).unwrap();
            assert_eq!(der.len(), 10);
            der.sort();
            assert_eq!(der[0].0, "0");
            assert_eq!(der[9].0, "9");
        }
    }
}
