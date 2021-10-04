//! Cooperative cancellation for [async-std](https://async.rs/).
//!
//! # Status
//!
//! Experimental. The library works as is, breaking changes will bump major
//! version, but there are no guarantees of long-term support.
//!
//! # Motivation
//!
//! Rust futures come with a build-in cancellation mechanism: dropping a future
//! prevents any further progress of the future. This is a *hard* cancellation
//! mechanism, meaning that the future can potentially be cancelled at any
//! `.await` expression.
//!
//! Sometimes, you need are more fine-grained cancellation mechanism. Imagine a
//! chat server that relays messages to peers. Sending a single message
//! potentially needs several writes on the socket object. That means that, if
//! we use hard-cancellation for client connections, a connection can be
//! abruptly terminated mid-message (even mid-emoji, if we are especially
//! unlucky). What we need here is cooperative cancellation: client connection
//! should be gracefully shutdown *between* the messages.
//!
//! More generally, if you have an event processing loop like
//!
//! ```ignore
//! while let Some(event) = work.next().await {
//!     process_event(event).await
//! }
//! ```
//!
//! you usually want to maintain an invariant that each event is either fully
//! processed or not processed at all. If you need to terminate this loop early,
//! you want to do this *between* iteration.
//!
//! # Usage
//!
//! You can use this crate to create a deadline received through a [`StopToken`]:
//!
//! ```
//! use async_std::prelude::*;
//! use stop_token::prelude::*;
//! use stop_token::StopToken;
//!
//! struct Event;
//!
//! async fn do_work(work: impl Stream<Item = Event> + Unpin, stop: StopToken) {
//!     let mut work = work.until(stop);
//!     while let Some(Ok(event)) = work.next().await {
//!         process_event(event).await
//!     }
//! }
//!
//! async fn process_event(_event: Event) {
//! }
//! ```
//!
//! Or `Duration` or `Instant` to create a [`time`]-based deadline:
//!
//! ```
//! use std::time::Instant;
//! use async_std::prelude::*;
//! use stop_token::prelude::*;
//! use stop_token::StopToken;
//!
//! struct Event;
//!
//! async fn do_work(work: impl Stream<Item = Event> + Unpin, until: Instant) {
//!     let mut work = work.until(until);
//!     while let Some(Ok(event)) = work.next().await {
//!         process_event(event).await
//!     }
//! }
//!
//! async fn process_event(_event: Event) {
//! }
//! ```
//!
//! # Features
//!
//! The `time` submodule is empty when no features are enabled. To implement [`IntoDeadline`]
//! for `Instant` and `Duration` you can enable one of the following features:
//!
//! - `async-io`: for use with the `async-std` or `smol` runtimes.
//! - `tokio`: for use with the `tokio` runtime.
//!
//! # Lineage
//!
//! The cancellation system is a subset of `C#` [`CancellationToken / CancellationTokenSource`](https://docs.microsoft.com/en-us/dotnet/standard/threading/cancellation-in-managed-threads).
//! The `StopToken / StopTokenSource` terminology is borrowed from [C++ paper P0660](https://wg21.link/p0660).

#![forbid(unsafe_code)]
#![deny(missing_debug_implementations, nonstandard_style, rust_2018_idioms)]
#![warn(missing_docs, future_incompatible, unreachable_pub)]

pub mod future;
pub mod stream;
pub mod time;

mod deadline;
mod stop_source;

pub use deadline::{IntoDeadline, TimedOutError};
pub use stop_source::{StopSource, StopToken};

/// A prelude for `stop-token`.
pub mod prelude {
    pub use crate::future::FutureExt as _;
    pub use crate::stream::StreamExt as _;
}
