//! IP addresses use network-order octets without a prefix; socket fields then
//! follow as little-endian integers: port, and for IPv6 flowinfo and scope_id.
//! SocketAddr adds a boolean family tag (0 for IPv4, 1 for IPv6).

use crate::*;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};

impl Serialize for Ipv4Addr {
    fn serialize_to<S: Serializer>(&self, serializer: &mut S) -> Result<()> {
        self.octets().serialize_to(serializer)
    }
}

impl<'a> Deserialize<'a> for Ipv4Addr {
    fn deserialize_from<D: Deserializer<'a>>(buf: &mut D) -> Result<Self>
    where
        Self: Sized,
    {
        let bits = u32::deserialize_from(buf)?;
        // Restore the wire octets after the little-endian read on either host endian.
        Ok(Ipv4Addr::from(bits.to_le_bytes()))
    }
}

impl Serialize for SocketAddrV4 {
    fn serialize_to<S: Serializer>(&self, serializer: &mut S) -> Result<()> {
        self.port().serialize_to(serializer)?;
        self.ip().serialize_to(serializer)
    }
}

impl<'a> Deserialize<'a> for SocketAddrV4 {
    fn deserialize_from<D: Deserializer<'a>>(buf: &mut D) -> Result<Self>
    where
        Self: Sized,
    {
        let ip = Ipv4Addr::deserialize_from(buf)?;
        let port = u16::deserialize_from(buf)?;
        Ok(SocketAddrV4::new(ip, port))
    }
}

impl Serialize for Ipv6Addr {
    fn serialize_to<S: Serializer>(&self, serializer: &mut S) -> Result<()> {
        self.octets().serialize_to(serializer)
    }
}

impl<'a> Deserialize<'a> for Ipv6Addr {
    fn deserialize_from<D: Deserializer<'a>>(buf: &mut D) -> Result<Self>
    where
        Self: Sized,
    {
        let bits = <[u8; 16]>::deserialize_from(buf)?;
        Ok(Ipv6Addr::from(bits))
    }
}

impl Serialize for SocketAddrV6 {
    fn serialize_to<S: Serializer>(&self, serializer: &mut S) -> Result<()> {
        self.scope_id().serialize_to(serializer)?;
        self.flowinfo().serialize_to(serializer)?;
        self.port().serialize_to(serializer)?;
        self.ip().serialize_to(serializer)
    }
}

impl<'a> Deserialize<'a> for SocketAddrV6 {
    fn deserialize_from<D: Deserializer<'a>>(buf: &mut D) -> Result<Self>
    where
        Self: Sized,
    {
        let ip = Ipv6Addr::deserialize_from(buf)?;
        let port = u16::deserialize_from(buf)?;
        let flowinfo = u32::deserialize_from(buf)?;
        let scope_id = u32::deserialize_from(buf)?;
        Ok(SocketAddrV6::new(ip, port, flowinfo, scope_id))
    }
}

impl Serialize for SocketAddr {
    fn serialize_to<S: Serializer>(&self, serializer: &mut S) -> Result<()> {
        match self {
            SocketAddr::V4(v4) => {
                v4.serialize_to(serializer)?;
                false.serialize_to(serializer)?;
            }
            SocketAddr::V6(v6) => {
                v6.serialize_to(serializer)?;
                true.serialize_to(serializer)?;
            }
        }
        Ok(())
    }
}

