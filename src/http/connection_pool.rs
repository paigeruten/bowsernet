use rustls::{pki_types::ServerName, ClientConfig};
use rustls_platform_verifier::ConfigVerifierExt;
use std::{
    collections::HashMap,
    io::{BufReader, Read, Write},
    net::TcpStream,
    sync::Arc,
};

use crate::url::HttpUrl;

pub struct ConnectionPool {
    connections: HashMap<ConnectionKey, Stream>,
}

#[derive(Debug, PartialEq, Eq, Hash)]
struct ConnectionKey {
    pub host: String,
    pub port: u16,
    pub tls: bool,
}

impl From<&HttpUrl> for ConnectionKey {
    fn from(http_url: &HttpUrl) -> Self {
        Self {
            host: http_url.host.clone(),
            port: http_url.port,
            tls: http_url.tls,
        }
    }
}

struct Stream(BufReader<Box<dyn ReadWrite>>);

impl ConnectionPool {
    pub fn new() -> Self {
        Self {
            connections: HashMap::new(),
        }
    }

    pub fn get_connection(
        &mut self,
        http_url: &HttpUrl,
    ) -> color_eyre::Result<&mut BufReader<Box<dyn ReadWrite>>> {
        let num_connections = self.connections.len();
        Ok(&mut self
            .connections
            .entry(http_url.into())
            .or_insert_with(|| {
                tracing::info!(
                    "Connecting to http{}://{}:{} (total connections: {})",
                    if http_url.tls { "s" } else { "" },
                    http_url.host,
                    http_url.port,
                    num_connections + 1
                );
                Stream(BufReader::new(match http_url.tls {
                    false => connect_http(http_url).unwrap(),
                    true => connect_https(http_url).unwrap(),
                }))
            })
            .0)
    }

    #[cfg(test)]
    pub fn set_connection(&mut self, http_url: &HttpUrl, conn: Box<dyn ReadWrite>) {
        self.connections
            .insert(http_url.into(), Stream(BufReader::new(conn)));
    }
}

impl Default for ConnectionPool {
    fn default() -> Self {
        Self::new()
    }
}

pub trait ReadWrite: Read + Write {}
impl<T: Read + Write> ReadWrite for T {}

fn connect_http(http_url: &HttpUrl) -> color_eyre::Result<Box<dyn ReadWrite>> {
    let sock = TcpStream::connect((http_url.host.as_str(), http_url.port))?;
    Ok(Box::new(sock))
}

fn connect_https(http_url: &HttpUrl) -> color_eyre::Result<Box<dyn ReadWrite>> {
    let config = ClientConfig::with_platform_verifier();
    let conn = rustls::ClientConnection::new(
        Arc::new(config),
        ServerName::try_from(http_url.host.to_string())?,
    )?;
    let sock = TcpStream::connect((http_url.host.as_str(), http_url.port))?;
    Ok(Box::new(rustls::StreamOwned::new(conn, sock)))
}

#[cfg(test)]
pub mod fake {
    use std::io::{Cursor, Read, Write};

    pub struct FakeStream {
        response: Cursor<Vec<u8>>,
    }

    impl FakeStream {
        pub fn new(response: &[u8]) -> Self {
            Self {
                response: Cursor::new(response.to_vec()),
            }
        }
    }

    impl Read for FakeStream {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            self.response.read(buf)
        }
    }

    impl Write for FakeStream {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
}
