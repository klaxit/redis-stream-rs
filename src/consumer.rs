use anyhow::{Context, Result};
use redis::streams::{StreamReadOptions, StreamReadReply};
use redis::{Commands, Connection, RedisResult, Value};
use std::collections::HashMap;

pub use super::types::{ConsumerOpts, StartPosition};

pub type Message = HashMap<String, Value>;
// pub type MessageHandler = Fn(&mut Connection, &str, &Message) -> Result<()>;

// A Consumer or Group Consumer handling connection to Redis and able to consume
// messages.
pub struct Consumer<'a, F>
where
  F: FnMut(&str, &Message) -> Result<()>,
{
  pub count: Option<usize>,
  pub group: Option<(String, String)>,
  pub handled_messages: u32,
  pub handler: F,
  pub next_pos: String,
  pub process_pending: bool,
  pub redis: &'a mut Connection,
  pub stream: String,
  pub timeout: usize,
}

impl<'a, F> Consumer<'a, F>
where
  F: FnMut(&str, &Message) -> Result<()>,
{
  /// Initializes a new `stream::Consumer`.
  pub fn init(
    redis: &'a mut Connection,
    stream: &str,
    handler: F,
    opts: ConsumerOpts,
  ) -> Result<Self> {
    let count = opts.count;
    let timeout = opts.timeout;
    let group = opts.group;
    let create_stream_if_not_exists = opts.create_stream_if_not_exists;
    let process_pending = opts.process_pending;
    let start_pos = opts.start_pos;

    let (group_create_pos, consumer_start_pos) = positions(&group, process_pending, start_pos);

    if let Some((group_name, _)) = &group {
      ensure_stream_and_group(
        redis,
        &stream,
        group_name.as_ref(),
        &group_create_pos.unwrap(),
        create_stream_if_not_exists,
      )?;
    }

    Ok(Consumer {
      count,
      group,
      handled_messages: 0,
      handler,
      next_pos: consumer_start_pos,
      process_pending,
      redis,
      stream: stream.to_string(),
      timeout,
    })
  }

  /// Handle new messages from the stream, and dispatch them to the registered
  /// handler.
  pub fn consume(&mut self) -> Result<()> {
    // Prepare options for XREAD
    let opts = if let Some((group_name, consumer_name)) = &self.group {
      // We have a consumer group
      // XREADGROUP GROUP <group_name> <consumer_name> BLOCK <timeout> STREAMS <stream> <start_pos>
      StreamReadOptions::default()
        .group(group_name, consumer_name)
        .block(self.timeout)
    } else {
      // We have a simple consumer
      // XREAD BLOCK <timeout> STREAMS <stream> <start_pos>
      StreamReadOptions::default().block(self.timeout)
    };

    let stream_results: StreamReadReply =
      self
        .redis
        .xread_options(&[&self.stream], &[&self.next_pos], &opts)?;

    if !stream_results.keys.is_empty() {
      let stream = &stream_results.keys[0];

      if self.group.is_some() && self.process_pending && stream.ids.is_empty() {
        // We ran out of pending results, let's switch to processing most
        // recent.
        self.process_pending = false;
        self.next_pos = String::from(">");
        return self.consume();
      } else {
        // Process the results and set the next position to consume from
        for message in &stream.ids {
          // Keep next_post if we are in a consumer-group and it's already `>`
          if self.next_pos != ">" {
            // or take the last id
            self.next_pos = message.id.to_string();
          }
          let items = &message.map;

          self.process_message(&message.id, items)?;
        }
      }
    }

    Ok(())
  }

  /// Process a message by calling the handler and acknowledging the message-id
  /// to Redis if necessary.
  fn process_message(&mut self, id: &str, message: &Message) -> Result<()> {
    // Call handler
    (self.handler)(id, message)?;
    self.handled_messages += 1;
    // XACK if needed
    if let Some((group_name, _)) = &self.group {
      let _ack_count: i32 = self.redis.xack(&self.stream, group_name, &[id]).unwrap();
    }
    Ok(())
  }
}

// Helpers

/// Create Stream and Consumer-Group if required.
fn ensure_stream_and_group(
  redis: &mut Connection,
  stream: &str,
  group_name: &str,
  create_pos: &str,
  create_stream_if_not_exists: bool,
) -> Result<()> {
  let mut result: RedisResult<String> = if create_stream_if_not_exists {
    redis.xgroup_create_mkstream(stream, group_name, create_pos)
  } else {
    redis.xgroup_create(stream, group_name, create_pos)
  };

  // Ignore BUSYGROUP errors, it means the group already exists, which is fine.
  if let Err(err) = &result {
    if err.to_string() == "BUSYGROUP: Consumer Group name already exists" {
      result = Ok("OK".to_string());
    }
  }

  result.context(format!(
    "failed to run redis command:\n\
     XGROUP CREATE {} {} {}{}",
    stream,
    group_name,
    create_pos,
    if create_stream_if_not_exists {
      " MKSTREAM"
    } else {
      ""
    }
  ))?;

  Ok(())
}

