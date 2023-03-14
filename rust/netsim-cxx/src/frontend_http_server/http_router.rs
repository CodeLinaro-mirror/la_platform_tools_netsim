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
//! This module implements a basic request router with matching of URI
//! fields. For example
//!
//!   router.add_route("/user/{id})", handle_user);
//!
//! will register a handler that matches user ids.
//!
//! This library is only used for serving the netsim client and is not
//! meant to implement all aspects of an http router.

use crate::frontend_http_server::http_request::HttpRequest;
use crate::frontend_http_server::http_response::HttpResponse;

type RequestHandler = Box<dyn Fn(&HttpRequest, &str) -> HttpResponse>;

pub struct Router {
    routes: Vec<(String, RequestHandler)>,
}

impl Router {
    pub fn new() -> Router {
        Router { routes: Vec::new() }
    }

    pub fn add_route(&mut self, route: &str, handler: RequestHandler) {
        self.routes.push((route.to_owned(), handler));
    }

    pub fn handle_request(&self, request: &HttpRequest) -> HttpResponse {
        for (route, handler) in &self.routes {
            if let Some(param) = match_route(route, &request.uri) {
                return handler(request, param);
            }
        }
        let body = format!("404 Not found (netsim): HttpRouter unknown uri {}", request.uri);
        HttpResponse::new_404(body.into_bytes())
    }
}

/// Match the uri against the route and return extracted parameter or
/// None.
///
/// Example:
///   pattern: "/users/{id}/info"
///   uri: "/users/33/info"
///   result: Some("33")
///
fn match_route<'a>(route: &str, uri: &'a str) -> Option<&'a str> {
    let open = route.find('{');
    let close = route.find('}');

    // check for literal routes with no parameter
    if open.is_none() && close.is_none() {
        return if route == uri { Some("") } else { None };
    }

    // check for internal errors in the app's route table.
    if open.is_none() || close.is_none() || open.unwrap() > close.unwrap() {
        panic!("Malformed route pattern: {route}");
    }

    // check for match of route like "/user/{id}/info"
    let open = open.unwrap();
    let close = close.unwrap();
    let prefix = &route[0..open];
    let suffix = &route[close + 1..];

    if uri.starts_with(prefix) && uri.ends_with(suffix) {
        Some(&uri[prefix.len()..(uri.len() - suffix.len())])
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend_http_server::http_response::HttpHeaders;

    fn handle_index(_request: &HttpRequest, _param: &str) -> HttpResponse {
        HttpResponse::new_200("text/html", b"Hello, world!".to_vec())
    }

    fn handle_user(_request: &HttpRequest, user_id: &str) -> HttpResponse {
        HttpResponse::new_200("application/json", format!("Hello, {user_id}!").as_bytes().to_vec())
    }

    #[test]
    fn test_match_route() {
        assert_eq!(match_route("/user/{id}", "/user/1920"), Some("1920"));
        assert_eq!(match_route("/user/{id}/info", "/user/123/info"), Some("123"));
        assert_eq!(match_route("{id}/user/info", "123/user/info"), Some("123"));
        assert_eq!(match_route("/{id}/", "/123/"), Some("123"));
        assert_eq!(match_route("/user", "/user"), Some(""));
        assert_eq!(match_route("/", "/"), Some(""));
        assert_eq!(match_route("a", "b"), None);
        assert_eq!(match_route("/{id}", "|123"), None);
        assert_eq!(match_route("{id}/", "123|"), None);
        assert_eq!(match_route("/{id}/", "/123|"), None);
        assert_eq!(match_route("/{id}/", "|123/"), None);
    }

    #[test]
    fn test_handle_request() {
        let mut router = Router::new();
        router.add_route("/", Box::new(handle_index));
        router.add_route("/user/{id}", Box::new(handle_user));
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

    #[test]
    fn test_mismatch_uri() {
        let mut router = Router::new();
        router.add_route("/user/{id}", Box::new(handle_user));
        let request = HttpRequest {
            method: "GET".to_string(),
            uri: "/player/1920".to_string(),
            version: "HTTP/1.1".to_string(),
            headers: HttpHeaders::new(),
            body: vec![],
        };
        let response = router.handle_request(&request);
        assert_eq!(response.status_code, 404);
        let expected_body =
            format!("404 Not found (netsim): HttpRouter unknown uri {}", &request.uri);
        assert_eq!(response.body, expected_body.as_bytes());
        let content_type = response.headers.get("Content-Type");
        assert!(content_type.is_some());
        assert_eq!(content_type.unwrap(), "text/plain");
    }
}
