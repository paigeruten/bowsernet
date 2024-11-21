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

pub fn request(url: &Url) -> color_eyre::Result<String> {
    let stream = if url.scheme == "https" {
        connect_https(url)?
    } else {
        connect_http(url)?
    };
    let mut r = BufReader::new(stream);

    write!(r.get_mut(), "GET {} HTTP/1.0\r\n", url.path)?;
    write!(r.get_mut(), "Host: {}\r\n", url.host)?;
    write!(r.get_mut(), "\r\n")?;
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

    let mut response_headers = HashMap::new();
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
        response_headers.insert(header.trim().to_ascii_lowercase(), value.trim().to_string());
    }
    dbg!(&response_headers);

    assert!(!response_headers.contains_key("transfer-encoding"));
    assert!(!response_headers.contains_key("content-encoding"));

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
