//! Defines types to use with the consumer commands.

#[derive(Clone, Debug)]
pub enum StartPosition {
  EndOfStream,
  Other(String),
  StartOfStream,
}

/// Builder options for [`Consumer::init`].
///
/// Configuration settings for stream consumers (simple or group).
///
/// # Basic usage
///
/// ```
/// use redis_stream::consumer::{ConsumerOpts, StartPosition};
///
/// let opts = ConsumerOpts::default().start_pos(StartPosition::StartOfStream);
/// ```
///
/// # Group consumer
///
/// Specifying a `group` (with `group_name` and `consumer_name`), will instruct
/// the [`Consumer`] to use consumer groups specific commands (like `XGROUP
/// CREATE`, `XREADGROUP` or `XACK`).
///
/// ```
/// use redis_stream::consumer::{ConsumerOpts, StartPosition};
///
/// let opts = ConsumerOpts::default()
///   .group("my-group", "consumer.1")
///   .start_pos(StartPosition::StartOfStream);
/// ```
/// [`Consumer`]: ../consumer/struct.Consumer.html
/// [`Consumer::init`]:../consumer/struct.Consumer.html#method.init
#[derive(Debug)]
pub struct ConsumerOpts {
  pub count: Option<usize>,
  pub create_stream_if_not_exists: bool,
  pub group: Option<(String, String)>,
  pub process_pending: bool,
  pub start_pos: StartPosition,
  pub timeout: usize,
}

impl Default for ConsumerOpts {
  fn default() -> Self {
    Self {
      count: None,
      create_stream_if_not_exists: true,
      group: None,
      process_pending: true,
      start_pos: StartPosition::EndOfStream,
      timeout: 2_000,
    }
  }
}

impl ConsumerOpts {
  /// Maximum number of message to read from the stream in one batch
  pub fn count(mut self, count: usize) -> Self {
    self.count = Some(count);
    self
  }

  /// Create the stream in Redis before registering the group (default: `true`).
  pub fn create_stream_if_not_exists(mut self, create_stream_if_not_exists: bool) -> Self {
    self.create_stream_if_not_exists = create_stream_if_not_exists;
    self
  }

  /// Name of the group and consumer. Enables Redis group consumer behavior if
  /// specified
  pub fn group(mut self, group_name: &str, consumer_name: &str) -> Self {
    self.group = Some((group_name.to_string(), consumer_name.to_string()));
    self
  }

  /// Start by processing pending messages before switching to real time data
  /// (default: `true`)
  pub fn process_pending(mut self, process_pending: bool) -> Self {
    self.process_pending = process_pending;
    self
  }

  /// Where to start reading messages in the stream.
  pub fn start_pos(mut self, start_pos: StartPosition) -> Self {
    self.start_pos = start_pos;
    self
  }

  /// Maximum ms duration to block waiting for messages.
  pub fn timeout(mut self, timeout: usize) -> Self {
    self.timeout = timeout;
    self
  }
}
