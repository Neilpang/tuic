use crate::{Address, Error, TUIC_PROTOCOL_VERSION};
use bytes::{BufMut, BytesMut};
use std::io::Result as IoResult;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// Command
///
/// ```plain
/// +-----+-----+----------+
/// | VER | CMD |   OPT    |
/// +-----+-----+----------+
/// |  1  |  1  | Variable |
/// +-----+-----+----------+
/// ```
#[derive(Clone)]
pub enum Command {
    Authenticate {
        digest: [u8; 32],
    },
    Connect {
        addr: Address,
    },
    Bind {
        addr: Address,
    },
    Packet {
        assoc_id: u32,
        len: u16,
        addr: Address,
    },
    Dissociate {
        assoc_id: u32,
    },
}

impl Command {
    const CMD_AUTHENTICATE: u8 = 0x00;
    const CMD_CONNECT: u8 = 0x01;
    const CMD_BIND: u8 = 0x02;
    const CMD_PACKET: u8 = 0x03;
    const CMD_DISSOCIATE: u8 = 0x04;

    pub fn new_authenticate(digest: [u8; 32]) -> Self {
        Self::Authenticate { digest }
    }

    pub fn new_connect(addr: Address) -> Self {
        Self::Connect { addr }
    }

    pub fn new_bind(addr: Address) -> Self {
        Self::Bind { addr }
    }

    pub fn new_packet(assoc_id: u32, len: u16, addr: Address) -> Self {
        Self::Packet {
            assoc_id,
            len,
            addr,
        }
    }

    pub fn new_dissociate(assoc_id: u32) -> Self {
        Self::Dissociate { assoc_id }
    }

    pub async fn read_from<R>(r: &mut R) -> Result<Self, Error>
    where
        R: AsyncRead + Unpin,
    {
        let mut buf = [0; 2];
        r.read_exact(&mut buf).await?;

        let ver = buf[0];
        let cmd = buf[1];

        if ver != TUIC_PROTOCOL_VERSION {
            return Err(Error::UnsupportedVersion(ver));
        }

        match cmd {
            Self::CMD_AUTHENTICATE => {
                let mut digest = [0; 32];
                r.read_exact(&mut digest).await?;
                Ok(Self::new_authenticate(digest))
            }
            Self::CMD_CONNECT => {
                let addr = Address::read_from(r).await?;
                Ok(Self::new_connect(addr))
            }
            Self::CMD_BIND => {
                let addr = Address::read_from(r).await?;
                Ok(Self::new_bind(addr))
            }
            Self::CMD_PACKET => {
                let mut buf = [0; 6];
                r.read_exact(&mut buf).await?;

                let assoc_id = unsafe { u32::from_be(*(buf.as_ptr() as *const u32)) };
                let len = unsafe { u16::from_be(*(buf.as_ptr().add(4) as *const u16)) };
                let addr = Address::read_from(r).await?;

                Ok(Self::new_packet(assoc_id, len, addr))
            }
            Self::CMD_DISSOCIATE => {
                let assoc_id = r.read_u32().await?;
                Ok(Self::new_dissociate(assoc_id))
            }
            _ => Err(Error::UnsupportedCommand(cmd)),
        }
    }

    pub async fn write_to<W>(&self, w: &mut W) -> IoResult<()>
    where
        W: AsyncWrite + Unpin,
    {
        let mut buf = BytesMut::with_capacity(self.serialized_len());
        self.write_to_buf(&mut buf);
        w.write_all(&buf).await
    }

    pub fn write_to_buf<B: BufMut>(&self, buf: &mut B) {
        buf.put_u8(TUIC_PROTOCOL_VERSION);

        match self {
            Self::Authenticate { digest } => {
                buf.put_u8(Self::CMD_AUTHENTICATE);
                buf.put_slice(digest);
            }
            Self::Connect { addr } => {
                buf.put_u8(Self::CMD_CONNECT);
                addr.write_to_buf(buf);
            }
            Self::Bind { addr } => {
                buf.put_u8(Self::CMD_BIND);
                addr.write_to_buf(buf);
            }
            Self::Packet {
                assoc_id,
                len,
                addr,
            } => {
                buf.put_u8(Self::CMD_PACKET);
                buf.put_u32(*assoc_id);
                buf.put_u16(*len);
                addr.write_to_buf(buf);
            }
            Self::Dissociate { assoc_id } => {
                buf.put_u8(Self::CMD_DISSOCIATE);
                buf.put_u32(*assoc_id);
            }
        }
    }

    pub fn serialized_len(&self) -> usize {
        2 + match self {
            Self::Authenticate { .. } => 32,
            Self::Connect { addr } => addr.serialized_len(),
            Self::Bind { addr } => addr.serialized_len(),
            Self::Packet { addr, .. } => 6 + addr.serialized_len(),
            Self::Dissociate { .. } => 4,
        }
    }
}
