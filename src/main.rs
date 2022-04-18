///////////////////////////////////////////////////////////////////////////////
// NAME:            main.rs
//
// AUTHOR:          Ethan D. Twardy <ethan.twardy@gmail.com>
//
// DESCRIPTION:     Entrypoint for the server.
//
// CREATED:         04/17/2022
//
// LAST EDITED:     04/18/2022
////

use core::convert::Infallible;
use core::task::{Context, Poll};
use core::future::{self, Future};
use core::pin::Pin;

use std::env::current_dir;
use std::fs::File;
use std::io::{self, Read};
use std::path::PathBuf;

use hyper::{
    Body, Client,
    client::{connect::HttpConnector, ResponseFuture},
    Request, Response,
    server::conn::AddrStream,
    service::{make_service_fn, Service},
    Uri,
};

///////////////////////////////////////////////////////////////////////////////
// Proxy
////

#[derive(Clone)]
struct ProxyRoute {
    route: String,
    proxy: Uri,
    client: Client<HttpConnector>,
}

impl ProxyRoute {
    pub fn new(route: String, proxy: Uri) -> Self {
        Self { route, proxy, client: Client::new() }
    }

    pub fn matched(&self, path: &str) -> bool {
        path.starts_with(&self.route)
    }

    pub fn proxy(&self, request: Request<Body>) -> ResponseFuture {
        let uri: Uri = (
            self.route.to_string()
                + request.uri().path().strip_prefix(&self.route).unwrap())
            .parse().unwrap();
        let proxy_request = Request::builder()
            .method(request.method())
            .uri(uri)
            .body(request.into_body())
            .unwrap();
        self.client.request(proxy_request)
    }
}

///////////////////////////////////////////////////////////////////////////////
// StaticFileFuture
////

struct StaticFileFuture {
    path: PathBuf,
}

impl Future for StaticFileFuture {
    type Output = io::Result<Response<Body>>;
    fn poll(self: Pin<&mut Self>, _context: &mut Context<'_>) ->
        Poll<Self::Output>
    {
        use io::ErrorKind::*;

        let result = File::open(&self.path);
        let response = match result {
            Ok(mut file) => {
                let mut contents = String::new();
                match file.read_to_string(&mut contents) {
                    Ok(_) => Ok(Response::builder().status(200)
                                .body(Body::from(contents)).unwrap()),
                    Err(error) => Err(error),
                }
            },

            Err(error) => {
                match error.kind() {
                    NotFound => Ok(
                        Response::builder().status(404)
                            .body(Body::empty()).unwrap()
                    ),
                    _ => Err(error),
                }
            },
        };

        Poll::Ready(response)
    }
}

///////////////////////////////////////////////////////////////////////////////
// Service
////

#[derive(Clone)]
struct DevProxService {
    root: PathBuf,
    proxies: Vec<ProxyRoute>,
}

impl DevProxService {
    pub fn new(root: PathBuf) -> Self {
        DevProxService { root, proxies: Vec::new() }
    }

    pub fn proxy(&mut self, proxy: ProxyRoute) {
        self.proxies.push(proxy);
    }
}

impl Service<Request<Body>> for DevProxService {
    type Response = Response<Body>;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<
            Output = Result<Self::Response, Self::Error>> + Send + Sync>>;

    fn poll_ready(&mut self, _context: &mut Context<'_>) ->
        Poll<Result<(), Self::Error>>
    { Ok(()).into() }

    fn call(&mut self, _request: Request<Body>) -> Self::Future {
        Box::pin(future::ready(Ok(
            Response::builder()
                .status(200)
                .body(Body::from("Hello, world!"))
                .unwrap()
        )))
    }
}

///////////////////////////////////////////////////////////////////////////////
// Main
////

#[tokio::main]
async fn main() {
    let service = DevProxService::new(current_dir().unwrap());
    hyper::Server::bind(&"127.0.0.1:3000".parse().unwrap())
        .serve(make_service_fn(|_: &AddrStream| {
            let service = service.clone();
            async move { Ok::<_, Infallible>(service) }
        }))
        .await
        .unwrap();
}

///////////////////////////////////////////////////////////////////////////////
