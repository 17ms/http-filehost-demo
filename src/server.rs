/*

Really simple HTTP-server to serve files asynchronously to the recipients. Fileroot defaults to ./data.

Usage: ./http-filehost-demo --rootdir <PATH>

*/

use clap::Parser;
use hyper::{
    service::{make_service_fn, service_fn},
    Body, Method, Request, Response, Server,
};
use std::{
    collections::HashMap, convert::Infallible, env::current_dir, fs, net::SocketAddr, path::PathBuf,
};

#[derive(Parser, Debug)]
struct Args {
    #[clap(short = 'r', value_parser = clap::value_parser!(PathBuf))]
    rootdir: Option<PathBuf>,
}

pub async fn create_server() -> Result<(), Box<dyn std::error::Error>> {
    // Create a simple HTTP-server and host a send_file-service

    let _args = Args::parse(); // Parse --help before starting a service

    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    let service = service_fn(file_service);
    let make_svc = make_service_fn(|_conn| async move { Ok::<_, Infallible>(service) });
    let server = Server::bind(&addr).serve(make_svc);

    if let Err(e) = server.await {
        eprintln!("{e}");
    }

    Ok(())
}

async fn file_service(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    // HTTP-service for serving files

    let args = Args::parse();
    let encoding_map = collect_hashmap().expect("Couldn't read encoding.json");

    match *req.method() {
        Method::GET => {
            // Use defined root-directory for files or default to ./data

            let data_root = match args.rootdir {
                Some(path) => path,
                None => {
                    let mut data_root = current_dir().unwrap();
                    data_root.push("data");

                    data_root
                }
            };

            match path_from_req(req.uri().path(), &data_root, &encoding_map) {
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
            println!("Served file [{:#?}]", filepath);
            let body = contents.into();
            Ok(Response::new(body))
        }
        Err(e) => {
            if filepath.ends_with("favicon.ico") {
                Ok(Response::new(Body::from("")))
            } else {
                eprintln!("Error serving file ({:#?}): {}", filepath, e);
                Ok(Response::new(Body::from("404 File not found")))
            }
        }
    }
}

fn path_from_req(
    req_path: &str,
    data_dir: &PathBuf,
    encoding_map: &HashMap<String, String>,
) -> Option<PathBuf> {
    // Validate and match the request URI's path with files inside ./data

    if req_path.trim().is_empty() {
        None
    } else {
        let mut path = data_dir.to_owned();
        let mut extension = req_path[1..].to_string();

        for (k, v) in encoding_map {
            extension = extension.replace(k, v); // Optimize if possible
        }

        path.push(extension);
        Some(path)
    }
}

fn collect_hashmap() -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
    let content_string = fs::read_to_string("./encoding.json")?;
    let map: HashMap<String, String> = serde_json::from_str(content_string.as_str())?;

    Ok(map)
}
