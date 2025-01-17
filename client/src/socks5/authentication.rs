use super::protocol::HandshakeMethod;

#[allow(unused)]
pub enum Authentication {
    None,
    Gssapi,
    Password {
        username: Vec<u8>,
        password: Vec<u8>,
    },
}

impl Authentication {
    pub fn as_handshake_method(&self) -> HandshakeMethod {
        match self {
            Authentication::None => HandshakeMethod::None,
            Authentication::Gssapi => HandshakeMethod::Gssapi,
            Authentication::Password { .. } => HandshakeMethod::Password,
        }
    }
}