/// Returns the tuple (`group_create_position`, `consumer_start_position`)
/// containing position args for `XGROUP CREATE`, `XREADGROUP` or `XREAD`:
/// - `group_create_pos`: Position to start in the stream if group is upserted
///   - for consumer-group:
///     - `0` for the beginning of the stream
///     - `$` for the end of the stream (new messages only)
///     - `<id>` for a specific id
/// - `start_pos`: Position to start in the stream for consumer
///   - for groups-consumers:
///     - `0` for pending messages
///     - `>` for new messages
///     - `<id>` for a specific id
///   - for stream-consumers:
///     - `0` for the beginning of the stream
///     - `$` for the end of the stream
///     - `<id>` for a specific id
fn positions(
  group_name: &Option<(String, String)>,
  process_pending: bool,
  start_pos: StartPosition,
) -> (Option<String>, String) {
  use StartPosition::*;
  let (group_create_position, consumer_start_position) =
    match (group_name, process_pending, start_pos) {
      // no group name: we'll simply XREAD starting from beginning or end
      (None, _, StartOfStream) => (None, String::from("0")),
      (None, _, EndOfStream) => (None, String::from("$")),
      (None, _, Other(id)) => (None, id),
      // group name and process pending:
      (_, true, StartOfStream) => str_to_positions("0", "0"),
      (_, true, EndOfStream) => str_to_positions("$", "0"),
      (_, true, Other(id)) => (Some(id), String::from("0")),
      // group name and don't process pending
      (_, false, StartOfStream) => str_to_positions("0", ">"),
      (_, false, EndOfStream) => str_to_positions("$", ">"),
      (_, false, Other(id)) => (Some(id.clone()), id),
    };

  (group_create_position, consumer_start_position)
}

