/*

Simple HTTP-server to serve files asynchronously to the recipients. Built mostly with Hyper & Tokio.

Files get served automatically from a hardcoded path ./data.

*/

use clap::Parser;
use hyper::{
    service::{make_service_fn, service_fn},
    Body, Method, Request, Response, Server,
};
use std::{convert::Infallible, env::current_dir, net::SocketAddr, path::PathBuf};

#[derive(Parser, Debug)]
struct Args {
    #[clap(short = 'r', value_parser = clap::value_parser!(PathBuf))]
    rootdir: Option<PathBuf>,
}

async fn file_service(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    // HTTP-service for serving files

    match *req.method() {
        Method::GET => {
            // Use defined root-directory for files or default to ./data

            let args = Args::parse();

            let data_root = match args.rootdir {
                Some(path) => path,
                None => {
                    let mut data_root = current_dir().unwrap();
                    data_root.push("data");

                    data_root
                }
            };

            match path_from_req(req.uri().path(), &data_root) {
                Some(filepath) => serve_file(filepath).await,
                None => Ok(Response::new(Body::from("404 File not found"))),
            }
        }
        _ => Ok(Response::new(Body::from("Only GET-requests supported"))),
    }
}

async fn serve_file(filepath: PathBuf) -> Result<Response<Body>, Infallible> {
    // Serve a single file asynchronously

    match tokio::fs::read(&filepath).await {
        Ok(contents) => {
            let body = contents.into();
            Ok(Response::new(body))
        }
        Err(e) => {
            if filepath.ends_with("favicon.ico") {
                Ok(Response::new(Body::from("")))
            } else {
                eprintln!("Error serving file: {e}");
                Ok(Response::new(Body::from("404 File not found")))
            }
        }
    }
}

fn path_from_req(req_path: &str, data_dir: &PathBuf) -> Option<PathBuf> {
    // Validate and match the request URI's path with files inside ./data

    if req_path.trim().is_empty() {
        None
    } else {
        let mut path = data_dir.to_owned();
        path.push(&req_path[1..]);

        Some(path)
    }
}

pub async fn create_server() -> Result<(), Box<dyn std::error::Error>> {
    // Create a simple HTTP-server and host a send_file-service

    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    let service = service_fn(file_service);
    let make_svc = make_service_fn(|_conn| async move { Ok::<_, Infallible>(service) });
    let server = Server::bind(&addr).serve(make_svc);

    if let Err(e) = server.await {
        eprintln!("{e}");
    }

    Ok(())
}
