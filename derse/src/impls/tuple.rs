use crate::*;

macro_rules! tuple_impls {
    (@impl [$($name:ident),+] [$($idx:tt),+]) => {
        impl<$($name),+> Serialize for ($($name,)+)
        where
            $($name: Serialize),+
        {
            fn serialize_to<S: Serializer>(&self, serializer: &mut S) -> Result<()> {
                $(self.$idx.serialize_to(serializer)?;)+
                Ok(())
            }
        }

        impl<'a, $($name),+> Deserialize<'a> for ($($name,)+)
        where
            $($name: Deserialize<'a>),+
        {
            fn deserialize_from<D: Deserializer<'a>>(buf: &mut D) -> Result<Self> {
                Ok(($($name::deserialize_from(buf)?,)+))
            }
        }
    };
    // Extend the tuple one field at a time, prepending indices for reverse writes.
    (@build [$($name:ident,)*] [$($idx:tt,)*]; $next:ident:$next_idx:tt $(, $tail:ident:$tail_idx:tt)*) => {
        tuple_impls!(@impl [$($name,)* $next] [$next_idx $(, $idx)*]);
        tuple_impls!(@build [$($name,)* $next,] [$next_idx, $($idx,)*]; $($tail:$tail_idx),*);
    };
    (@build [$($name:ident,)*] [$($idx:tt,)*];) => {};
    ($($name:ident:$idx:tt),+ $(,)?) => {
        tuple_impls!(@build [] []; $($name:$idx),+);
    };
}

tuple_impls!(
    T0: 0, T1: 1, T2: 2, T3: 3, T4: 4, T5: 5, T6: 6, T7: 7,
    T8: 8, T9: 9, T10: 10, T11: 11, T12: 12, T13: 13, T14: 14, T15: 15,
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tuple_field_order() {
        let value = (3u8, 0x1020u16, true, "hi");
        let expected = [3, 0x20, 0x10, 1, 2, b'h', b'i'];
        let bytes = value.serialize::<DownwardBytes>().unwrap();
        assert_eq!(bytes.as_ref(), expected);

        let mut input = expected.as_slice();
        let decoded = <(u8, u16, bool, &str)>::deserialize_from(&mut input).unwrap();
        assert_eq!(decoded, value);
        assert!(input.is_empty());
    }

    #[test]
    fn test_tuple_supported_arities() {
        let bytes = (42u8,).serialize::<DownwardBytes>().unwrap();
        assert_eq!(bytes.as_ref(), [42]);
        assert_eq!(<(u8,)>::deserialize(bytes.as_ref()).unwrap(), (42,));

        type Wide = (
            u8,
            u8,
            u8,
            u8,
            u8,
            u8,
            u8,
            u8,
            u8,
            u8,
            u8,
            u8,
            u8,
            u8,
            u8,
            u8,
        );
        let value: Wide = (0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15);
        let expected: Vec<u8> = (0..16).collect();
        assert_eq!(
            value.serialize::<DownwardBytes>().unwrap().as_ref(),
            expected
        );

        let mut input = expected.as_slice();
        let decoded = Wide::deserialize_from(&mut input).unwrap();
        assert!(input.is_empty());
        // Standard tuple equality is unavailable for arities above twelve.
        assert_eq!(
            decoded.serialize::<DownwardBytes>().unwrap().as_ref(),
            expected
        );
    }
}
