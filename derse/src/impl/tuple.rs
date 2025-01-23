use crate::*;

macro_rules! tuple_serialization {
    (($($name:ident),+), ($($idx:tt),+)) => {
        impl<'a, $($name),+> Serialize for ($($name,)+)
        where
            $($name: Serialize),+
        {
            fn serialize_to<S: Serializer>(&self, serializer: &mut S) -> Result<()> {
                $((self.$idx.serialize_to(serializer))?;)+
                Ok(())
            }
        }

        impl<'a, $($name),+> Deserialize<'a> for ($($name,)+)
        where
            $($name: Deserialize<'a>),+
        {
            fn deserialize_from<D: Deserializer<'a>>(buf: &mut D) -> Result<Self>
            where
                Self: Sized,
            {
                Ok(($($name::deserialize_from(buf)?,)+))
            }
        }
    };
}

tuple_serialization!((T0), (0));
tuple_serialization!((T0, T1), (1, 0));
tuple_serialization!((T0, T1, T2), (2, 1, 0));
tuple_serialization!((T0, T1, T2, T3), (3, 2, 1, 0));
tuple_serialization!((T0, T1, T2, T3, T4), (4, 3, 2, 1, 0));
tuple_serialization!((T0, T1, T2, T3, T4, T5), (5, 4, 3, 2, 1, 0));
tuple_serialization!((T0, T1, T2, T3, T4, T5, T6), (6, 5, 4, 3, 2, 1, 0));
tuple_serialization!((T0, T1, T2, T3, T4, T5, T6, T7), (7, 6, 5, 4, 3, 2, 1, 0));
tuple_serialization!(
    (T0, T1, T2, T3, T4, T5, T6, T7, T8),
    (8, 7, 6, 5, 4, 3, 2, 1, 0)
);
tuple_serialization!(
    (T0, T1, T2, T3, T4, T5, T6, T7, T8, T9),
    (9, 8, 7, 6, 5, 4, 3, 2, 1, 0)
);
tuple_serialization!(
    (T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10),
    (10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0)
);
tuple_serialization!(
    (T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11),
    (11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0)
);
tuple_serialization!(
    (T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12),
    (12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0)
);
tuple_serialization!(
    (T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13),
    (13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0)
);
tuple_serialization!(
    (T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14),
    (14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0)
);
tuple_serialization!(
    (T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15),
    (15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0)
);
