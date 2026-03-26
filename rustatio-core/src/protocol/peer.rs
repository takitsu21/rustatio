use thiserror::Error;

pub const PROTOCOL_STR: &str = "BitTorrent protocol";
pub const PROTOCOL_LEN: u8 = 19;
pub const RESERVED_LEN: usize = 8;
pub const INFO_HASH_LEN: usize = 20;
pub const PEER_ID_LEN: usize = 20;
pub const HANDSHAKE_LEN: usize = 68;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerHandshake {
    pub reserved: [u8; RESERVED_LEN],
    pub info_hash: [u8; INFO_HASH_LEN],
    pub peer_id: [u8; PEER_ID_LEN],
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum PeerProtocolError {
    #[error("invalid handshake length: {0}")]
    InvalidLength(usize),
    #[error("invalid protocol string length: {0}")]
    InvalidProtocolLength(u8),
    #[error("invalid protocol string")]
    InvalidProtocol,
    #[error("invalid peer id length: {0}")]
    InvalidPeerIdLength(usize),
}

impl PeerHandshake {
    pub const fn new(
        reserved: [u8; RESERVED_LEN],
        info_hash: [u8; INFO_HASH_LEN],
        peer_id: [u8; PEER_ID_LEN],
    ) -> Self {
        Self { reserved, info_hash, peer_id }
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, PeerProtocolError> {
        if bytes.len() != HANDSHAKE_LEN {
            return Err(PeerProtocolError::InvalidLength(bytes.len()));
        }

        let pstrlen = bytes[0];
        if pstrlen != PROTOCOL_LEN {
            return Err(PeerProtocolError::InvalidProtocolLength(pstrlen));
        }

        if &bytes[1..=usize::from(PROTOCOL_LEN)] != PROTOCOL_STR.as_bytes() {
            return Err(PeerProtocolError::InvalidProtocol);
        }

        let mut reserved = [0u8; RESERVED_LEN];
        reserved.copy_from_slice(&bytes[20..28]);

        let mut info_hash = [0u8; INFO_HASH_LEN];
        info_hash.copy_from_slice(&bytes[28..48]);

        let mut peer_id = [0u8; PEER_ID_LEN];
        peer_id.copy_from_slice(&bytes[48..68]);

        Ok(Self { reserved, info_hash, peer_id })
    }

    pub fn to_bytes(&self) -> [u8; HANDSHAKE_LEN] {
        let mut out = [0u8; HANDSHAKE_LEN];
        out[0] = PROTOCOL_LEN;
        out[1..20].copy_from_slice(PROTOCOL_STR.as_bytes());
        out[20..28].copy_from_slice(&self.reserved);
        out[28..48].copy_from_slice(&self.info_hash);
        out[48..68].copy_from_slice(&self.peer_id);
        out
    }
}

pub const fn peer_id_to_array(peer_id: &str) -> Result<[u8; PEER_ID_LEN], PeerProtocolError> {
    if peer_id.len() != PEER_ID_LEN {
        return Err(PeerProtocolError::InvalidPeerIdLength(peer_id.len()));
    }

    let mut out = [0u8; PEER_ID_LEN];
    out.copy_from_slice(peer_id.as_bytes());
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_handshake() -> PeerHandshake {
        PeerHandshake::new([0u8; RESERVED_LEN], [7u8; INFO_HASH_LEN], *b"-UT0001-123456789012")
    }

    #[test]
    fn handshake_roundtrip() {
        let hs = sample_handshake();
        let parsed = PeerHandshake::from_bytes(&hs.to_bytes());
        assert!(parsed.is_ok());
        assert_eq!(parsed.unwrap_or_else(|_| unreachable!()), hs);
    }

    #[test]
    fn handshake_rejects_bad_length() {
        let err = PeerHandshake::from_bytes(&[0u8; 10]);
        assert_eq!(err, Err(PeerProtocolError::InvalidLength(10)));
    }

    #[test]
    fn handshake_rejects_bad_protocol_length() {
        let mut bytes = sample_handshake().to_bytes();
        bytes[0] = 18;
        let err = PeerHandshake::from_bytes(&bytes);
        assert_eq!(err, Err(PeerProtocolError::InvalidProtocolLength(18)));
    }

    #[test]
    fn handshake_rejects_bad_protocol() {
        let mut bytes = sample_handshake().to_bytes();
        bytes[1] = b'X';
        let err = PeerHandshake::from_bytes(&bytes);
        assert_eq!(err, Err(PeerProtocolError::InvalidProtocol));
    }

    #[test]
    fn peer_id_to_array_accepts_twenty_bytes() {
        let arr = peer_id_to_array("-UT0001-123456789012");
        assert!(arr.is_ok());
        assert_eq!(arr.unwrap_or_else(|_| unreachable!()), *b"-UT0001-123456789012");
    }

    #[test]
    fn peer_id_to_array_rejects_wrong_length() {
        let err = peer_id_to_array("short");
        assert_eq!(err, Err(PeerProtocolError::InvalidPeerIdLength(5)));
    }
}
