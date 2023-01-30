mod thread_pool;

use std::fs;
use std::io::prelude::*;
use std::net::TcpListener;
use std::net::TcpStream;
use thread_pool::ThreadPool;

const BUFFER_SIZE: usize = 1024;
const GET_RESOURCE: &str = "GET /";
const GET_ROOT: &str = "GET / HTTP/1.1\r\n";
const RESPONSE_200: &str = "HTTP/1.1 200 OK";
const RESPONSE_200_JS: &str = "HTTP/1.1 200 OK\r\nContent-Type: text/javascript";
const RESPONSE_404: &str = "HTTP/1.1 404 NOT FOUND";

fn main() {
    let listener = TcpListener::bind("127.0.0.1:7681").unwrap();
    let pool = ThreadPool::new(4);

    for stream in listener.incoming() {
        let stream = stream.unwrap();

        pool.execute(|| {
            handle_connection(stream);
        });
    }

    println!("Shutting down.");
}

fn handle_connection(mut stream: TcpStream) {
    let mut buffer = [0; BUFFER_SIZE];
    let bytes_amount = stream.read(&mut buffer).unwrap();
    if bytes_amount > BUFFER_SIZE {
        println!("error: bytes_amount > buffer_size");
    }

    let mut filepath = std::env::current_exe().unwrap();
    filepath.pop();
    filepath.push("netsim-ui");

    let (status_line, mut contents) = if buffer.starts_with(GET_ROOT.as_bytes()) {
        filepath.push("index.html");
        if !filepath.exists() {
            (RESPONSE_404, None)
        } else {
            (RESPONSE_200, Some(fs::read_to_string(filepath.as_path()).unwrap()))
        }
    } else if buffer.starts_with(GET_RESOURCE.as_bytes()) {
        let filename_slices = std::str::from_utf8(&buffer)
            .unwrap()
            .strip_prefix(GET_RESOURCE)
            .unwrap()
            .split_once(' ')
            .unwrap();
        filepath.push(filename_slices.0);
        if !filepath.exists() {
            (RESPONSE_404, None)
        } else if filename_slices.0.contains(".js") {
            (RESPONSE_200_JS, Some(fs::read_to_string(filepath.as_path()).unwrap()))
        } else {
            (RESPONSE_200, Some(fs::read_to_string(filepath.as_path()).unwrap()))
        }
    } else {
        (RESPONSE_404, None)
    };

    if contents.is_none() {
        contents = Some(String::from(include_str!("404.html")));
    }

    let response = format!(
        "{}\r\nContent-Length: {}\r\n\r\n{}",
        status_line,
        contents.as_ref().unwrap().len(),
        contents.unwrap().as_str()
    );
    stream.write_all(response.as_bytes()).unwrap();
    stream.flush().unwrap();
}
