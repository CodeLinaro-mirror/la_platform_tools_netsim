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

//! Request router for micro HTTP server.
//!
//! This module implements a basic request router with regex matching
//! of URI fields. For example
//!
//!   router.add_route("/user/([0-9]+)", handle_device);
//!
//! will register a handler that matches numeric user ids.
//!
//! This library is only used for serving the netsim client and is not
//! meant to implement all aspects of an http router.

use regex::Captures;
use regex::Regex;

use crate::frontend_http_server::http_request::HttpRequest;
use crate::frontend_http_server::http_response::HttpResponse;

type RequestHandler = fn(&HttpRequest, Captures) -> HttpResponse;

pub struct Router {
    routes: Vec<(Regex, RequestHandler)>,
}

impl Router {
    pub fn new() -> Router {
        Router { routes: Vec::new() }
    }

    pub fn add_route(&mut self, uri: &str, handler: RequestHandler) {
        // force whole string match using ^ and $
        let regex = Regex::new(format!("^{uri}$").as_str()).unwrap();
        self.routes.push((regex, handler));
    }

    pub fn handle_request(&self, request: &HttpRequest) -> HttpResponse {
        for (regex, handler) in &self.routes {
            if regex.is_match(&request.uri) {
                let captures = regex.captures(&request.uri).unwrap();
                return handler(request, captures);
            }
        }
        println!("netsim: HttpRouter unknown uri {}", request.uri);
        HttpResponse::new_404()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend_http_server::http_response::HttpHeaders;

    fn handle_index(_request: &HttpRequest, _captures: Captures) -> HttpResponse {
        HttpResponse::new_200("text/html", b"Hello, world!".to_vec())
    }

    fn handle_user(_request: &HttpRequest, captures: Captures) -> HttpResponse {
        let user_id = match captures.get(1) {
            Some(user_id) => user_id.as_str(),
            None => return HttpResponse::new_404(),
        };
        HttpResponse::new_200("application/json", format!("Hello, {user_id}!").as_bytes().to_vec())
    }

    #[test]
    fn test_handle_request() {
        let mut router = Router::new();
        router.add_route("/", handle_index);
        router.add_route("/user/([0-9]+)", handle_user);
        let request = HttpRequest {
            method: "GET".to_string(),
            uri: "/".to_string(),
            version: "HTTP/1.1".to_string(),
            headers: HttpHeaders::new(),
            body: vec![],
        };
        let response = router.handle_request(&request);
        assert_eq!(response.status_code, 200);
        let content_type = response.headers.get("Content-Type");
        println!("headers are {:?}", response.headers);
        assert!(content_type.is_some());
        assert_eq!(content_type.unwrap(), "text/html");
        assert_eq!(response.body, b"Hello, world!".to_vec());

        let request = HttpRequest {
            method: "GET".to_string(),
            uri: "/user/1920".to_string(),
            version: "HTTP/1.1".to_string(),
            headers: HttpHeaders::new(),
            body: vec![],
        };
        let response = router.handle_request(&request);
        assert_eq!(response.status_code, 200);
        assert_eq!(response.body, b"Hello, 1920!".to_vec());
        let content_type = response.headers.get("Content-Type");
        assert!(content_type.is_some());
        assert_eq!(content_type.unwrap(), "application/json");
    }
}
