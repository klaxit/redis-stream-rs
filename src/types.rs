#[derive(Clone, Debug)]
pub enum StartPosition {
  EndOfStream,
  Other(String),
  StartOfStream,
}

/// Builder options for `Consumer::init`.
#[derive(Debug)]
pub struct ConsumerOpts {
  pub consumer_name: Option<String>,
  /// Maximum number of messages to read from the stream in one batch.
  pub count: usize,
  pub create_stream_if_not_exists: bool,
  pub group_name: Option<String>,
  pub process_pending: bool,
  pub start_pos: StartPosition,
  /// Maximum blocking time to wait for messages on Redis.
  pub timeout: usize,
}

impl Default for ConsumerOpts {
  fn default() -> Self {
    Self {
      consumer_name: None,
      count: 10,
      create_stream_if_not_exists: true,
      group_name: None,
      process_pending: true,
      start_pos: StartPosition::EndOfStream,
      timeout: 2_000,
    }
  }
}

impl ConsumerOpts {
  pub fn consumer_name(mut self, consumer_name: &str) -> Self {
    self.consumer_name = Some(consumer_name.to_string());
    self
  }

  pub fn count(mut self, count: usize) -> Self {
    self.count = count;
    self
  }

  pub fn create_stream_if_not_exists(mut self, create_stream_if_not_exists: bool) -> Self {
    self.create_stream_if_not_exists = create_stream_if_not_exists;
    self
  }

  pub fn group_name(mut self, group_name: &str) -> Self {
    self.group_name = Some(group_name.to_string());
    self
  }

  pub fn process_pending(mut self, process_pending: bool) -> Self {
    self.process_pending = process_pending;
    self
  }

  pub fn start_pos(mut self, start_pos: StartPosition) -> Self {
    self.start_pos = start_pos;
    self
  }

  pub fn timeout(mut self, timeout: usize) -> Self {
    self.timeout = timeout;
    self
  }
}
