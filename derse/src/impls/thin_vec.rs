//! ThinVec uses Vec's count-prefixed encoding and preserves element order.
//!
//! Decode through a fallible iterator so allocation grows with successfully
//! decoded elements rather than the untrusted count. Keep the count as u64 to
//! avoid truncating it on narrower targets.

use crate::{Deserialize, Deserializer, Result, Serialize, Serializer, VarInt64};
use thin_vec::ThinVec;

impl<T: Serialize> Serialize for ThinVec<T> {
    fn serialize_to<S: Serializer>(&self, serializer: &mut S) -> Result<()> {
        for item in self.iter().rev() {
            item.serialize_to(serializer)?;
        }
        VarInt64(self.len() as u64).serialize_to(serializer)
    }
}

impl<'a, T: Deserialize<'a>> Deserialize<'a> for ThinVec<T> {
    fn deserialize_from<D: Deserializer<'a>>(buf: &mut D) -> Result<Self> {
        let len = VarInt64::deserialize_from(buf)?.0;
        (0..len).map(|_| T::deserialize_from(buf)).collect()
    }
}
