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
        Ok(Ipv4Addr::from(bits.to_be()))
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
    fn test_socket_addr() {
        let ipv4 = "120.18.30.27:8888";
        let ipv6 = "[2001:0db8:85a3:0000:0000:8a2e:0370:7334]:8080";

        let ser = SocketAddrV4::from_str(ipv4).unwrap();
        let bytes = ser.serialize::<DownwardBytes>().unwrap();
        assert_eq!(bytes.len(), 6);
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
        let der = SocketAddr::deserialize(&bytes[..]).unwrap();
        assert_eq!(ser, der);

        let ser = SocketAddr::from_str(ipv6).unwrap();
        let bytes = ser.serialize::<DownwardBytes>().unwrap();
        assert_eq!(bytes.len(), 27);
        let der = SocketAddr::deserialize(&bytes[..]).unwrap();
        assert_eq!(ser, der);
    }
}
