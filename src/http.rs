use color_eyre::eyre::OptionExt;
use rustls::{pki_types::ServerName, ClientConfig};
use rustls_platform_verifier::ConfigVerifierExt;
use std::{
    fs::File,
    io::{BufRead, BufReader, Read, Write},
    net::TcpStream,
    sync::Arc,
};

use crate::{
    http::headers::Headers,
    url::{FileUrl, HttpUrl, Scheme},
    Url,
};

mod headers;

const HTTP_VERSION: &str = "1.1";
const USER_AGENT: &str = "bowsernet 0.00001";

pub fn request(url: &Url) -> color_eyre::Result<String> {
    let content = match &url.scheme {
        Scheme::Http(http_url) => handle_normal_request(http_url)?,
        Scheme::File(file_url) => handle_file_request(file_url)?,
    };
    Ok(content)
}

fn handle_normal_request(http_url: &HttpUrl) -> color_eyre::Result<String> {
    let stream = if http_url.tls {
        connect_https(http_url)?
    } else {
        connect_http(http_url)?
    };
    let mut r = BufReader::new(stream);

    let request_headers = Headers::new()
        .add("Host", &http_url.host)
        .add("Connection", "close")
        .add("User-Agent", USER_AGENT);

    write!(
        r.get_mut(),
        "GET {} HTTP/{}\r\n",
        http_url.path,
        HTTP_VERSION
    )?;
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

fn handle_file_request(file_url: &FileUrl) -> color_eyre::Result<String> {
    let mut f = File::open(&file_url.path)?;
    let mut content = String::new();
    f.read_to_string(&mut content)?;
    Ok(content)
}