// mainly converts &str to Strings...
#[inline]
fn str_to_positions(a: &str, b: &str) -> (Option<String>, String) {
  (Some(a.to_string()), b.to_string())
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_helpers::*;
  use anyhow::bail;
  use redis::FromRedisValue;

  fn delete_group(stream: &str, group: &str) {
    redis_connection()
      .xgroup_destroy::<&str, &str, bool>(stream, group)
      .unwrap();
  }

  #[allow(clippy::unnecessary_wraps)]
  fn print_message(_id: &str, message: &Message) -> Result<()> {
    for (k, v) in message {
      println!("{}: {}", k, String::from_redis_value(&v).unwrap());
    }
    Ok(())
  }

  #[test]
  fn test_init_options() {
    // TODO
    // - Test with default values
    // - Test with custom values
    // - Test guards (group_name or consumer_name)
  }

  #[test]
  fn test_init() {
    let mut redis = redis_connection();
    let mut redis_c = redis_connection();
    let stream = &format!("test-stream-{}", random_string(25));
    let group_name = &format!("test-group-{}", random_string(25));
    let consumer_name = &format!("test-consumer-{}", random_string(25));

    // it creates an empty stream (if opt)
    assert!(!key_exists(&mut redis, stream));

    let opts = ConsumerOpts::default()
      .create_stream_if_not_exists(true)
      .group(group_name, consumer_name);
    Consumer::init(&mut redis_c, &stream, print_message, opts).unwrap();
    assert!(key_exists(&mut redis, stream));
    // with length = 0
    let len: usize = redis.xlen(stream).unwrap();
    assert_eq!(len, 0);

    delete_group(stream, group_name);
    delete_stream(stream);

    // it doesn't create an empty stream (if !opt)
    assert!(!key_exists(&mut redis, stream));
    let opts = ConsumerOpts::default()
      .create_stream_if_not_exists(false)
      .group(group_name, consumer_name);
    assert!(Consumer::init(&mut redis_c, stream, print_message, opts).is_err());
    assert!(!key_exists(&mut redis, stream));
  }

  #[test]
  fn test_consume() {
    use std::thread;
    use std::time::Duration;

    let group_name = &format!("test-group-{}", random_string(25));
    let consumer_name = &format!("test-consumer-{}", random_string(25));
    let stream = &format!("test-stream-{}", random_string(25));
    let mut redis = redis_connection();
    let mut redis_c = redis_connection();

    crate::produce(&mut redis, stream, &[("key", "value_1")]).unwrap();

    // simple consumers
    {
      // it processes old messages if StartOfStream
      {
        let mut messages = vec![];
        let handler = |_id: &str, message: &Message| {
          messages.push(message.clone());
          Ok(())
        };
        let opts = ConsumerOpts::default().start_pos(StartPosition::StartOfStream);
        let mut consumer = Consumer::init(&mut redis_c, stream, handler, opts).unwrap();

        consumer.consume().unwrap();
        let value = String::from_redis_value(messages.pop().unwrap().get("key").unwrap()).unwrap();
        assert_eq!(value, "value_1".to_string());
      }

      // it skips old messages if EndOfStream
      {
        let messages = &mut vec![];
        let handler = |_id: &str, message: &Message| {
          messages.push(message.clone());
          Ok(())
        };
        let opts = ConsumerOpts::default().start_pos(StartPosition::EndOfStream);
        let mut consumer = Consumer::init(&mut redis_c, stream, handler, opts).unwrap();
        let stream_name = stream.clone();
        let child = thread::spawn(move || {
          // allow consumer time to call consume
          thread::sleep(Duration::from_millis(500));
          let mut redis = redis_connection();
          crate::produce(&mut redis, &stream_name, &[("key", "value_2")]).unwrap();
        });

        consumer.consume().unwrap();
        child.join().unwrap();
        let value = String::from_redis_value(messages.pop().unwrap().get("key").unwrap()).unwrap();
        assert_eq!(value, "value_2".to_string());
      }
    }

    // consumer groups
    {
      // it skips old messages if EndOfStream
      {
        let mut messages = vec![];
        let handler = |_id: &str, message: &Message| {
          messages.push(message.clone());
          bail!("I don't ack message");
        };
        let opts = ConsumerOpts::default()
          .group(group_name, consumer_name)
          .start_pos(StartPosition::EndOfStream)
          .process_pending(true);
        let mut consumer = Consumer::init(&mut redis_c, stream, handler, opts).unwrap();
        let stream_name = stream.clone();
        let child = thread::spawn(move || {
          // allow consumer time to call consume
          thread::sleep(Duration::from_millis(500));
          let mut redis = redis_connection();
          crate::produce(&mut redis, &stream_name, &[("key", "value_3")]).unwrap();
          crate::produce(&mut redis, &stream_name, &[("key", "value_4")]).unwrap();
        });

        // skip the error so we can check for pending messages in next test
        consumer.consume().unwrap_or(());
        child.join().unwrap();
        let value = String::from_redis_value(messages.pop().unwrap().get("key").unwrap()).unwrap();
        assert_eq!(value, "value_3".to_string());
      }

      // it processes pending messages if process pending is true
      {
        let mut messages = vec![];
        let handler = |_id: &str, message: &Message| {
          messages.push(message.clone());
          bail!("I don't ack message");
        };
        let opts = ConsumerOpts::default()
          .group(group_name, consumer_name)
          .start_pos(StartPosition::EndOfStream)
          .process_pending(true);
        let mut consumer = Consumer::init(&mut redis_c, stream, handler, opts).unwrap();
        // skip the error so we can check pending messages are skipped in next test
        consumer.consume().unwrap_or(());
        let value = String::from_redis_value(messages.pop().unwrap().get("key").unwrap()).unwrap();
        assert_eq!(value, "value_3".to_string());
      }

      // it skips pending messages if process_pending is false
      {
        let mut messages = vec![];
        let handler = |_id: &str, message: &Message| {
          messages.push(message.clone());
          Ok(())
        };
        let opts = ConsumerOpts::default()
          .group(group_name, consumer_name)
          .start_pos(StartPosition::EndOfStream)
          .process_pending(false);
        let mut consumer = Consumer::init(&mut redis_c, stream, handler, opts).unwrap();
        consumer.consume().unwrap();
        let value = String::from_redis_value(messages.pop().unwrap().get("key").unwrap()).unwrap();
        assert_eq!(value, "value_4".to_string());
      }

      // it ack messages
      {
        let mut messages = vec![];
        let handler = |_id: &str, message: &Message| {
          messages.push(message.clone());
          Ok(())
        };
        let opts = ConsumerOpts::default()
          .group(group_name, consumer_name)
          .start_pos(StartPosition::EndOfStream)
          .process_pending(true);
        let mut consumer = Consumer::init(&mut redis_c, stream, handler, opts).unwrap();
        consumer.consume().unwrap();
        let value = String::from_redis_value(messages.pop().unwrap().get("key").unwrap()).unwrap();
        assert_eq!(value, "value_3".to_string());

        let mut messages = vec![];
        let handler = |_id: &str, message: &Message| {
          messages.push(message.clone());
          Ok(())
        };
        let opts = ConsumerOpts::default()
          .group(group_name, consumer_name)
          .start_pos(StartPosition::EndOfStream)
          .process_pending(true);
        let mut consumer = Consumer::init(&mut redis_c, stream, handler, opts).unwrap();
        consumer.consume().unwrap();
        assert!(messages.is_empty());
      }

      delete_group(stream, group_name);

      // it processes old messages if StartOfStream
      {
        // when process_pending is false
        let mut messages = vec![];
        let handler = |_id: &str, message: &Message| {
          messages.push(message.clone());
          Ok(())
        };
        let opts = ConsumerOpts::default()
          .group(group_name, consumer_name)
          .start_pos(StartPosition::StartOfStream)
          .process_pending(false);
        let mut consumer = Consumer::init(&mut redis_c, stream, handler, opts).unwrap();
        consumer.consume().unwrap();
        let value = String::from_redis_value(messages.pop().unwrap().get("key").unwrap()).unwrap();
        assert_eq!(value, "value_4".to_string());
        let value = String::from_redis_value(messages.pop().unwrap().get("key").unwrap()).unwrap();
        assert_eq!(value, "value_3".to_string());
        let value = String::from_redis_value(messages.pop().unwrap().get("key").unwrap()).unwrap();
        assert_eq!(value, "value_2".to_string());
        let value = String::from_redis_value(messages.pop().unwrap().get("key").unwrap()).unwrap();
        assert_eq!(value, "value_1".to_string());

        delete_group(stream, group_name);

        // when process_pending is true
        let mut messages = vec![];
        let handler = |_id: &str, message: &Message| {
          messages.push(message.clone());
          Ok(())
        };
        let opts = ConsumerOpts::default()
          .group(group_name, consumer_name)
          .start_pos(StartPosition::StartOfStream)
          .process_pending(true);
        let mut consumer = Consumer::init(&mut redis_c, stream, handler, opts).unwrap();
        consumer.consume().unwrap();
        let value = String::from_redis_value(messages.pop().unwrap().get("key").unwrap()).unwrap();
        assert_eq!(value, "value_4".to_string());
        let value = String::from_redis_value(messages.pop().unwrap().get("key").unwrap()).unwrap();
        assert_eq!(value, "value_3".to_string());
        let value = String::from_redis_value(messages.pop().unwrap().get("key").unwrap()).unwrap();
        assert_eq!(value, "value_2".to_string());
        let value = String::from_redis_value(messages.pop().unwrap().get("key").unwrap()).unwrap();
        assert_eq!(value, "value_1".to_string());
      }

      delete_group(stream, group_name);

      // it skip old messages if EndOfStream
      {
        // when process_pending is false
        let mut messages = vec![];
        let handler = |_id: &str, message: &Message| {
          messages.push(message.clone());
          Ok(())
        };
        let opts = ConsumerOpts::default()
          .group(group_name, consumer_name)
          .start_pos(StartPosition::EndOfStream)
          .process_pending(false);
        let mut consumer = Consumer::init(&mut redis_c, stream, handler, opts).unwrap();
        consumer.consume().unwrap();
        assert!(messages.is_empty());

        delete_group(stream, group_name);

        // when process_pending is true
        let mut messages = vec![];
        let handler = |_id: &str, message: &Message| {
          messages.push(message.clone());
          Ok(())
        };
        let opts = ConsumerOpts::default()
          .group(group_name, consumer_name)
          .start_pos(StartPosition::EndOfStream)
          .process_pending(true);
        let mut consumer = Consumer::init(&mut redis_c, stream, handler, opts).unwrap();
        consumer.consume().unwrap();
        assert!(messages.is_empty());
      }
    }

    delete_group(stream, group_name);
    delete_stream(stream);
  }

  // note: `test_process_messages` is already tested by `test_consume`

  // note: `test_positions` is already tested by `test_consume` (but adding more
  // tests wouldn't hurt)

  #[test]
  fn test_ensure_stream_and_group() -> Result<()> {
    let mut redis = redis_connection();

    delete_stream("test-stream");
    ensure_stream_and_group(&mut redis, "test-stream", "test-group", "0", true)
      .context("failed to produce entry to stream")?;
    ensure_stream_and_group(&mut redis, "test-stream", "test-group", "0", true)
      .context("failed to produce entry to stream")?;

    Ok(())
  }
}
