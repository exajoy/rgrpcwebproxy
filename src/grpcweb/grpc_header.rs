// use clap::Parser;
// use http::HeaderValue;
// use http::Uri;
// use http::uri::Authority;
// use http_body_util::BodyExt;
// use http_body_util::Full;
// use hyper::body::Bytes;
// use hyper::body::Incoming;
// use hyper::{Request, Response};
// use tower::BoxError;
// use tower::Service;
//
// use crate::grpcweb::args::Args;
// use crate::grpcweb::call::GrpcWebCall;
//
// #[derive(Debug, Clone)]
// pub struct GrpcHeader<S> {
//     inner: S,
// }
// impl<S> GrpcHeader<S> {
//     pub fn new(inner: S) -> Self {
//         GrpcHeader { inner }
//     }
// }
//
// impl<S> Service<Request<Incoming>> for GrpcHeader<S>
// where
//     S: Service<
//             Request<GrpcWebCall<Full<Bytes>>>,
//             // Response = Response<GrpcWebCall<Full<Bytes>>>,
//             // Response = Response<Incoming>,
//             Response = Response<GrpcWebCall<Full<Bytes>>>,
//             Error = BoxError,
//         > + Clone,
//     // S: Service<Request<Body>, Response = Response<Body>, Error = BoxError> + Clone,
// {
//     type Response = S::Response;
//     type Error = S::Error;
//     type Future = impl Future<Output = Result<Self::Response, Self::Error>>;
//
//     fn poll_ready(
//         &mut self,
//         cx: &mut std::task::Context<'_>,
//     ) -> std::task::Poll<Result<(), Self::Error>> {
//         self.inner.poll_ready(cx)
//     }
//
//     fn call(&mut self, req: Request<Incoming>) -> Self::Future {
//         // if (is_upgrade_request(&req)) {
//         //     let (res, ws) = upgrade(req, None).unwrap();
//         //     return Ok(res);
//         // }
//         let mut inner = self.inner.clone();
//         let (mut parts, res_body) = req.into_parts();
//         let args = Args::parse();
//         // let authority = format!("", args.forward_host, args.forward_port);
//         let authority = format!("{}:{}", args.forward_host, args.forward_port);
//         let authority = Authority::from_maybe_shared(authority).unwrap();
//         // this will resplace
//
//         let path = parts.uri.path();
//         let url = format!("http://{}{}", authority.as_ref(), path);
//
//         parts.uri = Uri::from(url.parse::<Uri>().unwrap());
//         println!("Forward URL: {}", parts.uri);
//         println!("Authority: {}", authority);
//
//         async move {
//             let res_body = res_body.collect().await?;
//             let mut req = Request::from_parts(
//                 parts,
//                 GrpcWebCall::client_request(Full::<Bytes>::new(res_body.to_bytes())),
//             );
//
//             req.headers_mut().insert(
//                 hyper::header::CONTENT_TYPE,
//                 HeaderValue::from_static("application/grpc"),
//             );
//
//             req.headers_mut().remove(hyper::header::CONTENT_LENGTH);
//
//             req.headers_mut()
//                 .insert(hyper::header::HOST, authority.as_str().parse()?);
//             // let body = GrpcWebCall::client_request(req.body());
//             // let req = Request::from_parts(parts, body);
//             let res = inner.call(req).await;
//             res
//         }
//     }
// }
