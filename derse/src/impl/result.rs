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
