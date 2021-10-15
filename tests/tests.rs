use std::time::Duration;

use async_std::prelude::*;
use stop_token::prelude::*;

use async_channel::bounded;
use async_std::task;

use stop_token::StopSource;

#[test]
fn smoke() {
    task::block_on(async {
        let (sender, receiver) = bounded::<i32>(10);
        let source = StopSource::new();
        let task = task::spawn({
            let token = source.token();
            let receiver = receiver.clone();
            async move {
                let mut xs = Vec::new();
                let mut stream = receiver.until(token);
                while let Some(Ok(x)) = stream.next().await {
                    xs.push(x)
                }
                xs
            }
        });
        sender.send(1).await.unwrap();
        sender.send(2).await.unwrap();
        sender.send(3).await.unwrap();

        task::sleep(Duration::from_millis(250)).await;
        drop(source);
        task::sleep(Duration::from_millis(250)).await;

        sender.send(4).await.unwrap();
        sender.send(5).await.unwrap();
        sender.send(6).await.unwrap();
        assert_eq!(task.await, vec![1, 2, 3]);
    })
}

#[cfg(feature = "async-io")]
#[test]
fn async_io_time() {
    task::block_on(async {
        let (sender, receiver) = bounded::<i32>(10);
        let task = task::spawn({
            let receiver = receiver.clone();
            async move {
                let mut xs = Vec::new();
                let mut stream = receiver.until(Duration::from_millis(200));
                while let Some(Ok(x)) = stream.next().await {
                    xs.push(x)
                }
                xs
            }
        });
        sender.send(1).await.unwrap();
        sender.send(2).await.unwrap();
        sender.send(3).await.unwrap();

        task::sleep(Duration::from_millis(250)).await;

        sender.send(4).await.unwrap();
        sender.send(5).await.unwrap();
        sender.send(6).await.unwrap();
        assert_eq!(task.await, vec![1, 2, 3]);
    })
}

#[cfg(feature = "tokio")]
#[tokio::test]
async fn tokio_time() {
    use tokio::time::Instant;
    let (sender, receiver) = bounded::<i32>(10);
    let task = tokio::task::spawn({
        let receiver = receiver.clone();
        async move {
            let mut xs = Vec::new();
            let mut stream = receiver.until(Instant::now() + Duration::from_millis(200));
            while let Some(Ok(x)) = stream.next().await {
                xs.push(x)
            }
            xs
        }
    });
    sender.send(1).await.unwrap();
    sender.send(2).await.unwrap();
    sender.send(3).await.unwrap();

    task::sleep(Duration::from_millis(250)).await;

    sender.send(4).await.unwrap();
    sender.send(5).await.unwrap();
    sender.send(6).await.unwrap();
    assert_eq!(task.await.unwrap(), vec![1, 2, 3]);
}
