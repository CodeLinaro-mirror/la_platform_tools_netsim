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

//! Response module for micro HTTP server.
//!
//! This library implements the basic parts of Response Message from
//! (RFC 5322)[ https://www.rfc-editor.org/rfc/rfc5322.html] "HTTP
//! Message Format."
//!
//! This library is only used for serving the netsim client and is not
//! meant to implement all aspects of RFC 5322.

use std::str::FromStr;

use http::{HeaderMap, HeaderName, HeaderValue};

use super::server_response::StrHeaders;

pub struct HttpResponse {
    pub status_code: u16,
    pub headers: HeaderMap,
    pub body: Vec<u8>,
}

impl HttpResponse {
    pub fn new_ok_with_length(content_type: &str, length: usize) -> HttpResponse {
        let body = Vec::new();
        let mut headers = HeaderMap::new();
        headers.insert("Content-Type", HeaderValue::from_str(content_type).unwrap());
        headers
            .insert("Content-Length", HeaderValue::from_str(length.to_string().as_str()).unwrap());
        HttpResponse { status_code: 200, headers, body }
    }

    pub fn new_ok(content_type: &str, body: Vec<u8>) -> HttpResponse {
        let mut headers = HeaderMap::new();
        headers.insert("Content-Type", HeaderValue::from_str(content_type).unwrap());
        headers.insert(
            "Content-Length",
            HeaderValue::from_str(body.len().to_string().as_str()).unwrap(),
        );
        HttpResponse { status_code: 200, headers, body }
    }

    pub fn new_ok_switch_protocol(connection: &str) -> HttpResponse {
        let mut headers = HeaderMap::new();
        headers.insert("Upgrade", HeaderValue::from_str(connection).unwrap());
        headers.insert("Connection", HeaderValue::from_static("Upgrade"));
        HttpResponse { status_code: 101, headers, body: Vec::new() }
    }

    pub fn new_error(status_code: u16, body: Vec<u8>) -> HttpResponse {
        let mut headers = HeaderMap::new();
        headers.insert("Content-Type", HeaderValue::from_static("text/plain"));
        headers.insert(
            "Content-Length",
            HeaderValue::from_str(body.len().to_string().as_str()).unwrap(),
        );
        HttpResponse { status_code, headers, body }
    }

    pub fn add_headers(&mut self, headers: StrHeaders) {
        for (key, value) in headers {
            self.headers.insert(
                HeaderName::from_str(key.as_str()).unwrap(),
                HeaderValue::from_str(value.as_str()).unwrap(),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::http_server::server_response::{ServerResponseWritable, ServerResponseWriter};
    use std::io::Cursor;

    #[test]
    fn test_write_to() {
        let mut stream = Cursor::new(Vec::new());
        let mut writer = ServerResponseWriter::new(&mut stream);
        writer.put_ok_with_vec("text/plain", b"Hello World".to_vec(), vec![]);
        let written_bytes = stream.get_ref();
        let expected_bytes =
            b"HTTP/1.1 200 OK\r\ncontent-type: text/plain\r\ncontent-length: 11\r\n\r\nHello World";
        assert_eq!(written_bytes, expected_bytes);
    }
}
