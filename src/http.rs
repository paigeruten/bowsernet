use color_eyre::eyre::OptionExt;
use rustls::{pki_types::ServerName, ClientConfig};
use rustls_platform_verifier::ConfigVerifierExt;
use std::{
    collections::HashMap,
    io::{BufRead, BufReader, Read, Write},
    net::TcpStream,
    sync::Arc,
};

use crate::Url;

const HTTP_VERSION: &str = "1.1";
const USER_AGENT: &str = "bowsernet 0.00001";

pub fn request(url: &Url) -> color_eyre::Result<String> {
    let stream = if url.scheme == "https" {
        connect_https(url)?
    } else {
        connect_http(url)?
    };
    let mut r = BufReader::new(stream);

    let request_headers = Headers::new()
        .add("Host", &url.host)
        .add("Connection", "close")
        .add("User-Agent", USER_AGENT);

    write!(r.get_mut(), "GET {} HTTP/{}\r\n", url.path, HTTP_VERSION)?;
    write!(r.get_mut(), "{}\r\n", request_headers.to_http_string())?;
    r.get_mut().flush()?;

    let mut line = String::new();
    r.read_line(&mut line)?;

    let mut statusline = line.trim_ascii().splitn(3, ' ');
    let version = statusline
        .next()
        .ok_or_eyre("Version expected in HTTP response")?;
    let status = statusline
        .next()
        .ok_or_eyre("Status expected in HTTP response")?;
    let explanation = statusline
        .next()
        .ok_or_eyre("Explanation expected in HTTP response")?;
    dbg!(version, status, explanation);

    let mut response_headers = Headers::new();
    loop {
        line.clear();
        r.read_line(&mut line)?;
        if line == "\r\n" {
            break;
        }
        let (header, value) = line
            .trim()
            .split_once(':')
            .ok_or_eyre("Expected a colon in HTTP header line")?;
        response_headers.set(header.trim(), value.trim());
    }
    dbg!(&response_headers);

    assert!(!response_headers.contains("transfer-encoding"));
    assert!(!response_headers.contains("content-encoding"));

    let mut content = String::new();
    r.read_to_string(&mut content)?;

    Ok(content)
}

trait ReadWrite: Read + Write {}
impl<T: Read + Write> ReadWrite for T {}

fn connect_http(url: &Url) -> color_eyre::Result<Box<dyn ReadWrite>> {
    let sock = TcpStream::connect((url.host.as_str(), url.port))?;
    Ok(Box::new(sock))
}

fn connect_https(url: &Url) -> color_eyre::Result<Box<dyn ReadWrite>> {
    let config = ClientConfig::with_platform_verifier();
    let conn = rustls::ClientConnection::new(
        Arc::new(config),
        ServerName::try_from(url.host.to_string())?,
    )?;
    let sock = TcpStream::connect((url.host.as_str(), url.port))?;
    Ok(Box::new(rustls::StreamOwned::new(conn, sock)))
}

#[derive(Debug)]
struct Headers {
    values: HashMap<String, HeaderValue>,
}

#[derive(Debug)]
struct HeaderValue {
    original_name: String,
    value: String,
}

impl Headers {
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
        }
    }

    pub fn add(mut self, name: &str, value: &str) -> Self {
        self.set(name, value);
        self
    }

    pub fn set(&mut self, name: &str, value: &str) {
        self.values.insert(
            name.to_ascii_lowercase(),
            HeaderValue {
                original_name: name.to_string(),
                value: value.to_string(),
            },
        );
    }

    pub fn contains(&self, name: &str) -> bool {
        self.values.contains_key(&name.to_ascii_lowercase())
    }

    pub fn to_http_string(&self) -> String {
        let mut s = String::new();
        for header in self.values.values() {
            s.push_str(&format!("{}: {}\r\n", header.original_name, header.value));
        }
        s
    }
}
