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

use httparse;

#[cfg(test)]
mod tests {
    use super::*;
    use httparse::Header;

    #[test]
    fn rewrite_test() {
        let mut headers = [httparse::EMPTY_HEADER; 64];
        let _ = rewrite(&mut headers);
    }

    fn rewrite(headers: &mut [Header]) -> Result<(), Box<dyn std::error::Error>> {
        let mut req = httparse::Request::new(headers);

        let buf = b"GET /index.html HTTP/1.1\r\nHost";
        assert!(req.parse(buf)?.is_partial());

        // a partial request, so we try again once we have more data
        let buf = b"GET /index.html HTTP/1.1\r\nHost: example.domain\r\n\r\n";
        assert!(req.parse(buf)?.is_complete());

        println!("headers: {:?}", req.headers);
        println!("request: {:?}", req);
        Ok(())
    }
}
