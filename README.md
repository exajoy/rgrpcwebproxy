<img src="misc/logo/logo.png" width="200" />

# Griffin - A lightweight gRPC-Web to gRPC proxy

In the world of package transmission, gRPC-Web and gRPC are becoming increasingly polular.
However, the current solutions of converting gRPRC-Web to gRPC, such as Envoy,
are often too large and complex for lightweight applications.

Griffin is a lightweight proxy built on top of hyper.rs to handle gRPC-web, translating gRPC-web requests to standard gRPC requests.
Griffin's binary is only 2.5MB, 100 times smaller than Envoy's binary (250MB+).

```
grpc-web client <--> griffin (grpc-web to grpc proxy) <--> grpc server
```

```
grpc client <--> griffin <--> grpc server
```

## Features

- Interoeprability between grpc-web clients and grpc servers
- Support 2 types of grpc-web requests (unary and server streaming)
- Support 4 types standard grpc requests (unary request, server streaming, client streaming, bidi streaming)

## How to use

```ssh
griffin \
--proxy-host=127.0.0.1 \
--proxy-port=8080 \
--forward-host=127.0.0.1 \
--forward-port=3000
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

#### Requirements

rustc 1.91.0

cargo 1.81.0

#### Commands

```ssh
git clone https://github.com/exajoy/griffin
cd griffin
cargo build --release
```

## TODO

- [x] Integration tests implementation
- [ ] Telemetry support
- [ ] Health check support
- [ ] CORS support
- [ ] TLS support
- [ ] FFI to use in other languages

## Contribution

Please feel free to open issues or submit pull requests.

### Run tests (init test and integration test)

```ssh
cargo test --feature test
```
