<h1 align="center">stop-token</h1>
<div align="center">
  <strong>
    Cooperative cancellation for async Rust
  </strong>
</div>

<br />

<div align="center">
  <!-- Crates version -->
  <a href="https://crates.io/crates/stop-token">
    <img src="https://img.shields.io/crates/v/stop-token.svg?style=flat-square"
    alt="Crates.io version" />
  </a>
  <!-- Downloads -->
  <a href="https://crates.io/crates/stop-token">
    <img src="https://img.shields.io/crates/d/stop-token.svg?style=flat-square"
      alt="Download" />
  </a>
  <!-- docs.rs docs -->
  <a href="https://docs.rs/stop-token">
    <img src="https://img.shields.io/badge/docs-latest-blue.svg?style=flat-square"
      alt="docs.rs docs" />
  </a>
</div>

<div align="center">
  <h3>
    <a href="https://docs.rs/stop-token">
      API Docs
    </a>
    <span> | </span>
    <a href="https://github.com/async-rs/stop-token/releases">
      Releases
    </a>
    <span> | </span>
    <a href="https://github.com/async-rs/stop-token/blob/master.github/CONTRIBUTING.md">
      Contributing
    </a>
  </h3>
</div>

See crate docs for details

You can use this crate to create a deadline received through a `StopToken`:

```rust
use async_std::prelude::*;
use async_std::{stream, task};

use stop_token::prelude::*;
use stop_token::StopSource;

use std::time::Duration;

#[async_std::main]
async fn main() {
    // Create a stop source and generate a token.
    let src = StopSource::new();
    let deadline = src.token();

    // When stop source is dropped, the loop will stop.
    // Move the source to a task, and drop it after 100 millis.
    task::spawn(async move {
        task::sleep(Duration::from_millis(100)).await;
        drop(src);
    });

    // Create a stream that generates numbers until
    // it receives a signal it needs to stop.
    let mut work = stream::repeat(12u8).timeout_at(deadline);

    // Loop over each item in the stream.
    while let Some(Ok(ev)) = work.next().await {
        println!("{}", ev);
    }
}
```

Or `Instant` to create a `time`-based deadline:

```rust
use async_std::prelude::*;
use async_std::stream;

use stop_token::prelude::*;

use std::time::{Instant, Duration};

#[async_std::main]
async fn main() {
    // Create a stream that generates numbers for 100 millis.
    let deadline = Instant::now() + Duration::from_millis(100);
    let mut work = stream::repeat(12u8).timeout_at(deadline);

    // Loop over each item in the stream.
    while let Some(Ok(ev)) = work.next().await {
        println!("{}", ev);
    }
}
```
