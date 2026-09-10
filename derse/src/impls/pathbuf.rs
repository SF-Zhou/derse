use crate::*;
use std::{
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
};

impl Serialize for Path {
    fn serialize_to<S: Serializer>(&self, serializer: &mut S) -> Result<()> {
        self.as_os_str().serialize_to(serializer)
    }
}

impl<'de: 'a, 'a> Deserialize<'de> for &'a Path {
    fn deserialize_from<D: Deserializer<'de>>(buf: &mut D) -> Result<Self>
    where
        Self: Sized,
    {
        <&OsStr>::deserialize_from(buf).map(Path::new)
    }
}

impl Serialize for PathBuf {
    fn serialize_to<S: Serializer>(&self, serializer: &mut S) -> Result<()> {
        self.as_os_str().serialize_to(serializer)
    }
}

impl<'a> Deserialize<'a> for PathBuf {
    fn deserialize_from<D: Deserializer<'a>>(buf: &mut D) -> Result<Self>
    where
        Self: Sized,
    {
        OsString::deserialize_from(buf).map(PathBuf::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path() {
        fn deserialize_shorter<'de: 'a, 'a>(bytes: &'de [u8]) -> &'a Path {
            <&'a Path as Deserialize<'de>>::deserialize(bytes).unwrap()
        }

        let ser = std::env::current_dir().unwrap();

        let bytes = ser.as_path().serialize::<DownwardBytes>().unwrap();
        let der = deserialize_shorter(&bytes[..]);
        assert_eq!(ser, der);

        let bytes = ser.serialize::<DownwardBytes>().unwrap();
        let der = PathBuf::deserialize(&bytes[..]).unwrap();
        assert_eq!(ser, der);
    }
}
