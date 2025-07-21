/*!
    HyRAID daemon

    Copyright (C) 2025 LIZARD-OFFICIAL-77
    This program is free software; you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation; either version 2 of the License, or
    (at your option) any later version.
    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License along
    with this program; if not, write to the Free Software Foundation, Inc.,
    51 Franklin Street, Fifth Floor, Boston, MA 02110-1301 USA.
*/

use std::os::unix::net::{UnixStream, UnixListener};
use std::io::{Read, Write};

const SOCK_PATH: &str = "/tmp/hyraid.sock";
const REQUEST_DELIMITER: &str = ";";
const BUFFER_SIZE: usize = 1024;

fn decode_stream(mut stream: UnixStream) -> (Vec<String>,UnixStream) {
    let mut buffer = [0u8; BUFFER_SIZE];
    match stream.read(&mut buffer) {
        Ok(size) => {
            let decoded = String::from_utf8_lossy(&buffer[..size]);
            let decoded: Vec<String> = decoded
                .split(REQUEST_DELIMITER)
                .collect::<Vec<_>>()
                .iter()
                .map(|s| s.to_string())
                .collect();
            return (decoded,stream);
        }
        Err(err) => {
            eprintln!("Error occurred while decoding stream");
            eprintln!("{}",err);
            return (vec!["error".to_string()],stream);
        }
    }
}

/// Encodes Vec<String> into bytes
fn encode_bytes(value: Vec<String>) -> [u8; BUFFER_SIZE] {
    let mut buffer = [0u8; BUFFER_SIZE];
    let value = value.join(REQUEST_DELIMITER);
    if value.len() > BUFFER_SIZE {
        panic!("Request is longer than {} characters.",BUFFER_SIZE)
    }
    let value_bytes = value.as_bytes();
    let len = value_bytes.len().min(BUFFER_SIZE);
    buffer[..len].copy_from_slice(&value_bytes[..len]);
    return buffer;
}

fn handle_request(request: Vec<String>) -> Vec<String> {
    todo!()
}

pub fn listen() -> std::io::Result<()> {
    let listener = UnixListener::bind(SOCK_PATH)?;
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let mut decoded = decode_stream(stream);
                if decoded.0 != vec!["error".to_string()] {
                    let response = handle_request(decoded.0);
                    let encoded = &encode_bytes(response);
                    decoded.1.write_all(encoded)?;
                }
            },
            Err(err) => {
                eprintln!("Error occurred while accepting stream:");
                eprintln!("{}",err);
            }
        }
    }
    return Ok(());
}

pub fn send(request: Vec<String>) -> std::io::Result<Vec<String>> {
    let mut stream = UnixStream::connect(SOCK_PATH)?;
    let request = &encode_bytes(request);
    stream.write_all(request)?;

    return Ok(decode_stream(stream).0);
}