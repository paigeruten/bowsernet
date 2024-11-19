use std::{
    io::{Read, Write},
    net::TcpStream,
};

use crate::Url;

pub fn request(url: &Url) -> color_eyre::Result<String> {
    let mut stream = TcpStream::connect((url.host.as_str(), 80))?;

    let mut request = format!("GET {} HTTP/1.0\r\n", url.path);
    request.push_str(&format!("Host: {}\r\n", url.host));
    request.push_str("\r\n");
    stream.write_all(request.as_bytes())?;

    let mut response = String::new();
    stream.read_to_string(&mut response)?;

    Ok(response)
}
