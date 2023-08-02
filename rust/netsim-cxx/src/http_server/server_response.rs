// Copyright 2023 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Server Response Writer module for micro HTTP server.
//!
//! This module implements a basic response writer that can pass
//! chunked http responses between a uri handler and the network.
//!
//! The main use is for streaming large files from the capture_handler()
//! to the Http client.
//!
//! This library is intended solely for serving netsim clients.

use std::io::Write;

use log::error;

use crate::http_server::http_response::HttpResponse;

pub type ResponseWritable<'a> = &'a mut dyn ServerResponseWritable;

pub type StrHeaders = Vec<(String, String)>;

// ServerResponseWritable trait is used by both the Http and gRPC
// servers.
pub trait ServerResponseWritable {
    fn put_ok_with_length(&mut self, mime_type: &str, length: usize, headers: StrHeaders);
    fn put_chunk(&mut self, chunk: &[u8]);
    fn put_ok(&mut self, mime_type: &str, body: &str, headers: StrHeaders);
    fn put_error(&mut self, error_code: u16, error_message: &str);
    fn put_ok_with_vec(&mut self, mime_type: &str, body: Vec<u8>, headers: StrHeaders);
    fn put_ok_switch_protocol(&mut self, connection: &str, headers: StrHeaders);
}

// A response writer that can contain a TCP stream or other writable.
pub struct ServerResponseWriter<'a> {
    writer: &'a mut dyn Write,
    response: Option<HttpResponse>,
}

impl<'a> ServerResponseWriter<'a> {
    pub fn new<W: Write>(writer: &mut W) -> ServerResponseWriter {
        ServerResponseWriter { writer, response: None }
    }
    pub fn put_response(&mut self, response: HttpResponse) {
        let reason = match response.status_code {
            101 => "Switching Protocols",
            200 => "OK",
            404 => "Not Found",
            _ => "Unknown Reason",
        };
        let mut buffer = format!("HTTP/1.1 {} {}\r\n", response.status_code, reason).into_bytes();
        for (name, value) in response.headers.iter() {
            buffer
                .extend_from_slice(format!("{}: {}\r\n", name, value.to_str().unwrap()).as_bytes());
        }
        buffer.extend_from_slice(b"\r\n");
        buffer.extend_from_slice(&response.body);
        if let Err(e) = self.writer.write_all(&buffer) {
            error!("handle_connection error {e}");
        };
        self.response = Some(response);
    }
    pub fn get_response(self) -> Option<HttpResponse> {
        self.response
    }
}

// Implement the ServerResponseWritable trait for the
// ServerResponseWriter struct. These methods are called
// by the handler methods.
impl ServerResponseWritable for ServerResponseWriter<'_> {
    fn put_error(&mut self, error_code: u16, error_message: &str) {
        let response = HttpResponse::new_error(error_code, error_message.into());
        self.put_response(response);
    }
    fn put_chunk(&mut self, chunk: &[u8]) {
        if let Err(e) = self.writer.write_all(chunk) {
            error!("handle_connection error {e}");
        };
        self.writer.flush().unwrap();
    }
    fn put_ok_with_length(&mut self, mime_type: &str, length: usize, headers: StrHeaders) {
        let mut response = HttpResponse::new_ok_with_length(mime_type, length);
        response.add_headers(headers);
        self.put_response(response);
    }
    fn put_ok(&mut self, mime_type: &str, body: &str, headers: StrHeaders) {
        let mut response = HttpResponse::new_ok(mime_type, body.into());
        response.add_headers(headers);
        self.put_response(response);
    }
    fn put_ok_with_vec(&mut self, mime_type: &str, body: Vec<u8>, headers: StrHeaders) {
        let mut response = HttpResponse::new_ok(mime_type, body);
        response.add_headers(headers);
        self.put_response(response);
    }
    fn put_ok_switch_protocol(&mut self, connection: &str, headers: StrHeaders) {
        let mut response = HttpResponse::new_ok_switch_protocol(connection);
        response.add_headers(headers);
        self.put_response(response);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_put_error() {
        let mut stream = Cursor::new(Vec::new());
        let mut writer = ServerResponseWriter::new(&mut stream);
        writer.put_error(404, "Hello World");
        let written_bytes = stream.get_ref();
        let expected_bytes =
            b"HTTP/1.1 404 Not Found\r\ncontent-type: text/plain\r\ncontent-length: 11\r\n\r\nHello World";
        assert_eq!(written_bytes, expected_bytes);
    }

    #[test]
    fn test_put_ok() {
        let mut stream = Cursor::new(Vec::new());
        let mut writer = ServerResponseWriter::new(&mut stream);
        writer.put_ok("text/plain", "Hello World", vec![]);
        let written_bytes = stream.get_ref();
        let expected_bytes =
            b"HTTP/1.1 200 OK\r\ncontent-type: text/plain\r\ncontent-length: 11\r\n\r\nHello World";
        assert_eq!(written_bytes, expected_bytes);
    }
}
