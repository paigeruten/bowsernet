use color_eyre::eyre::OptionExt;
use flate2::bufread::GzDecoder;
use std::{
    fs::File,
    io::{BufRead, Read, Write},
};

use crate::{
    cache::RequestCache,
    http::headers::{CacheControl, Headers},
    url::{BuiltinUrl, DataUrl, FileUrl, HttpUrl, Scheme},
    Url,
};

mod connection_pool;
mod headers;

pub use connection_pool::ConnectionPool;

const HTTP_VERSION: &str = "1.1";
const USER_AGENT: &str = "bowsernet 0.00001";
const REDIRECT_LIMIT: usize = 5;

pub fn request(
    url: &Url,
    connection_pool: &mut ConnectionPool,
    cache: &mut RequestCache,
) -> color_eyre::Result<String> {
    tracing::info!("Requesting {}", url);
    let content = match &url.scheme {
        Scheme::Http(http_url) => handle_normal_request(http_url, connection_pool, cache, 0)?,
        Scheme::File(file_url) => handle_file_request(file_url)?,
        Scheme::Data(data_url) => handle_data_request(data_url)?,
        Scheme::Builtin(builtin_url) => handle_builtin_request(builtin_url)?,
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

#[tracing::instrument(skip(http_url, connection_pool, cache), fields(http_url = %http_url))]
fn handle_normal_request(
    http_url: &HttpUrl,
    connection_pool: &mut ConnectionPool,
    cache: &mut RequestCache,
    num_redirects: usize,
) -> color_eyre::Result<String> {
    if num_redirects >= REDIRECT_LIMIT {
        return Err(color_eyre::eyre::eyre!("Too many redirects"));
    }

    if let Some(content) = cache.get(http_url) {
        tracing::info!("Loading response from cache");
        return Ok(content.to_string());
    }

    let stream = connection_pool.get_connection(http_url)?;

    let request_headers = Headers::new()
        .add("Host", &http_url.host)
        .add("Accept-Encoding", "gzip")
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

    let content = if let Some(transfer_encoding) = response_headers.get("transfer-encoding") {
        if transfer_encoding != "chunked" {
            return Err(color_eyre::eyre::eyre!(
                "Unhandled transfer-encoding: {transfer_encoding}"
            ));
        }

        tracing::info!("Reading chunked response");

        let mut expected_newline = vec![0; 2];
        let mut content = Vec::new();
        let mut line = String::new();
        let mut chunk = Vec::new();
        loop {
            line.clear();
            stream.read_line(&mut line)?;
            let chunk_length = usize::from_str_radix(line.trim_ascii_end(), 16)?;

            tracing::debug!("Reading chunk of length {chunk_length}");

            chunk.clear();
            chunk.resize(chunk_length, 0);
            stream.read_exact(&mut chunk)?;
            content.append(&mut chunk);

            expected_newline.fill(0);
            stream.read_exact(&mut expected_newline)?;
            assert_eq!(expected_newline, b"\r\n");

            if chunk_length == 0 {
                break;
            }
        }

        content
    } else {
        let content_length: usize = response_headers.get("content-length").unwrap().parse()?;
        let mut content = vec![0; content_length];
        stream.read_exact(&mut content)?;
        content
    };

    let content = if response_headers.contains("content-encoding") {
        tracing::info!("Decompressing gzipped response");
        let mut gz = GzDecoder::new(&content[..]);
        let mut decompressed = String::new();
        gz.read_to_string(&mut decompressed)?;
        decompressed
    } else {
        String::from_utf8(content)?
    };

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
        return handle_normal_request(&redirect_url, connection_pool, cache, num_redirects + 1);
    }

    if status == 200 {
        let cache_control: CacheControl = response_headers
            .get("cache-control")
            .map(|value| value.into())
            .unwrap_or_default();

        if cache_control.no_store {
            tracing::info!("Not caching request due to no-store directive");
        } else {
            tracing::info!(
                "Caching request with max_age of {:?}",
                cache_control.max_age
            );
            cache.set(http_url, &content, cache_control.max_age);
        }
    }

    Ok(content)
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

fn handle_builtin_request(builtin_url: &BuiltinUrl) -> color_eyre::Result<String> {
    match builtin_url {
        BuiltinUrl::AboutBlank => Ok("".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::connection_pool::fake::FakeStream;

    fn mocked_request(url: &Url, raw_response: &[u8]) -> color_eyre::Result<String> {
        let http_url = match &url.scheme {
            Scheme::Http(http_url) => http_url,
            _ => return Err(color_eyre::eyre::eyre!("Mock URL's scheme must be HTTP(S)")),
        };

        let mut connection_pool = ConnectionPool::new();
        connection_pool.set_connection(http_url, Box::new(FakeStream::new(raw_response)));

        request(url, &mut connection_pool, &mut RequestCache::new())
    }

    #[test]
    fn basic_request() -> color_eyre::Result<()> {
        let url = Url::parse("http://example.org")?;
        let raw_response = b"\
            HTTP/1.1 200 OK\r\n\
            Content-Length: 13\r\n\
            \r\n\
            Hello, world!";

        let response = mocked_request(&url, raw_response)?;

        assert_eq!(response, "Hello, world!");

        Ok(())
    }

    #[test]
    fn request_with_view_source() -> color_eyre::Result<()> {
        let url = Url::parse("view-source:http://example.org")?;
        let raw_response = b"\
            HTTP/1.1 200 OK\r\n\
            Content-Length: 20\r\n\
            \r\n\
            <p>Hello, world!</p>";

        let response = mocked_request(&url, raw_response)?;

        assert_eq!(response, "&lt;p&gt;Hello, world!&lt;/p&gt;");

        Ok(())
    }

    #[test]
    fn request_with_redirect() -> color_eyre::Result<()> {
        let url = Url::parse("http://example.org")?;
        let raw_response = b"\
            HTTP/1.1 301 Moved Permanently\r\n\
            Location: /index.html\r\n\
            Content-Length: 0\r\n\
            \r\n\
            HTTP/1.1 200 OK\r\n\
            Content-Length: 13\r\n\
            \r\n\
            Hello, world!";

        let response = mocked_request(&url, raw_response)?;

        assert_eq!(response, "Hello, world!");

        Ok(())
    }

    #[test]
    fn request_chunked_encoding() -> color_eyre::Result<()> {
        let url = Url::parse("http://example.org")?;
        let raw_response = b"\
            HTTP/1.1 200 OK\r\n\
            Transfer-Encoding: chunked\r\n\
            \r\n\
            7\r\n\
            Hello, \r\n\
            6\r\n\
            world!\r\n\
            0\r\n\
            \r\n";

        let response = mocked_request(&url, raw_response)?;

        assert_eq!(response, "Hello, world!");

        Ok(())
    }

    #[test]
    fn request_gzipped() -> color_eyre::Result<()> {
        let url = Url::parse("http://example.org")?;
        let raw_response = b"\
            HTTP/1.1 200 OK\r\n\
            Content-Encoding: gzip\r\n\
            Transfer-Encoding: chunked\r\n\
            \r\n\
            21\r\n\
            \x1F\x8B\x08\x00\x00\x00\x00\x00\x00\x03\xF3\x48\xCD\xC9\xC9\xD7\x51\x28\xCF\x2F\xCA\x49\x51\x04\x00\xE6\xC6\xE6\xEB\x0D\x00\x00\x00\r\n\
            0\r\n\
            \r\n";

        let response = mocked_request(&url, raw_response)?;

        assert_eq!(response, "Hello, world!");

        Ok(())
    }
}
