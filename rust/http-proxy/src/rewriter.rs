// Copyright 2025 Google LLC
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

//! # HTTP Request Rewriter
//!
//! This module provides functionality to parse and rewrite an HTTP request
//! from its `origin-form` (style used for direct communication with a server)
//! to its `absolute-form` (style required for proxying).
//!
//! ## Key Components
//!
//! - **`rewrite_request_to_absolute_form`**: The primary function that reads from a
//!   `BufRead` stream and performs the transformation.
//! - **`RewriteError`**: An enum that defines possible errors, such as I/O
//!   issues, malformed requests, or a missing `Host` header.
//!
//! This is typically used in the core logic of an HTTP proxy server.

use std::fmt;
use std::io::{self, BufRead};

// --- Custom Error Type ---

/// Represents all possible errors that can occur during request rewriting.
/// The `PartialEq` trait is derived for easier comparison in tests.
#[derive(Debug)]
pub enum RewriteError {
    /// An error occurred during I/O operations (e.g., reading from the stream).
    /// Note: `io::Error` does not implement `PartialEq`, so we handle it specially in tests.
    Io(io::Error),
    /// The HTTP request-line is malformed and cannot be parsed.
    MalformedRequestLine(String),
    /// The 'Host' header is missing, which is required for HTTP/1.1.
    MissingHostHeader,
}

impl PartialEq for RewriteError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            // For Io errors, we compare their 'kind'. This is a reliable way
            // to check if they represent the same type of error (e.g., NotFound).
            (Self::Io(a), Self::Io(b)) => a.kind() == b.kind(),

            // For other variants, we compare their inner values directly.
            (Self::MalformedRequestLine(a), Self::MalformedRequestLine(b)) => a == b,
            (Self::MissingHostHeader, Self::MissingHostHeader) => true,

            // If the variants are different, they are not equal.
            _ => false,
        }
    }
}

// Implement the Display trait for user-friendly error messages.
impl fmt::Display for RewriteError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RewriteError::Io(err) => write!(f, "I/O error: {}", err),
            RewriteError::MalformedRequestLine(line) => {
                write!(f, "Malformed request line: '{}'", line)
            }
            RewriteError::MissingHostHeader => write!(f, "Mandatory 'Host' header is missing"),
        }
    }
}

// Allow `io::Error` to be converted into our custom `RewriteError::Io`.
impl From<io::Error> for RewriteError {
    fn from(err: io::Error) -> Self {
        RewriteError::Io(err)
    }
}

// --- Core Rewriting Function ---

