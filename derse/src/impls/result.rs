//! Result uses a boolean tag followed by one payload: 1 for Ok and 0 for Err.
//! This is distinct from the name-tagged encoding generated for user-defined enums.

use crate::*;

impl<T: Serialize, E: Serialize> Serialize for std::result::Result<T, E> {
    fn serialize_to<S: Serializer>(&self, serializer: &mut S) -> Result<()> {
        match self {
            Ok(t) => {
                t.serialize_to(serializer)?;
                true.serialize_to(serializer)
            }
            Err(e) => {
                e.serialize_to(serializer)?;
                false.serialize_to(serializer)
            }
        }
    }
}

impl<'a, T: Deserialize<'a>, E: Deserialize<'a>> Deserialize<'a> for std::result::Result<T, E> {
    fn deserialize_from<D: Deserializer<'a>>(buf: &mut D) -> Result<Self>
    where
        Self: Sized,
    {
        let has = bool::deserialize_from(buf)?;
        if has {
            Ok(Ok(T::deserialize_from(buf)?))
        } else {
            Ok(Err(E::deserialize_from(buf)?))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn result_propagates_write_errors_before_adding_the_tag() {
        for value in [Ok::<u8, u8>(7), Err(9)] {
            let mut serializer = crate::serializer::tests::FailingSerializer::default();
            assert_eq!(
                value.serialize_to(&mut serializer),
                Err(Error::InvalidValue("write failed".into()))
            );
            assert_eq!(serializer.writes, 1);
        }
    }

    #[test]
    fn result_rejects_invalid_tags_and_truncated_payloads() {
        type Value = std::result::Result<u16, u16>;

        assert_eq!(Value::deserialize(&[2][..]), Err(Error::InvalidBool(2)));
        for bytes in [&[][..], &[0][..], &[1][..], &[0, 7][..], &[1, 7][..]] {
            assert!(matches!(
                Value::deserialize(bytes),
                Err(Error::DataIsShort { .. })
            ));
        }

        for (value, encoded) in [(Ok(7), [1, 7, 0]), (Err(9), [0, 9, 0])] {
            let bytes = value.serialize::<DownwardBytes>().unwrap();
            assert_eq!(bytes.as_ref(), encoded);
            assert_eq!(Value::deserialize(&encoded[..]).unwrap(), value);
        }
    }

    #[test]
    fn test_result() {
        let ser = Result::Ok(233i32);
        let bytes = ser.serialize::<DownwardBytes>().unwrap();
        assert_eq!(bytes.len(), 1 + 4);
        let der = Result::deserialize(&bytes[..]).unwrap();
        assert_eq!(ser, der);

        let ser = Result::<()>::Err(Error::VarintIsShort);
        let bytes = ser.serialize::<DownwardBytes>().unwrap();
        assert_eq!(bytes.len(), 1 + 1 + 1 + 13);
        let der = Result::deserialize(&bytes[..]).unwrap();
        assert_eq!(ser, der);
    }
}
