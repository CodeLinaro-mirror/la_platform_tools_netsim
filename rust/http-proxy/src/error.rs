// Copyright 2024 Google LLC
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

//! This module defines the Proxy error types.

use std::fmt;
use std::io;
use std::net::SocketAddr;

/// An enumeration of possible errors.
#[derive(Debug)]
pub enum Error {
    /// An I/O error occurred.
    IoError(io::Error),
    /// An error occurred during connection establishment.
    ConnectionError(SocketAddr, String),
    /// The configuration string was malformed.
    MalformedConfigString,
    /// The provided port number was invalid.
    InvalidPortNumber,
    /// The provided host was invalid.
    InvalidHost,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ProxyError: {self:?}")
    }
}

impl std::error::Error for Error {}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::IoError(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_io_error_chaining() {
        let inner_error = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let outer_error = Error::IoError(inner_error);

        assert!(outer_error.to_string().contains("file not found"));
    }
}
