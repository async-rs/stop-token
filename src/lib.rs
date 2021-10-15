//! Cooperative cancellation for async Rust.
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
//! You can use this crate to create a deadline received through a
//! [`StopToken`]. You can think of a `StopSource` + `StopToken` as a
//! single-producer, multi-consumer channel that receives a single message to
//! "stop" when the producer is dropped:
//!
//! ```
//! use async_std::prelude::*;
//! use async_std::{stream, task};
//!
//! use stop_token::prelude::*;
//! use stop_token::StopSource;
//!
//! use std::time::Duration;
//!
//! #[async_std::main]
//! async fn main() {
//!     // Create a stop source and generate a token.
//!     let src = StopSource::new();
//!     let stop = src.token();
//!
//!     // When stop source is dropped, the loop will stop.
//!     // Move the source to a task, and drop it after 100 millis.
//!     task::spawn(async move {
//!         task::sleep(Duration::from_millis(100)).await;
//!         drop(src);
//!     });
//!
//!     // Create a stream that generates numbers until
//!     // it receives a signal it needs to stop.
//!     let mut work = stream::repeat(12u8).until(stop);
//!
//!     // Loop over each item in the stream.
//!     while let Some(Ok(ev)) = work.next().await {
//!         println!("{}", ev);
//!     }
//! }
//! ```
//!
//! Or `Duration` or `Instant` to create a [`time`]-based deadline:
//!
//! ```
//! # #![allow(dead_code)]
//! use async_std::prelude::*;
//! use async_std::stream;
//!
//! use stop_token::prelude::*;
//!
//! use std::time::Duration;
//!
//! # #[cfg(feature = "tokio")]
//! # fn main() {}
//! # #[cfg(not(feature = "tokio"))]
//! #[async_std::main]
//! async fn main() {
//!     // Create a stream that generates numbers for 100 millis.
//!     let stop = Duration::from_millis(100);
//!     let mut work = stream::repeat(12u8).until(stop);
//!
//!     // Loop over each item in the stream.
//!     while let Some(Ok(ev)) = work.next().await {
//!         println!("{}", ev);
//!     }
//! }
//! ```
//!
//! # Features
//!
//! The `time` submodule is empty when no features are enabled. To implement `Into<Deadline>`
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

#[cfg(any(feature = "async-io", feature = "docs"))]
pub mod async_io;
#[cfg(feature = "async_std")]
pub mod async_std;
#[cfg(feature = "tokio")]
pub mod tokio;

mod deadline;
mod stop_source;

pub use deadline::{Deadline, TimedOutError};
pub use stop_source::{StopSource, StopToken};

/// A prelude for `stop-token`.
pub mod prelude {
    pub use crate::future::FutureExt as _;
    pub use crate::stream::StreamExt as _;
}
