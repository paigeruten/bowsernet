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
        let key = ConnectionKey {
            host: http_url.host.clone(),
            port: http_url.port,
            tls: http_url.tls,
        };
        println!("Checking for existing connection for {:?}...", &key);
        Ok(&mut self
            .connections
            .entry(key)
            .or_insert_with(|| {
                println!("No existing connection found, creating a new one.");
                Stream(BufReader::new(match http_url.tls {
                    false => connect_http(http_url).unwrap(),
                    true => connect_https(http_url).unwrap(),
                }))
            })
            .0)
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
