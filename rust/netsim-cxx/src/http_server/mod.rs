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

pub(crate) mod http_request;
mod http_response;
mod http_router;
pub(crate) mod server_response;
mod thread_pool;

use crate::captures::handlers::*;
use crate::devices::devices_handler::handle_device;
use crate::http_server::http_router::Router;
use crate::http_server::server_response::{
    ResponseWritable, ServerResponseWritable, ServerResponseWriter,
};
use crate::transport::websocket::{handle_websocket, run_websocket_transport};
use crate::version::VERSION;

use crate::http_server::thread_pool::ThreadPool;

use http::{Request, Uri};
use log::{info, warn};
use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::fs;
use std::io::BufReader;
use std::net::TcpListener;
use std::net::TcpStream;
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use std::thread;

use self::http_request::parse_http_request;

const PATH_PREFIXES: [&str; 4] = ["js", "js/netsim", "assets", "node_modules/tslib"];
const DEFAULT_HTTP_PORT: u16 = 7681;

/// Start the HTTP Server.

pub fn run_http_server(instance_num: u16) {
    let _ = thread::Builder::new().name("http_server".to_string()).spawn(move || {
        let http_port = DEFAULT_HTTP_PORT + instance_num;
        let listener = match TcpListener::bind(format!("127.0.0.1:{}", http_port)) {
            Ok(listener) => listener,
            Err(e) => {
                warn!("bind error in netsimd frontend http server. {}", e);
                return;
            }
        };
        let pool = ThreadPool::new(4);
        info!("Frontend http server is listening on http://localhost:{}", http_port);
        let valid_files = Arc::new(create_filename_hash_set());
        for stream in listener.incoming() {
            let stream = stream.unwrap();
            let valid_files = valid_files.clone();
            pool.execute(move || {
                handle_connection(stream, valid_files);
            });
        }
        info!("Shutting down frontend http server.");
    });
}

fn ui_path(suffix: &str) -> PathBuf {
    let mut path = std::env::current_exe().unwrap();
    path.pop();
    path.push("netsim-ui");
    for subpath in suffix.split('/') {
        path.push(subpath);
    }
    path
}

fn create_filename_hash_set() -> HashSet<String> {
    let mut valid_files: HashSet<String> = HashSet::new();
    for path_prefix in PATH_PREFIXES {
        let dir_path = ui_path(path_prefix);
        if let Ok(mut file) = fs::read_dir(dir_path) {
            while let Some(Ok(entry)) = file.next() {
                valid_files.insert(entry.path().to_str().unwrap().to_string());
            }
        } else {
            warn!("netsim-ui doesn't exist");
        }
    }
    valid_files
}

fn check_valid_file_path(path: &str, valid_files: &HashSet<String>) -> bool {
    let filepath = match path.strip_prefix('/') {
        Some(stripped_path) => ui_path(stripped_path),
        None => ui_path(path),
    };
    valid_files.contains(filepath.as_path().to_str().unwrap())
}

fn to_content_type(file_path: &Path) -> &str {
    match file_path.extension().and_then(OsStr::to_str) {
        Some("html") => "text/html",
        Some("txt") => "text/plain",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("png") => "image/png",
        Some("js") => "application/javascript",
        Some("svg") => "image/svg+xml",
        _ => "application/octet-stream",
    }
}

fn handle_file(method: &str, path: &str, writer: ResponseWritable) {
    if method == "GET" {
        let filepath = match path.strip_prefix('/') {
            Some(stripped_path) => ui_path(stripped_path),
            None => ui_path(path),
        };
        if let Ok(body) = fs::read(&filepath) {
            writer.put_ok_with_vec(to_content_type(&filepath), body, vec![]);
            return;
        }
    }
    let body = format!("404 not found (netsim): handle_file with unknown path {path}");
    writer.put_error(404, body.as_str());
}

// TODO handlers accept additional "context" including filepath
fn handle_index(request: &Request<Vec<u8>>, _param: &str, writer: ResponseWritable) {
    handle_file(request.method().as_str(), "index.html", writer)
}

fn handle_static(request: &Request<Vec<u8>>, path: &str, writer: ResponseWritable) {
    // The path verification happens in the closure wrapper around handle_static.
    handle_file(request.method().as_str(), path, writer)
}

