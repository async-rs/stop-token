use std::time::Duration;

use async_std::{prelude::*, sync::channel, task};

use stop_token::{Error, StopSource, StopToken, WithStopTokenExt as _};

#[test]
fn smoke() {
    task::block_on(async {
        let (sender, receiver) = channel::<i32>(10);
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
        sender.send(1).await;
        sender.send(2).await;
        sender.send(3).await;

        task::sleep(Duration::from_millis(250)).await;
        drop(stop_source);
        task::sleep(Duration::from_millis(250)).await;

        sender.send(4).await;
        sender.send(5).await;
        sender.send(6).await;
        assert_eq!(task.await, vec![1, 2, 3]);
    })
}

#[test]
fn extension_methods() {
    async fn long_running(stop_token: StopToken) -> Result<(), Error> {
        loop {
            task::sleep(Duration::from_secs(10))
                .with_stop_token(&stop_token)
                .await?;
        }
    }

    task::block_on(async {
        let stop_source = StopSource::new();
        let stop_token = stop_source.stop_token();

        task::spawn(async {
            task::sleep(Duration::from_millis(250)).await;
            drop(stop_source);
        });

        if let Ok(_) = long_running(stop_token).await {
            panic!("expected to have been stopped");
        }
    })
}
