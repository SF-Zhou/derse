//! Owning pointers encode only their contents, without an ownership tag.
//!
//! Each decoder constructs a new pointer. Rc/Arc sharing and reference counts
//! are not preserved, and cycles cannot be encoded. Unsized targets decode
//! through their owned counterparts so fragmented payloads remain supported.

use crate::*;
use std::{
    borrow::Cow,
    ffi::{CStr, CString, OsStr, OsString},
    path::{Path, PathBuf},
    rc::Rc,
    sync::Arc,
};

macro_rules! pointer_dst_impl {
    ($pointer:ident, $target:ty, $owned:ty) => {
        impl<'a> Deserialize<'a> for $pointer<$target> {
            fn deserialize_from<D: Deserializer<'a>>(buf: &mut D) -> Result<Self> {
                <$owned as Deserialize<'a>>::deserialize_from(buf).map(Self::from)
            }
        }
    };
}

macro_rules! pointer_impl {
    ($pointer:ident) => {
        impl<T: Serialize + ?Sized> Serialize for $pointer<T> {
            fn serialize_to<S: Serializer>(&self, serializer: &mut S) -> Result<()> {
                <T as Serialize>::serialize_to(self.as_ref(), serializer)
            }
        }

        impl<'a, T: Deserialize<'a>> Deserialize<'a> for $pointer<T> {
            fn deserialize_from<D: Deserializer<'a>>(buf: &mut D) -> Result<Self> {
                <T as Deserialize<'a>>::deserialize_from(buf).map(Self::new)
            }
        }

        impl<'a> Deserialize<'a> for $pointer<[u8]> {
            fn deserialize_from<D: Deserializer<'a>>(buf: &mut D) -> Result<Self> {
                Ok(Self::from(Cow::<[u8]>::deserialize_from(buf)?.into_owned()))
            }
        }

        pointer_dst_impl!($pointer, str, String);
        pointer_dst_impl!($pointer, CStr, CString);
        pointer_dst_impl!($pointer, OsStr, OsString);
        pointer_dst_impl!($pointer, Path, PathBuf);
    };
}

pointer_impl!(Box);
pointer_impl!(Rc);
pointer_impl!(Arc);
