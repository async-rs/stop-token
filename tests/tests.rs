use std::time::Duration;

use async_std::{prelude::*, task, channel};

use stop_token::StopSource;

#[test]
fn smoke() {
    task::block_on(async {
        let (sender, receiver) = channel::bounded::<i32>(10);
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
        }});
        assert!(sender.send(1).await.is_ok());
        assert!(sender.send(2).await.is_ok());
        assert!(sender.send(3).await.is_ok());

        task::sleep(Duration::from_millis(250)).await;
        drop(stop_source);
        task::sleep(Duration::from_millis(250)).await;

        assert!(sender.send(4).await.is_ok());
        assert!(sender.send(5).await.is_ok());
        assert!(sender.send(6).await.is_ok());
        assert_eq!(task.await, vec![1, 2, 3]);
    })
}