fn handle_version(_request: &Request<Vec<u8>>, _param: &str, writer: ResponseWritable) {
    let body = format!("{{\"version\": \"{}\"}}", VERSION);
    writer.put_ok("text/plain", body.as_str(), vec![]);
}

/// Collect queries and output key and values into Vec
pub fn collect_query(param: &str) -> Result<HashMap<&str, &str>, &str> {
    let mut result = HashMap::new();
    for word in param.split('&') {
        let equal = word.find('=');
        match equal {
            Some(equal) => {
                if result.insert(&word[..equal], &word[equal + 1..]).is_some() {
                    return Err("Query has duplicate keys");
                }
            }
            None => return Err("Invalid query string"),
        }
    }
    // TODO: Check if initial ChipInfo is included
    Ok(result)
}

fn handle_dev(request: &Request<Vec<u8>>, _param: &str, writer: ResponseWritable) {
    handle_file(request.method().as_str(), "dev.html", writer)
}

fn handle_connection(mut stream: TcpStream, valid_files: Arc<HashSet<String>>) {
    let mut router = Router::new();
    router.add_route(Uri::from_static("/"), Box::new(handle_index));
    router.add_route(Uri::from_static("/version"), Box::new(handle_version));
    router.add_route(Uri::from_static("/v1/devices"), Box::new(handle_device));
    router.add_route(Uri::from_static(r"/v1/devices/{id}"), Box::new(handle_device));
    router.add_route(Uri::from_static("/v1/captures"), Box::new(handle_capture));
    router.add_route(Uri::from_static(r"/v1/captures/{id}"), Box::new(handle_capture));
    router.add_route(Uri::from_static(r"/v1/websocket"), Box::new(handle_websocket));

    // Adding additional routes in dev mode.
    if crate::config::get_dev() {
        router.add_route(Uri::from_static("/dev"), Box::new(handle_dev));
    }

    // A closure for checking if path is a static file we wish to serve, and call handle_static
    let handle_static_wrapper =
        move |request: &Request<Vec<u8>>, path: &str, writer: ResponseWritable| {
            for prefix in PATH_PREFIXES {
                let new_path = format!("{prefix}/{path}");
                if check_valid_file_path(new_path.as_str(), &valid_files) {
                    handle_static(request, new_path.as_str(), writer);
                    return;
                }
            }
            let body = format!("404 not found (netsim): Invalid path {path}");
            writer.put_error(404, body.as_str());
        };

    // Connecting all path prefixes to handle_static_wrapper
    for prefix in PATH_PREFIXES {
        router.add_route(
            Uri::from_str(format!(r"/{prefix}/{{path}}").as_str()).unwrap(),
            Box::new(handle_static_wrapper.clone()),
        )
    }

    if let Ok(request) = parse_http_request::<&TcpStream>(&mut BufReader::new(&stream)) {
        let mut response_writer = ServerResponseWriter::new(&mut stream);
        router.handle_request(&request, &mut response_writer);
        if let Some(response) = response_writer.get_response() {
            if response.status_code == 101 {
                let query_start = request.uri().to_string().find('?').unwrap();
                let binding = request.uri().to_string();
                let queries = collect_query(&binding[query_start + 1..]).unwrap();
                run_websocket_transport(stream, queries);
            }
        }
    } else {
        let mut response_writer = ServerResponseWriter::new(&mut stream);
        let body = "404 not found (netsim): parse header failed";
        response_writer.put_error(404, body);
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collect_query() {
        // Single query pair
        let mut expected = HashMap::new();
        expected.insert("name", "hello");
        assert_eq!(collect_query("name=hello"), Ok(expected));

        // Multiple query pair
        let mut expected = HashMap::new();
        expected.insert("name", "hello");
        expected.insert("kind", "bt");
        assert_eq!(collect_query("name=hello&kind=bt"), Ok(expected));

        // Invalid query string or duplicate keys
        assert_eq!(collect_query("hello"), Err("Invalid query string"));
        assert_eq!(collect_query("name=hello&name=world"), Err("Query has duplicate keys"))
    }
}
