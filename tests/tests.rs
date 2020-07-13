use std::time::Duration;

use async_std::{prelude::*, sync::channel, task};

use stop_token::StopSource;

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
fn multiple_tokens() {
    task::block_on(async {
        let stop_source = StopSource::new();

        let (sender_a, receiver_a) = channel::<i32>(10);
        let task_a = task::spawn({
            let stop_token = stop_source.stop_token();
            let receiver = receiver_a.clone();
            async move {
                let mut xs = Vec::new();
                let mut stream = stop_token.stop_stream(receiver);
                while let Some(x) = stream.next().await {
                    xs.push(x)
                }
                xs
            }
        });

        let (sender_b, receiver_b) = channel::<i32>(10);
        let task_b = task::spawn({
            let stop_token = stop_source.stop_token();
            let receiver = receiver_b.clone();
            async move {
                let mut xs = Vec::new();
                let mut stream = stop_token.stop_stream(receiver);
                while let Some(x) = stream.next().await {
                    xs.push(x)
                }
                xs
            }
        });

        sender_a.send(1).await;
        sender_a.send(2).await;
        sender_a.send(3).await;

        sender_b.send(101).await;
        sender_b.send(102).await;
        sender_b.send(103).await;

        task::sleep(Duration::from_millis(250)).await;

        drop(stop_source);

        task::sleep(Duration::from_millis(250)).await;

        sender_a.send(4).await;
        sender_a.send(5).await;
        sender_a.send(6).await;

        sender_b.send(104).await;
        sender_b.send(105).await;
        sender_b.send(106).await;

        assert_eq!(task_a.await, vec![1, 2, 3]);
        assert_eq!(task_b.await, vec![101, 102, 103]);
    })
}

#[test]
fn contest_cached_listener() {
    task::block_on(async {
        let stop_source = StopSource::new();

        let (sender_a, receiver_a) = channel::<i32>(10);
        let recv_task_a = task::spawn({
            let stop_token = stop_source.stop_token();
            let receiver = receiver_a.clone();
            async move {
                let mut xs = Vec::new();
                let mut stream = stop_token.stop_stream(receiver);
                while let Some(x) = stream.next().await {
                    xs.push(x)
                }
                xs
            }
        });

        let (sender_b, receiver_b) = channel::<i32>(10);
        let recv_task_b = task::spawn({
            let stop_token = stop_source.stop_token();
            let receiver = receiver_b.clone();
            async move {
                let mut xs = Vec::new();
                let mut stream = stop_token.stop_stream(receiver);
                while let Some(x) = stream.next().await {
                    xs.push(x)
                }
                xs
            }
        });

        let _send_task_a = task::spawn({
            let sender = sender_a.clone();
            async move {
                for i in 0.. {
                    sender.send(i).await;
                }
            }
        });

        let _send_task_b = task::spawn({
            let sender = sender_b.clone();
            async move {
                for i in 0.. {
                    sender.send(i).await;
                }
            }
        });

        task::sleep(Duration::from_millis(250)).await;

        drop(stop_source);

        task::sleep(Duration::from_millis(250)).await;

        assert!(!recv_task_a.await.is_empty());
        assert!(!recv_task_b.await.is_empty())
    })
}
