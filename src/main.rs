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
use core::future::Future;
use core::pin::Pin;

use std::env::current_dir;
use std::error::Error;
use std::fs::File;
use std::io::{self, Read};
use std::path::PathBuf;
use std::fmt;

use hyper::{
    Body, Client,
    client::{connect::HttpConnector, ResponseFuture},
    Request, Response,
    server::conn::AddrStream,
    service::{make_service_fn, Service},
    Uri,
};

///////////////////////////////////////////////////////////////////////////////
// ProxyError
////

#[derive(Debug)]
pub enum ProxyError {
    Proxy(io::Error),
    Http(hyper::Error),
}

impl fmt::Display for ProxyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            Self::Proxy(error) => write!(f, "{}", error),
            Self::Http(error) => write!(f, "{}", error),
        }
    }
}

impl From<io::Error> for ProxyError {
    fn from(error: io::Error) -> Self {
        Self::Proxy(error)
    }
}

impl From<hyper::Error> for ProxyError {
    fn from(error: hyper::Error) -> Self {
        Self::Http(error)
    }
}

impl Error for ProxyError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

///////////////////////////////////////////////////////////////////////////////
// ProxyResponseFuture
////

struct ProxyResponseFuture(ResponseFuture);
impl Future for ProxyResponseFuture {
    type Output = Result<Response<Body>, ProxyError>;
    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) ->
        Poll<Self::Output>
    {
        match Pin::new(&mut self.0).poll(context) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(response) => match response {
                Ok(response) => Poll::Ready(Ok(response)),
                Err(err) => Poll::Ready(Err(err.into())),
            },
        }
    }
}

impl From<ResponseFuture> for ProxyResponseFuture {
    fn from(response: ResponseFuture) -> Self {
        Self(response)
    }
}

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

    pub fn matches(&self, path: &str) -> bool {
        path.starts_with(&self.route)
    }

    pub fn request(&self, request: Request<Body>) -> ProxyResponseFuture {
        let uri: Uri = (
            self.proxy.to_string()
                + request.uri().path().strip_prefix(&self.route).unwrap())
            .parse().unwrap();
        let proxy_request = Request::builder()
            .method(request.method())
            .uri(uri)
            .body(request.into_body())
            .unwrap();
        self.client.request(proxy_request).into()
    }
}

///////////////////////////////////////////////////////////////////////////////
// StaticFileFuture
////

struct StaticFileFuture {
    path: PathBuf,
}

impl StaticFileFuture {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl Future for StaticFileFuture {
    type Output = Result<Response<Body>, ProxyError>;
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
                    Err(error) => Err(error.into()),
                }
            },

            Err(error) => {
                match error.kind() {
                    NotFound => Ok(
                        Response::builder().status(404)
                            .body(Body::empty()).unwrap()
                    ),
                    _ => Err(error.into()),
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
    type Error = ProxyError;
    type Future = Pin<Box<dyn Future<
            Output = Result<Self::Response, Self::Error>> + Send + Sync>>;

    fn poll_ready(&mut self, _context: &mut Context<'_>) ->
        Poll<Result<(), Self::Error>>
    { Ok(()).into() }

    fn call(&mut self, request: Request<Body>) -> Self::Future {
        let path = request.uri().path();
        if let Some(proxy) = self.proxies.iter().find(|p| p.matches(path)) {
            return Box::pin(proxy.request(request));
        }

        Box::pin(StaticFileFuture::new(
            self.root.join(path.strip_prefix("/").unwrap())))
    }
}

///////////////////////////////////////////////////////////////////////////////
// Main
////

#[tokio::main]
async fn main() {
    let mut service = DevProxService::new(current_dir().unwrap());
    service.proxy(ProxyRoute::new(
        "/api".to_string(),
        "http://localhost:3000/api".parse().unwrap()
    ));
    hyper::Server::bind(&"127.0.0.1:8080".parse().unwrap())
        .serve(make_service_fn(|_: &AddrStream| {
            let service = service.clone();
            async move { Ok::<_, Infallible>(service) }
        }))
        .await
        .unwrap();
}

///////////////////////////////////////////////////////////////////////////////
