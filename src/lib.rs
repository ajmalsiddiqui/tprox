use std::io::Write;
use std::net::TcpStream;

use utils::SerializableBe;

#[derive(Debug)]
pub enum ProxyHeaderType {
    Start,
    Data,
    Fin,
}

impl ProxyHeaderType {
    pub fn from_byte(byte: u8) -> Result<ProxyHeaderType, &'static str> {
        match byte {
            0x1 => Ok(ProxyHeaderType::Start),
            0x2 => Ok(ProxyHeaderType::Data),
            0x3 => Ok(ProxyHeaderType::Fin),
            _ => Err("Invalid byte"),
        }
    }
}

impl utils::SerializableBe for ProxyHeaderType {
    // TODO this should return a Bytes/BytesMut
    fn serialize_be(&self) -> Vec<u8> {
        match self {
            ProxyHeaderType::Start => vec![0x1],
            ProxyHeaderType::Data => vec![0x2],
            ProxyHeaderType::Fin => vec![0x3],
        }
    }
}

#[derive(Debug)]
pub struct ProxyHeader {
    // TODO maybe conn_id and port should be Options?
    /// Control messages that are connection agnostic will have a conn_id of 0
    pub conn_id: u16,
    pub port: u16,
    pub header_type: ProxyHeaderType,
    pub data_len: u32,
}

impl ProxyHeader {
    pub const HEADER_LEN: usize = 2 + 2 + 1 + 4;

    pub fn from_be_bytes(bytes: &[u8; ProxyHeader::HEADER_LEN]) -> Result<ProxyHeader, &'static str> {
        // TODO handle error
        let conn_id_bytes = bytes[..2].try_into().unwrap();
        // TODO handle error
        let port_bytes = bytes[2..4].try_into().unwrap();
        let header_type_byte = bytes[4];
        // TODO handle error
        let data_len_bytes = bytes[5..].try_into().unwrap();

        let conn_id = u16::from_be_bytes(conn_id_bytes);
        let port = u16::from_be_bytes(port_bytes);
        let header_type = ProxyHeaderType::from_byte(header_type_byte)?;
        let data_len = u32::from_be_bytes(data_len_bytes);

        Ok(ProxyHeader {
            conn_id,
            port,
            header_type,
            data_len,
        })
    }

    pub fn make_start_header(port: u16) -> ProxyHeader {
        ProxyHeader {
            conn_id: 0,
            port: port,
            header_type: ProxyHeaderType::Start,
            data_len: 0,
        }
    }
}

impl utils::SerializableBe for ProxyHeader {
    fn serialize_be(&self) -> Vec<u8> {
        let mut serialized = Vec::with_capacity(ProxyHeader::HEADER_LEN);

        serialized.extend_from_slice(&self.conn_id.to_be_bytes());
        serialized.extend_from_slice(&self.port.to_be_bytes());
        serialized.extend_from_slice(&self.header_type.serialize_be());
        serialized.extend_from_slice(&self.data_len.to_be_bytes());

        serialized
    }
}

/// Proxies a connection one-way from src_conn to dst_conn
/// While adding a ProxyHeader header to each chunk of data sent to dst_conn
pub struct ProxyConn {
    /// TcpStream conn representing the connection to the target
    pub dst_conn: TcpStream,
    /// Connection ID. Should be unique for a given target
    pub conn_id: u16,
    /// The port of the proxy server that is proxying this request. Can be set to 0 to indicate
    /// that this should be ignored
    pub port: u16,
}

// All writes are done against the dst_conn
impl Write for ProxyConn {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let header = ProxyHeader {
            conn_id: self.conn_id,
            port: self.port,
            header_type: ProxyHeaderType::Data,
            data_len: buf.len() as u32,
        };

        // TODO do I need to handle the case where not all the bytes are written to the target?
        // I probably do
        if let Err(e) = self.dst_conn.write(&header.serialize_be().as_slice()) {
            return Err(e);
        }

        self.dst_conn.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.dst_conn.flush()
    }
}

pub mod utils {
    use std::net::TcpStream;
    use std::io::Read;
    use std::io;

    pub struct Counter {
        count: u16,
    }

    impl Counter {
        pub fn new() -> Self {
            Counter {
                count: 0
            }
        }

        pub fn increment(&mut self) -> u16 {
            self.count += 1;
            self.count
        }
    }

    // Represents objects that can be serialized as bytes in big endian order
    pub trait SerializableBe {
        fn serialize_be(&self) -> Vec<u8>;
    }

    // If this fn succeeds, it means that we've read enough from the stream to fill the buffer
    pub fn fill_buf_from_stream (client_conn: &mut TcpStream, buf: &mut [u8]) -> io::Result<()> {
        if buf.len() == 0 {
            return Ok(());
        }

        match client_conn.read(buf) {
            Ok(size) => {
                // We've already checked for a zero sized buffer here, so if we get a zero sized
                // read, that means that the client has terminated the connection
                if size == 0 {
                    // TODO close the conn here?
                    return Err(io::Error::from(io::ErrorKind::UnexpectedEof));
                }
                // TODO optimize: this implementation ensures that this func runs at least twice:
                // if you read the entire buf, size will be equal to buflen, and then it'll recur
                // again with a buffer of length zero, and then it'll hit this condition and return

                // Try to read some more until the header buffer is full
                fill_buf_from_stream(client_conn, &mut buf[size..])
            },
            Err(e) => Err(e),
        }
    }
}
