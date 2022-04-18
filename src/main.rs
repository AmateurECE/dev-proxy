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
        self.client.request(request)
    }
}

///////////////////////////////////////////////////////////////////////////////
// StaticFileFuture
////

struct StaticFileFuture {
    path: PathBuf,
}

impl Future for StaticFileFuture {
    type Output = Response<Body>;
    fn poll(self: Pin<&mut Self>, _context: &mut Context<'_>) ->
        Poll<Self::Output>
    {
        unimplemented!()
    }
}

///////////////////////////////////////////////////////////////////////////////
// Service
////

#[derive(Clone)]
struct DevProxService {
    root: PathBuf,
}

impl DevProxService {
    pub fn new(root: PathBuf) -> Self {
        DevProxService { root }
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
    let service = DevProxService::new(PathBuf::from("."));
    hyper::Server::bind(&"127.0.0.1:3000".parse().unwrap())
        .serve(make_service_fn(|_: &AddrStream| {
            let service = service.clone();
            async move { Ok::<_, Infallible>(service) }
        }))
        .await
        .unwrap();
}

///////////////////////////////////////////////////////////////////////////////
