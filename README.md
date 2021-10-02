# Cooperative cancellation for [async-std](https://async.rs/).

Status: experimental.

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
