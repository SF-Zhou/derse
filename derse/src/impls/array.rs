use crate::*;

impl<T: Serialize, const N: usize> Serialize for [T; N] {
    fn serialize_to<S: Serializer>(&self, serializer: &mut S) -> Result<()> {
        for item in self.iter().rev() {
            item.serialize_to(serializer)?;
        }
        Ok(())
    }
}

macro_rules! array_impls {
    ($($len:expr => ($($n:tt)+))+) => {
        $(
            impl<'a, T: Deserialize<'a>> Deserialize<'a> for [T; $len] {
                fn deserialize_from<D: Deserializer<'a>>(buf: &mut D) -> Result<Self>
                where
                    Self: Sized,
                {
                    Ok([$(
                        match T::deserialize_from(buf) {
                            Ok(val) => val,
                            Err(e) => return Err(Error::InvalidLength($n, e.to_string())),
                        }
                    ),+])
                }
            }
        )+
    }
}

array_impls! {
    1 => (0)
    2 => (0 1)
    3 => (0 1 2)
    4 => (0 1 2 3)
    5 => (0 1 2 3 4)
    6 => (0 1 2 3 4 5)
    7 => (0 1 2 3 4 5 6)
    8 => (0 1 2 3 4 5 6 7)
    9 => (0 1 2 3 4 5 6 7 8)
    10 => (0 1 2 3 4 5 6 7 8 9)
    11 => (0 1 2 3 4 5 6 7 8 9 10)
    12 => (0 1 2 3 4 5 6 7 8 9 10 11)
    13 => (0 1 2 3 4 5 6 7 8 9 10 11 12)
    14 => (0 1 2 3 4 5 6 7 8 9 10 11 12 13)
    15 => (0 1 2 3 4 5 6 7 8 9 10 11 12 13 14)
    16 => (0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15)
    17 => (0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16)
    18 => (0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17)
    19 => (0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18)
    20 => (0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19)
    21 => (0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20)
    22 => (0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21)
    23 => (0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22)
    24 => (0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23)
    25 => (0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24)
    26 => (0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25)
    27 => (0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26)
    28 => (0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27)
    29 => (0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27 28)
    30 => (0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27 28 29)
    31 => (0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27 28 29 30)
    32 => (0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27 28 29 30 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_array() {
        {
            let ser: [i32; 3] = [1, 2, 3];
            let bytes: DownwardBytes = ser.serialize().unwrap();
            let der = <[i32; 3]>::deserialize(&bytes[..]).unwrap();
            assert_eq!(ser, der);

            <[bool; 3]>::deserialize([].as_slice()).unwrap_err();
            <[bool; 3]>::deserialize([1].as_slice()).unwrap_err();
            <[bool; 3]>::deserialize([2].as_slice()).unwrap_err();
            <[bool; 3]>::deserialize([1, 1].as_slice()).unwrap_err();
            <[bool; 3]>::deserialize([1, 2].as_slice()).unwrap_err();
            <[bool; 3]>::deserialize([1, 1, 1].as_slice()).unwrap();
            <[bool; 3]>::deserialize([1, 1, 2].as_slice()).unwrap_err();
        }

        {
            let ser = 233u32;
            let bytes = ser.serialize::<DownwardBytes>().unwrap();
            let array = <[u8; 4]>::deserialize(&bytes[..]).unwrap();
            let der = u32::from_le_bytes(array);
            assert_eq!(ser, der);
        }
    }
}