impl<'a> Deserialize<'a> for SocketAddr {
    fn deserialize_from<D: Deserializer<'a>>(buf: &mut D) -> Result<Self>
    where
        Self: Sized,
    {
        Ok(match bool::deserialize_from(buf)? {
            false => SocketAddr::V4(SocketAddrV4::deserialize_from(buf)?),
            true => SocketAddr::V6(SocketAddrV6::deserialize_from(buf)?),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn ipv4_decode_wire_octets() {
        for octets in [
            [120, 18, 30, 27],
            [127, 0, 0, 1],
            [0, 0, 0, 0],
            [255, 255, 255, 255],
            [1, 2, 128, 254],
        ] {
            let expected = Ipv4Addr::from(octets);
            assert_eq!(Ipv4Addr::deserialize(octets.as_slice()).unwrap(), expected);

            let fragments = [&octets[..1], &octets[1..3], &octets[3..], &[42]];
            let mut input = BytesArray::new(&fragments);
            assert_eq!(Ipv4Addr::deserialize_from(&mut input).unwrap(), expected);
            assert_eq!(input.pop(1).unwrap().as_ref(), &[42]);
        }
    }

    #[test]
    fn ipv4_decode_socket_wire_fields() {
        let expected = SocketAddrV4::new(Ipv4Addr::new(120, 18, 30, 27), 0x1234);
        let wire = [120, 18, 30, 27, 0x34, 0x12, 42];
        let mut input = wire.as_slice();
        assert_eq!(
            SocketAddrV4::deserialize_from(&mut input).unwrap(),
            expected
        );
        assert_eq!(input, &[42]);

        let tagged_wire = [0, 120, 18, 30, 27, 0x34, 0x12, 42];
        let fragments = [&tagged_wire[..2], &tagged_wire[2..6], &tagged_wire[6..]];
        let mut input = BytesArray::new(&fragments);
        assert_eq!(
            SocketAddr::deserialize_from(&mut input).unwrap(),
            SocketAddr::V4(expected)
        );
        assert_eq!(input.pop(1).unwrap().as_ref(), &[42]);
    }

    #[test]
    fn ipv4_decode_short_input_preserves_cursor() {
        let octets = [120, 18, 30, 27];
        for len in 0..4 {
            let wire = &octets[..len];
            let expected = Error::DataIsShort {
                expect: 4,
                actual: len,
            };
            let mut input = wire;
            assert_eq!(
                Ipv4Addr::deserialize_from(&mut input),
                Err(expected.clone())
            );
            assert_eq!(input, wire);

            let fragments = [&wire[..len / 2], &wire[len / 2..]];
            let mut input = BytesArray::new(&fragments);
            assert_eq!(Ipv4Addr::deserialize_from(&mut input), Err(expected));
            assert_eq!(input.len(), len);
            assert_eq!(input.pop(len).unwrap().as_ref(), wire);
        }
    }

    #[test]
    fn test_socket_addr() {
        let ipv4 = "120.18.30.27:8888";
        let ipv6 = "[2001:0db8:85a3:0000:0000:8a2e:0370:7334]:8080";

        let ser = SocketAddrV4::from_str(ipv4).unwrap();
        let bytes = ser.serialize::<DownwardBytes>().unwrap();
        assert_eq!(bytes.len(), 6);
        assert_eq!(&bytes[..], &[120, 18, 30, 27, 0xb8, 0x22]);
        assert_eq!(
            &ser.ip().serialize::<DownwardBytes>().unwrap()[..],
            &[120, 18, 30, 27]
        );
        let der = SocketAddrV4::deserialize(&bytes[..]).unwrap();
        assert_eq!(ser, der);

        let ser = SocketAddrV6::from_str(ipv6).unwrap();
        let bytes = ser.serialize::<DownwardBytes>().unwrap();
        assert_eq!(bytes.len(), 26);
        let der = SocketAddrV6::deserialize(&bytes[..]).unwrap();
        assert_eq!(ser, der);

        let ser = SocketAddr::from_str(ipv4).unwrap();
        let bytes = ser.serialize::<DownwardBytes>().unwrap();
        assert_eq!(bytes.len(), 7);
        assert_eq!(&bytes[..], &[0, 120, 18, 30, 27, 0xb8, 0x22]);
        let der = SocketAddr::deserialize(&bytes[..]).unwrap();
        assert_eq!(ser, der);

        let ser = SocketAddr::from_str(ipv6).unwrap();
        let bytes = ser.serialize::<DownwardBytes>().unwrap();
        assert_eq!(bytes.len(), 27);
        let der = SocketAddr::deserialize(&bytes[..]).unwrap();
        assert_eq!(ser, der);
    }
}
