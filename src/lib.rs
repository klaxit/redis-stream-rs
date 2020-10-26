//! Simple API for producing and consuming redis streams.
//!
//! # Basic usage:
//!
//! ```
//! use redis_stream::consumer::{Consumer, ConsumerOpts, Message};
//!
//! let redis_url =
//!   std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
//!
//! let mut redis = redis::Client::open(redis_url)
//!   .expect("client")
//!   .get_connection()
//!   .expect("connection");
//!
//! // Message handler
//! let handler = |_id: &str, message: &Message| {
//!   // do something
//!   Ok(())
//! };
//!
//! // Consumer config
//! let opts = ConsumerOpts::default();
//! let mut consumer = Consumer::init(&mut redis, "my-stream", handler, opts).expect("consumer");
//!
//! // Consume some messages through handler.
//! consumer.consume().expect("consume messages");
//!
//! // Clean up redis
//! use redis::Commands;
//! redis.del::<&str, bool>("my-stream").expect("del");
//! ```
//!
//! # Consumer groups usage:
//!
//! ```
//! use redis_stream::consumer::{Consumer, ConsumerOpts, Message};
//!
//! let redis_url =
//!   std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
//!
//! let mut redis = redis::Client::open(redis_url)
//!   .expect("client")
//!   .get_connection()
//!   .expect("connection");
//!
//! // Message handler
//! let handler = |_id: &str, message: &Message| {
//!   // do something
//!   Ok(())
//! };
//!
//! // Consumer config
//! let opts = ConsumerOpts::default().group("my-group", "worker.1");
//! let mut consumer = Consumer::init(&mut redis, "my-stream-2", handler, opts).unwrap();
//!
//! // Consume some messages through handler.
//! consumer.consume().expect("consume messages");
//!
//! // Clean up redis
//! use redis::Commands;
//! redis.xgroup_destroy::<&str, &str, bool>("my-stream-2", "my-group").expect("xgroup destroy");
//! redis.del::<&str, bool>("my-stream-2").expect("del");
//! ```
//!
//! see:
//!
//! - [`ConsumerOpts`](types/struct.ConsumerOpts.html)
//! - [`Consumer::init`](consumer/struct.Consumer.html#method.init)
//! - [`Consumer::consume`](consumer/struct.Consumer.html#method.consume)
//! - [`produce`](fn.produce.html)
use anyhow::{Context, Result};
use redis::{Commands, Connection};

pub mod consumer;
pub mod types;

/// Produces a new message into a Redis stream.
pub fn produce(
  redis: &mut Connection,
  stream: &str,
  key_values: &[(&str, &str)],
) -> Result<String> {
  let id = redis
    .xadd::<&str, &str, &str, &str, String>(stream, "*", key_values)
    .context(format!(
      "failed to run redis command:\n\
       XADD {} * {}",
      stream,
      key_values
        .iter()
        .map(|(k, v)| format!("{} {}", k, v))
        .collect::<Vec<String>>()
        .join(" ")
    ))?;
  Ok(id)
}

#[cfg(test)]
pub mod test_helpers {
  use rand::distributions::Alphanumeric;
  use rand::{thread_rng, Rng};
  use redis::{Commands, Connection, RedisResult};

  pub fn delete_stream(stream: &str) {
    redis_connection().del::<&str, bool>(stream).unwrap();
  }

  pub fn key_exists(redis: &mut Connection, key: &str) -> bool {
    let exists: RedisResult<bool> = redis.exists(key);
    exists.unwrap()
  }

  pub fn redis_connection() -> Connection {
    let redis_url =
      std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    redis::Client::open(redis_url)
      .expect("failed to open redis client")
      .get_connection()
      .expect("failed to get redis connection")
  }

  pub fn random_string(n: usize) -> String {
    thread_rng().sample_iter(&Alphanumeric).take(n).collect()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_helpers::*;
  use regex::Regex;

  #[test]
  fn test_produce() -> Result<()> {
    let mut redis = redis_connection();

    let key_values = &[("temperature", "31")];
    let stream = &format!("test-stream-{}", random_string(25));
    let id =
      produce(&mut redis, stream, key_values).context("failed to produce entry to stream")?;
    let re = Regex::new(r"^\d+-\d+$").unwrap();
    assert!(re.is_match(&id), "{:?} doesn't match Regex: {:?}", id, re);
    delete_stream(stream);

    Ok(())
  }
}
