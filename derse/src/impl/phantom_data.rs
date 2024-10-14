use crate::*;
use std::marker::PhantomData;

impl<T> Serialize for PhantomData<T> {
    fn serialize_to<S: Serializer>(&self, _: &mut S) -> Result<()> {
        Ok(())
    }
}

impl<T: 'static> Deserialize for PhantomData<T> {
    type Output<'a> = PhantomData<T>;

    fn deserialize_from<'a, D: Deserializer<'a>>(_: &mut D) -> Result<Self> {
        Ok(Default::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phantom_data() {
        let ser = std::marker::PhantomData::<()>;
        let bytes = ser.serialize::<DownwardBytes>().unwrap();
        assert!(bytes.is_empty());
        let _ = std::marker::PhantomData::<()>::deserialize(&bytes[..]).unwrap();
    }
}
