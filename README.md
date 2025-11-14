![Alt text](misc/logo/logo.png)

# Griffin

A lightweight proxy written in rust to handle gRPC-web, translating gRPC-web requests to standard gRPC requests.
This is the flow:

```
grpc-web client <--> griffin (grpc-web to grpc proxy) <--> grpc server
```

```
grpc client <--> griffin <--> grpc server
```

## Inspirations

[Grpc Web](https://github.com/improbable-eng/grpc-web)
[Tonic](https://github.com/hyperium/tonic)

## Implementation Documents

<https://github.com/grpc/grpc/blob/master/doc/PROTOCOL-HTTP2.md>
<https://github.com/grpc/grpc/blob/master/doc/PROTOCOL-WEB.md>
<https://datatracker.ietf.org/doc/rfc7540/>

## Installation

### Build from source

```ssh
git clone https://github.com/exajoy/griffin
cd griffin
cargo build --release

```

## Contribution

Please feel free to open issues or submit pull requests.

### Run tests

```ssh
cargo test --feature test

```
