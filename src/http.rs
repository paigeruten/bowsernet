use std::{
    collections::HashMap,
    io::{BufRead, BufReader, BufWriter, Read, Write},
    net::TcpStream,
};

use color_eyre::eyre::OptionExt;

use crate::Url;

pub fn request(url: &Url) -> color_eyre::Result<String> {
    let stream = TcpStream::connect((url.host.as_str(), 80))?;
    let mut reader = BufReader::new(&stream);
    let mut writer = BufWriter::new(&stream);

    write!(writer, "GET {} HTTP/1.0\r\n", url.path)?;
    write!(writer, "Host: {}\r\n", url.host)?;
    write!(writer, "\r\n")?;
    writer.flush()?;

    let mut line = String::new();
    reader.read_line(&mut line)?;

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
        reader.read_line(&mut line)?;
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
    reader.read_to_string(&mut content)?;

    Ok(content)
}
