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
use stop_token::prelude::*;
use stop_token::StopToken;

struct Event;

async fn do_work(work: impl Stream<Item = Event> + Unpin, stop: StopToken) {
    let mut work = work.until(stop);
    while let Some(Ok(event)) = work.next().await {
        process_event(event).await
    }
}

async fn process_event(_event: Event) {
}
```

Or `Duration` or `Instant` to create a `time`-based deadline:

```rust
use std::time::Instant;
use async_std::prelude::*;
use stop_token::prelude::*;
use stop_token::StopToken;

struct Event;

async fn do_work(work: impl Stream<Item = Event> + Unpin, until: Instant) {
    let mut work = work.until(until);
    while let Some(Ok(event)) = work.next().await {
        process_event(event).await
    }
}

async fn process_event(_event: Event) {
}
```
