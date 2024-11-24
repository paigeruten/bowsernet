use color_eyre::eyre::OptionExt;
use std::{
    fs::File,
    io::{BufRead, Read, Write},
};

use crate::{
    http::headers::Headers,
    url::{DataUrl, FileUrl, HttpUrl, Scheme},
    Url,
};

mod connection_pool;
mod headers;

pub use connection_pool::ConnectionPool;

const HTTP_VERSION: &str = "1.1";
const USER_AGENT: &str = "bowsernet 0.00001";

pub fn request(url: &Url, connection_pool: &mut ConnectionPool) -> color_eyre::Result<String> {
    let content = match &url.scheme {
        Scheme::Http(http_url) => handle_normal_request(http_url, connection_pool)?,
        Scheme::File(file_url) => handle_file_request(file_url)?,
        Scheme::Data(data_url) => handle_data_request(data_url)?,
    };
    if url.view_source {
        Ok(content
            .replace("&", "&amp;")
            .replace("<", "&lt;")
            .replace(">", "&gt;"))
    } else {
        Ok(content)
    }
}

fn handle_normal_request(
    http_url: &HttpUrl,
    connection_pool: &mut ConnectionPool,
) -> color_eyre::Result<String> {
    let stream = connection_pool.get_connection(http_url)?;

    let request_headers = Headers::new()
        .add("Host", &http_url.host)
        .add("User-Agent", USER_AGENT);

    write!(
        stream.get_mut(),
        "GET {} HTTP/{}\r\n",
        http_url.path,
        HTTP_VERSION
    )?;
    write!(stream.get_mut(), "{}\r\n", request_headers.to_http_string())?;
    stream.get_mut().flush()?;

    let mut line = String::new();
    stream.read_line(&mut line)?;

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
        stream.read_line(&mut line)?;
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

    let content_length: usize = response_headers.get("content-length").unwrap().parse()?;

    let mut content = vec![0; content_length];
    stream.read_exact(&mut content)?;

    Ok(String::from_utf8(content)?)
}

fn handle_file_request(file_url: &FileUrl) -> color_eyre::Result<String> {
    let mut f = File::open(&file_url.path)?;
    let mut content = String::new();
    f.read_to_string(&mut content)?;
    Ok(content)
}

fn handle_data_request(data_url: &DataUrl) -> color_eyre::Result<String> {
    Ok(data_url.contents.clone())
}
