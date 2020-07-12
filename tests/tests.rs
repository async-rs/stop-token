use std::time::Duration;

use async_channel::bounded;
use async_std::{prelude::*, task};

use stop_token::StopSource;

#[test]
fn smoke() {
    task::block_on(async {
        let (sender, receiver) = bounded::<i32>(10);
        let stop_source = StopSource::new();
        let task = task::spawn({
            let stop_token = stop_source.stop_token();
            let receiver = receiver.clone();
            async move {
                let mut xs = Vec::new();
                let mut stream = stop_token.stop_stream(receiver);
                while let Some(x) = stream.next().await {
                    xs.push(x)
                }
                xs
            }
        });
        sender.send(1).await.unwrap();
        sender.send(2).await.unwrap();
        sender.send(3).await.unwrap();

        task::sleep(Duration::from_millis(250)).await;
        drop(stop_source);
        task::sleep(Duration::from_millis(250)).await;

        sender.send(4).await.unwrap();
        sender.send(5).await.unwrap();
        sender.send(6).await.unwrap();

        assert_eq!(task.await, vec![1, 2, 3]);
    })
}