/// Reads an HTTP request, rewriting the target from origin-form to absolute-form.
///
/// ## HTTP Request Target Forms Explained
///
/// An HTTP request-line is `METHOD TARGET HTTP_VERSION`. The `TARGET` has several forms:
///
/// 1.  **Origin-Form (Direct Style to Server)**
///     - This is the most common form, sent to an origin server.
///     - The target only contains the resource path and query string.
///     - Example: `GET /path/to/resource.html HTTP/1.1`
///
/// 2.  **Absolute-Form (Proxy Style)**
///     - This form is required when sending a request to an HTTP proxy.
///     - The target must be the full URI so the proxy knows which server to contact.
///     - Example: `GET http://www.example.com/path/to/resource.html HTTP/1.1`
///
/// This function performs the conversion from **origin-form** to **absolute-form**.
///
/// # Arguments
///
/// * `reader` - A mutable reference to a buffered reader containing the raw HTTP request.
///
/// # Returns
///
/// A `Result` containing either:
/// - `Ok(String)`: The rewritten request (new request-line + original headers).
/// - `Err(RewriteError)`: An error that occurred during processing.
///
pub fn rewrite_request_to_absolute_form<R: BufRead>(
    reader: &mut R,
) -> Result<String, RewriteError> {
    // A buffer to hold the raw header lines as we read them.
    let mut header_lines = Vec::new();

    // The first line from the reader is the special request-line.
    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;

    // Read all subsequent header lines until we find an empty line (`\r\n`),
    // which signifies the end of the headers section.
    loop {
        let mut line = String::new();
        let bytes_read = reader.read_line(&mut line)?;

        // An empty line or a 0-byte read indicates the end of headers or the stream.
        if line.trim().is_empty() || bytes_read == 0 {
            break;
        }
        header_lines.push(line);
    }

    // --- Parse, Validate, and Rewrite ---

    // 1. Find the 'Host' header. It's case-insensitive and required by HTTP/1.1.
    // The `trim()` handles potential whitespace around the host value.
    let host = header_lines
        .iter()
        .find(|h| h.to_lowercase().starts_with("host:"))
        .map(|h| h.split_once(':').map_or("", |(_key, value)| value.trim()))
        .ok_or(RewriteError::MissingHostHeader)?;

    // Ensure the Host header was not present but empty (e.g., "Host: ").
    if host.is_empty() {
        return Err(RewriteError::MissingHostHeader);
    }

    // 2. Parse the original request-line (e.g., "GET /path HTTP/1.1").
    let request_parts: Vec<&str> = request_line.split_whitespace().collect();
    if request_parts.len() != 3 {
        // Return the invalid line in the error for easier debugging.
        return Err(RewriteError::MalformedRequestLine(request_line.trim_end().to_string()));
    }
    let method = request_parts[0];
    let path = request_parts[1]; // This is the origin-form target (e.g., "/path")
    let version = request_parts[2];

    // 3. Construct the new "absolute-form" URI required for a proxy request.
    //    e.g., "http://" + "www.example.com" + "/path/to/resource.html"
    let absolute_uri = format!("http://{}{}", host, path);

    // 4. Build the final rewritten request string.
    let mut rewritten_request = String::new();

    // Add the new proxy-style request-line.
    rewritten_request.push_str(&format!("{} {} {}\r\n", method, absolute_uri, version));

    // Append all the original headers unmodified.
    for header in header_lines {
        rewritten_request.push_str(&header);
    }

    // Add the final empty line to terminate the header section.
    rewritten_request.push_str("\r\n");

    Ok(rewritten_request)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::BufReader;

    /// Tests a standard, well-formed GET request.
    #[test]
    fn test_successful_get_request_rewrite() {
        let request =
            b"GET /path/to/page.html HTTP/1.1\r\nHost: example.com\r\nUser-Agent: test\r\n\r\n";
        let mut reader = BufReader::new(&request[..]);
        let result = rewrite_request_to_absolute_form(&mut reader).unwrap();

        let expected = "GET http://example.com/path/to/page.html HTTP/1.1\r\nHost: example.com\r\nUser-Agent: test\r\n\r\n";

        assert_eq!(result, expected);
    }

    /// Tests that methods other than GET, like POST, are handled correctly.
    #[test]
    fn test_successful_post_request_rewrite() {
        let request =
            b"POST /api/v1/users HTTP/1.1\r\nHost: api.service.io\r\nContent-Length: 42\r\n\r\n";
        let mut reader = BufReader::new(&request[..]);
        let result = rewrite_request_to_absolute_form(&mut reader).unwrap();

        let expected = "POST http://api.service.io/api/v1/users HTTP/1.1\r\nHost: api.service.io\r\nContent-Length: 42\r\n\r\n";

        assert_eq!(result, expected);
    }

    /// Verifies that the 'Host' header key is treated as case-insensitive.
    #[test]
    fn test_host_header_is_case_insensitive() {
        let request = b"GET / HTTP/1.1\r\nhOsT: case-matters-not.com\r\n\r\n";
        let mut reader = BufReader::new(&request[..]);
        let result = rewrite_request_to_absolute_form(&mut reader).unwrap();

        let expected =
            "GET http://case-matters-not.com/ HTTP/1.1\r\nhOsT: case-matters-not.com\r\n\r\n";

        assert_eq!(result, expected);
    }

    /// Ensures that extra whitespace around the `Host` header value is trimmed.
    #[test]
    fn test_host_header_trims_whitespace() {
        let request = b"GET / HTTP/1.1\r\nHost:  spaced-out.net  \r\n\r\n";
        let mut reader = BufReader::new(&request[..]);
        let result = rewrite_request_to_absolute_form(&mut reader).unwrap();

        let expected = "GET http://spaced-out.net/ HTTP/1.1\r\nHost:  spaced-out.net  \r\n\r\n";

        assert_eq!(result, expected);
    }

    /// Tests that an error is returned if the 'Host' header is completely missing.
    #[test]
    fn test_error_on_missing_host_header() {
        let request = b"GET /path HTTP/1.1\r\nUser-Agent: No-Host-Client\r\n\r\n";
        let mut reader = BufReader::new(&request[..]);
        let result = rewrite_request_to_absolute_form(&mut reader);

        assert_eq!(result, Err(RewriteError::MissingHostHeader));
    }

    /// Tests that an error is returned if the 'Host' header is present but has an empty value.
    #[test]
    fn test_error_on_empty_host_header_value() {
        let request = b"GET /path HTTP/1.1\r\nHost: \r\n\r\n";
        let mut reader = BufReader::new(&request[..]);
        let result = rewrite_request_to_absolute_form(&mut reader);

        assert_eq!(result, Err(RewriteError::MissingHostHeader));
    }

    /// Tests that an error is returned for a malformed request-line.
    #[test]
    fn test_error_on_malformed_request_line() {
        // This request line only has two parts, which is invalid.
        let request = b"GET /path\r\nHost: example.com\r\n\r\n";
        let mut reader = BufReader::new(&request[..]);
        let result = rewrite_request_to_absolute_form(&mut reader);

        // We can check that the error contains the invalid line.
        let expected_error = RewriteError::MalformedRequestLine("GET /path".to_string());
        assert_eq!(result, Err(expected_error));
    }
}
