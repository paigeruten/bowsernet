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
const REDIRECT_LIMIT: usize = 5;

pub fn request(url: &Url, connection_pool: &mut ConnectionPool) -> color_eyre::Result<String> {
    tracing::info!("Requesting {}", url);
    let content = match &url.scheme {
        Scheme::Http(http_url) => handle_normal_request(http_url, connection_pool, 0)?,
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

#[tracing::instrument(skip(http_url, connection_pool), fields(http_url = %http_url))]
fn handle_normal_request(
    http_url: &HttpUrl,
    connection_pool: &mut ConnectionPool,
    num_redirects: usize,
) -> color_eyre::Result<String> {
    if num_redirects >= REDIRECT_LIMIT {
        return Err(color_eyre::eyre::eyre!("Too many redirects"));
    }

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
    let _version = statusline
        .next()
        .ok_or_eyre("Version expected in HTTP response")?;
    let status: u16 = statusline
        .next()
        .ok_or_eyre("Status expected in HTTP response")?
        .parse()
        .unwrap();
    let explanation = statusline
        .next()
        .ok_or_eyre("Explanation expected in HTTP response")?;
    tracing::info!("Server returned {} {}", status, explanation);

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
    tracing::debug!("Response headers: {:?}", &response_headers);

    assert!(!response_headers.contains("transfer-encoding"));
    assert!(!response_headers.contains("content-encoding"));

    let content_length: usize = response_headers.get("content-length").unwrap().parse()?;

    let mut content = vec![0; content_length];
    stream.read_exact(&mut content)?;

    if (300..=399).contains(&status) {
        let location = response_headers.get("location").unwrap();
        tracing::info!("Redirecting to {}", location);
        let redirect_url = if location.starts_with('/') {
            HttpUrl {
                path: location.to_string(),
                ..http_url.clone()
            }
        } else if let Scheme::Http(redirect_url) = Url::parse(location)?.scheme {
            redirect_url
        } else {
            return Err(color_eyre::eyre::eyre!("Invalid redirect URL"));
        };
        return handle_normal_request(&redirect_url, connection_pool, num_redirects + 1);
    }

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
