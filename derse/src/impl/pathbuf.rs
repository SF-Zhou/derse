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

impl<'a> Deserialize<'a> for &'a Path {
    fn deserialize_from<D: Deserializer<'a>>(buf: &mut D) -> Result<Self>
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
        let ser = std::env::current_dir().unwrap();

        let bytes = ser.as_path().serialize::<DownwardBytes>().unwrap();
        let der = <&Path>::deserialize(&bytes[..]).unwrap();
        assert_eq!(ser, der);

        let bytes = ser.serialize::<DownwardBytes>().unwrap();
        let der = PathBuf::deserialize(&bytes[..]).unwrap();
        assert_eq!(ser, der);
    }
}
