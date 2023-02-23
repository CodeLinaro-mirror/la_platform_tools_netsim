mod http_request;
mod http_response;
mod http_router;
mod thread_pool;

extern crate frontend_proto;

use crate::frontend_http_server::http_request::HttpRequest;
use crate::frontend_http_server::http_response::HttpResponse;
use crate::frontend_http_server::http_router::Router;

use crate::frontend_http_server::thread_pool::ThreadPool;

use crate::ffi::get_devices;
use cxx::let_cxx_string;
use regex::Captures;
use std::ffi::OsStr;
use std::fs;
use std::io::BufReader;
use std::net::TcpListener;
use std::net::TcpStream;
use std::path::Path;

pub fn run_frontend_http_server() {
    let listener = TcpListener::bind("127.0.0.1:7681").unwrap();
    let pool = ThreadPool::new(4);
    println!("Frontend http server is listening on http://localhost:7681");

    for stream in listener.incoming() {
        let stream = stream.unwrap();

        pool.execute(|| {
            handle_connection(stream);
        });
    }

    println!("Shutting down frontend http server.");
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

fn handle_file(method: &str, path: &str) -> HttpResponse {
    if method == "GET" {
        let mut filepath = std::env::current_exe().unwrap();
        filepath.pop();
        filepath.push("netsim-ui");
        if let Some(stripped_path) = path.strip_prefix('/') {
            filepath.push(stripped_path);
        } else {
            filepath.push(path);
        }
        if let Ok(body) = fs::read(&filepath) {
            return HttpResponse::new_200(to_content_type(&filepath), body);
        }
    }
    println!("netsim: handle_file with unknown path {path}");
    HttpResponse::new_404()
}

// TODO handlers accept additional "context" including filepath
fn handle_index(request: &HttpRequest, _capture: Captures) -> HttpResponse {
    handle_file(&request.method, "index.html")
}

fn handle_static(request: &HttpRequest, _capture: Captures) -> HttpResponse {
    handle_file(&request.method, &request.uri)
}

fn handle_version(_request: &HttpRequest, _capture: Captures) -> HttpResponse {
    HttpResponse::new_200("text/plain", b"{version: \"123b\"}".to_vec())
}

fn handle_get_device(_request: &HttpRequest, _capture: Captures) -> HttpResponse {
    let_cxx_string!(request = "");
    let_cxx_string!(response = "");
    let_cxx_string!(error_message = "");
    let status = get_devices(&request, response.as_mut(), error_message); // call cxx method for get device
    if status == 200 {
        HttpResponse::new_200("text/plain", response.to_string().into_bytes())
    } else {
        HttpResponse::new_404()
    }
}

fn handle_connection(mut stream: TcpStream) {
    let mut router = Router::new();
    router.add_route("/", handle_index);
    router.add_route("/get-version", handle_version);
    router.add_route("/get-devices", handle_get_device);
    router.add_route("/(.+)", handle_static);
    let response =
        if let Ok(request) = HttpRequest::parse::<&TcpStream>(&mut BufReader::new(&stream)) {
            router.handle_request(&request)
        } else {
            println!("netsim: parse header failed");
            HttpResponse::new_404()
        };
    if let Err(e) = response.write_to(&mut stream) {
        println!("netsim: handle_connection error {e}");
    }
}
