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
use core::future::{self, Ready};

use hyper::{
    body::Body,
    Request, Response,
    server::conn::AddrStream,
    service::{make_service_fn, Service}
};

///////////////////////////////////////////////////////////////////////////////
// Service
////

struct DevProxService;

impl Service<Request<Body>> for DevProxService {
    type Response = Response<Body>;
    type Error = Infallible;
    type Future = Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _context: &mut Context<'_>) ->
        Poll<Result<(), Self::Error>>
    { Ok(()).into() }

    fn call(&mut self, _request: Request<Body>) -> Self::Future {
        future::ready(Ok(
            Response::builder()
                .status(200)
                .body(Body::from("Hello, world!"))
                .unwrap()
        ))
    }
}

///////////////////////////////////////////////////////////////////////////////
// Main
////

#[tokio::main]
async fn main() {
    hyper::Server::bind(&"127.0.0.1:3000".parse().unwrap())
        .serve(make_service_fn(|_: &AddrStream| {
            async move { Ok::<_, Infallible>(DevProxService) }
        }))
        .await
        .unwrap();
}

///////////////////////////////////////////////////////////////////////////////
