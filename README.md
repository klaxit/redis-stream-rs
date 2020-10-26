# Redis Stream

[![Build Status](https://travis-ci.org/klaxit/redis-stream-rs.svg?branch=master)](https://travis-ci.org/klaxit/redis-stream-rs)
[![crates.io](http://meritbadge.herokuapp.com/redis-stream)](https://crates.io/crates/redis-stream)

A Rust high-level library to consume data from Redis streams.

This project is a slightly modified port of the Elixir
[Redix.Stream](https://github.com/compound-finance/redix_stream) library to Rust
and comes as an extension of [redis-rs](https://github.com/mitsuhiko/redis-rs).

## Installation

The crate is called `redis-stream` and you can depend on it via cargo:

```ini
[dependencies]
redis-stream = "0.1.0"
```

## Documentation

Documentation on the library can be found at
[docs.rs/redis-stream](https://docs.rs/redis-stream).

## Basic usage:

```rust
use redis_stream::consumer::{Consumer, ConsumerOpts, Message};

let redis_url =
  std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());

let mut redis = redis::Client::open(redis_url)
  .expect("client")
  .get_connection()
  .expect("connection");

// Message handler
let handler = |_id: &str, message: &Message| {
  // do something
  Ok(())
};

// Consumer config
let opts = ConsumerOpts::default();
let mut consumer = Consumer::init(&mut redis, "my-stream", handler, opts).expect("consumer");

// Consume some messages through handler.
consumer.consume().expect("consume messages");

// Clean up redis
use redis::Commands;
redis.del::<&str, bool>("my-stream").expect("del");
```

## Consumer groups usage:

```rust
use redis_stream::consumer::{Consumer, ConsumerOpts, Message};

let redis_url =
  std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());

let mut redis = redis::Client::open(redis_url)
  .expect("client")
  .get_connection()
  .expect("connection");

// Message handler
let handler = |_id: &str, message: &Message| {
  // do something
  Ok(())
};

// Consumer config
let opts = ConsumerOpts::default().group("my-group", "worker.1");
let mut consumer = Consumer::init(&mut redis, "my-stream-2", handler, opts).unwrap();

// Consume some messages through handler.
consumer.consume().expect("consume messages");

// Clean up redis
use redis::Commands;
redis.xgroup_destroy::<&str, &str, bool>("my-stream-2", "my-group").expect("xgroup destroy");
redis.del::<&str, bool>("my-stream-2").expect("del");
```

## Development

If you want to develop on the library, there are a few commands provided by the
makefile.

Run `make help` to get more info.

For testing, a `docker-compose.yml` file is also available if you need to start a local redis instance:

```sh
$ docker-compose up -d
$ make test
```
## License

Please see [LICENSE](./LICENSE)
