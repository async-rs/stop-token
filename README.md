# Cooperative cancellation for [async-std](https://async.rs/).

Status: experimental.

See crate docs for details

```rust
use stop_token::StopToken;

async fn do_work(work: impl Stream<Item = Event>, stop_token: StopToken) {
    // The `work` stream will end early: as soon as `stop_token` is cancelled. 
    let mut work = stop_token.stop_stream(work);
    while let Some(event) = work.next().await {
        process_event(event).await
    }
}
```
