use crate::*;
use std::cmp::Eq;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap, HashSet, LinkedList, VecDeque};
use std::hash::Hash;

macro_rules! seq_impl {
    ($ty:ident, [$($bound:path),*], $($method:ident()).+) => {
        impl<T> Serialize for $ty<T>
        where
            T: Serialize,
        {
            #[inline]
            fn serialize_to<S: Serializer>(&self, serializer: &mut S) -> Result<()> {
                for item in self.$($method()).+ {
                    item.serialize_to(serializer)?;
                }
                VarInt64(self.len() as u64).serialize_to(serializer)
            }
        }

        impl<'a, T> Deserialize<'a> for $ty<T>
        where
            T: Deserialize<'a> $(+ $bound)*,
        {
            fn deserialize_from<D: Deserializer<'a>>(buf: &mut D) -> Result<Self> {
                let len = VarInt64::deserialize_from(buf)?.0 as usize;
                (0..len).map(|_| T::deserialize_from(buf)).collect::<Result<Self>>()
            }
        }
    };
}

seq_impl!(Vec, [], iter().rev());
seq_impl!(VecDeque, [], iter().rev());
seq_impl!(LinkedList, [], iter().rev());
seq_impl!(BinaryHeap, [Ord], iter().rev());
seq_impl!(BTreeSet, [Ord], iter());
seq_impl!(HashSet, [Eq, Hash], iter());

macro_rules! map_impl {
    ($ty:ident, [$($bound:path),*]) => {
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
            K: Deserialize<'a> $(+ $bound)*,
            V: Deserialize<'a>,
        {
            #[inline]
            fn deserialize_from<D: Deserializer<'a>>(buf: &mut D) -> Result<Self> {
                let len = VarInt64::deserialize_from(buf)?.0 as usize;
                (0..len).map(|_| <(K, V)>::deserialize_from(buf)).collect::<Result<Self>>()
            }
        }
    };
}

map_impl!(BTreeMap, [Ord]);
map_impl!(HashMap, [Eq, Hash]);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sequence_encoding_preserves_iteration_order() {
        let mut deque = VecDeque::with_capacity(3);
        deque.extend([9u8, 1, 2]);
        deque.pop_front();
        deque.push_back(3);
        let bytes: DownwardBytes = deque.serialize().unwrap();
        assert_eq!(&bytes[..], &[3, 1, 2, 3]);
        assert_eq!(VecDeque::<u8>::deserialize(&bytes[..]).unwrap(), deque);

        let heap = BinaryHeap::from(vec![3u8, 1, 2]);
        let mut expected = vec![3];
        expected.extend(heap.iter().copied());
        let bytes: DownwardBytes = heap.serialize().unwrap();
        assert_eq!(bytes.as_ref(), expected);
        let decoded = BinaryHeap::<u8>::deserialize(&bytes[..]).unwrap();
        assert_eq!(decoded.into_sorted_vec(), heap.into_sorted_vec());
    }

    #[test]
    fn ordered_collections_encode_largest_key_first() {
        let set = BTreeSet::from([1u8, 2, 3]);
        let bytes: DownwardBytes = set.serialize().unwrap();
        assert_eq!(&bytes[..], &[3, 3, 2, 1]);
        assert_eq!(BTreeSet::<u8>::deserialize(&bytes[..]).unwrap(), set);

        let map = BTreeMap::from([(1u8, 10u8), (2, 20)]);
        let bytes: DownwardBytes = map.serialize().unwrap();
        assert_eq!(&bytes[..], &[2, 2, 20, 1, 10]);
        assert_eq!(BTreeMap::<u8, u8>::deserialize(&bytes[..]).unwrap(), map);
    }

    #[test]
    fn serialization_does_not_require_collection_key_bounds() {
        struct SerializeOnly(u8);

        impl Serialize for SerializeOnly {
            fn serialize_to<S: Serializer>(&self, serializer: &mut S) -> Result<()> {
                self.0.serialize_to(serializer)
            }
        }

        fn assert_empty_encoding(value: impl Serialize) {
            let bytes: DownwardBytes = value.serialize().unwrap();
            assert_eq!(&bytes[..], &[0]);
        }

        assert_empty_encoding(BinaryHeap::<SerializeOnly>::default());
        assert_empty_encoding(BTreeSet::<SerializeOnly>::default());
        assert_empty_encoding(HashSet::<SerializeOnly>::default());
        assert_empty_encoding(BTreeMap::<SerializeOnly, SerializeOnly>::default());
        assert_empty_encoding(HashMap::<SerializeOnly, SerializeOnly>::default());

        let values = vec![SerializeOnly(1), SerializeOnly(2)];
        let bytes: DownwardBytes = values.serialize().unwrap();
        assert_eq!(&bytes[..], &[2, 1, 2]);
    }

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
            let ser: LinkedList<_> = (0..10).collect();
            let bytes: DownwardBytes = ser.serialize().unwrap();
            let der = LinkedList::<i32>::deserialize(&bytes[..]).unwrap();
            assert_eq!(ser, der);
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
