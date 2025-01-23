use crate::*;
use std::borrow::Cow;

impl Serialize for Cow<'_, [u8]> {
    fn serialize_to<S: Serializer>(&self, serializer: &mut S) -> Result<()> {
        <[u8]>::serialize_to(self, serializer)
    }
}

impl<'a> Deserialize<'a> for Cow<'a, [u8]> {
    fn deserialize_from<D: Deserializer<'a>>(buf: &mut D) -> Result<Self>
    where
        Self: Sized,
    {
        let len = VarInt64::deserialize_from(buf)?.0 as usize;
        buf.pop(len)
    }
}

impl Serialize for Cow<'_, str> {
    fn serialize_to<S: Serializer>(&self, serializer: &mut S) -> Result<()> {
        self.as_ref().serialize_to(serializer)
    }
}

impl<'a> Deserialize<'a> for Cow<'a, str> {
    fn deserialize_from<D: Deserializer<'a>>(buf: &mut D) -> Result<Self>
    where
        Self: Sized,
    {
        let len = VarInt64::deserialize_from(buf)?.0 as usize;
        let front = buf.pop(len)?;
        match front {
            Cow::Borrowed(borrowed) => match std::str::from_utf8(borrowed) {
                Ok(str) => Ok(Cow::Borrowed(str)),
                Err(_) => Err(Error::InvalidString(Vec::from(borrowed))),
            },
            Cow::Owned(owned) => match String::from_utf8(owned) {
                Ok(str) => Ok(Cow::Owned(str)),
                Err(e) => Err(Error::InvalidString(e.into_bytes())),
            },
        }
    }
}

impl<T: ToOwned<Owned = T> + Serialize> Serialize for Cow<'_, T> {
    fn serialize_to<S: Serializer>(&self, serializer: &mut S) -> Result<()> {
        self.as_ref().serialize_to(serializer)
    }
}

impl<'a, T: ToOwned<Owned = T> + Deserialize<'a>> Deserialize<'a> for Cow<'a, T> {
    fn deserialize_from<D: Deserializer<'a>>(buf: &mut D) -> Result<Self>
    where
        Self: Sized,
    {
        Ok(Cow::Owned(T::deserialize_from(buf)?))
    }
}
