use futures_util::StreamExt;
use tokio_stream::wrappers::ReceiverStream;
use tonic::metadata::MetadataMap;
use tonic::{Code, Streaming};
use tonic::{Request, Response, Status, transport::Server};
// use tonic_reflection::server::Builder as ReflectionBuilder;

use hello_world::greeter_server::{Greeter, GreeterServer};
use hello_world::{HelloReply, HelloRequest};
// pub const FILE_DESCRIPTOR_SET: &[u8] = tonic::include_file_descriptor_set!("helloworld_descriptor");
pub mod hello_world {
    tonic::include_proto!("helloworld");
}

#[derive(Debug, Default)]
pub struct MyGreeter {}

#[tonic::async_trait]
impl Greeter for MyGreeter {
    async fn say_hello(
        &self,
        request: Request<HelloRequest>,
    ) -> Result<Response<HelloReply>, Status> {
        println!("Got a request: {:?}", request);

        request
            .metadata()
            .clone()
            .into_headers()
            .iter()
            .for_each(|(key, value)| {
                println!("Metadata Key: {}, Value: {:?}", key, value);
            });
        // return Err(Status::with_metadata(Code::Ok, "Something went wrong", {
        //     let mut trailers = MetadataMap::new();
        //     trailers.insert("debug-info", "some-trailer-value".parse().unwrap());
        //     trailers
        // }));
        let reply = HelloReply {
            message: format!("Hello {}!", request.into_inner().name),
        };
        // reply.
        let mut res = Response::new(reply);
        res.metadata_mut()
            .insert("custom-header", "custom-value".parse().unwrap());
        Ok(res)
    }
    // type SayHelloStreamStream: futures_core::Stream<Item = Result<HelloReply, tonic::Status>>
    //     + Send
    //     + 'static;
    type SayHelloStreamStream = ReceiverStream<Result<HelloReply, Status>>;
    async fn say_hello_stream(
        &self,
        // request: Request<Streaming<HelloRequest>>,
        request: Request<HelloRequest>,
    ) -> Result<Response<Self::SayHelloStreamStream>, Status> {
        // let name = request.into_inner().message().await?.unwrap().name;
        let name = request.into_inner().name;

        // Create a channel to send streaming replies
        let (tx, rx) = tokio::sync::mpsc::channel(4);

        // let name = name.clone();
        tokio::spawn(async move {
            // Send one message first
            if tx
                .send(Ok(HelloReply {
                    message: "first ok".into(),
                }))
                .await
                .is_err()
            {
                return;
            }
            // tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            if tx
                .send(Ok(HelloReply {
                    message: "second ok".into(),
                }))
                .await
                .is_err()
            {
                return;
            }

            // tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            // Then simulate an error condition
            let mut meta = MetadataMap::new();
            meta.insert("x-reason", "server-stream-error".parse().unwrap());

            // let status = Status::with_metadata(Code::Ok, "stream failure with trailers", meta);
            let status = Status::with_metadata(Code::Ok, "sss", meta);

            // Sending Err(Status) ends the stream early with gRPC error + trailers
            let _ = tx.send(Err(status)).await;
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    // type SayHelloBiStreamStream: ReceiverStream<Result<HelloReply, tonic::Status>>;
    type SayHelloBiStreamStream = ReceiverStream<Result<HelloReply, Status>>;
    // type SayHelloBiStreamStream = Stream<Item = Result<HelloReply, Status>> + Send + 'static;
    async fn say_hello_bi_stream(
        &self,
        // request: Request<Streaming<HelloRequest>>,
        request: Request<Streaming<HelloRequest>>,
    ) -> Result<Response<Self::SayHelloStreamStream>, Status> {
        // let name = request.into_inner().message().await?.unwrap().name;
        let mut inbound = request.into_inner();

        // Create a channel to send streaming replies
        let (tx, rx) = tokio::sync::mpsc::channel(4);

        // let name = name.clone();
        tokio::spawn(async move {
            while let Some(result) = inbound.next().await {
                if let Ok(req) = result {
                    println!("Got a request: {:?}", req);
                    if req.name == "client request 1" {
                        if tx
                            .send(Ok(HelloReply {
                                message: "first ok".into(),
                            }))
                            .await
                            .is_err()
                        {
                            return;
                        }
                    }
                    match req.name == "client request 2" {
                        true => {
                            if tx
                                .send(Ok(HelloReply {
                                    message: "second ok".into(),
                                }))
                                .await
                                .is_err()
                            {
                                return;
                            }
                        }
                        false => (),
                    }
                }
                // // Send one message first
                // if tx
                //     .send(Ok(HelloReply {
                //         message: "first ok".into(),
                //     }))
                //     .await
                //     .is_err()
                // {
                //     return;
                // }
                // tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                // if tx
                //     .send(Ok(HelloReply {
                //         message: "second ok".into(),
                //     }))
                //     .await
                //     .is_err()
                // {
                //     return;
                // }
                //
                // tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                // // Then simulate an error condition
                // let mut meta = MetadataMap::new();
                // meta.insert("x-reason", "server-stream-error".parse().unwrap());
                //
                // // let status = Status::with_metadata(Code::Ok, "stream failure with trailers", meta);
                // let status = Status::with_metadata(Code::Ok, "sss", meta);
                //
                // // Sending Err(Status) ends the stream early with gRPC error + trailers
                // let _ = tx.send(Err(status)).await;
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }
}
