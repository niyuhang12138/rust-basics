use std::time::Duration;

use anyhow::Result;
use criterion::{criterion_group, criterion_main, Criterion};
use futures::StreamExt;
use kv::{
    start_client_with_config, start_server_with_config, ClientConfig, CommandRequest, ServerConfig,
    StorageConfig, YamuxCtrl,
};
use rand::seq::SliceRandom;
use tokio::{net::TcpStream, runtime::Builder, time};
use tokio_rustls::client::TlsStream;
use tracing::info;

async fn start_server() -> Result<()> {
    let addr = "127.0.0.1:9999";
    let mut config: ServerConfig = toml::from_str(include_str!("../fixtures/server.conf"))?;
    config.general.addr = addr.into();
    config.storage = StorageConfig::MemTable;

    tokio::spawn(async move {
        start_server_with_config(&config).await.unwrap();
    });

    Ok(())
}

async fn connect() -> Result<YamuxCtrl<TlsStream<TcpStream>>> {
    let addr = "127.0.0.1:9999";
    let mut config: ClientConfig = toml::from_str(include_str!("../fixtures/client.conf"))?;
    config.general.addr = addr.into();

    Ok(start_client_with_config(&config).await?)
}

async fn start_subscribers(topic: &'static str) -> Result<()> {
    let mut ctrl = connect().await?;
    let stream = ctrl.open_stream().await?;
    info!("C(subscriber): stream opened");
    let cmd = CommandRequest::new_subscribe(topic.to_string());
    tokio::spawn(async move {
        let mut stream = stream.execute_streaming(&cmd).await.unwrap();
        while let Some(Ok(data)) = stream.next().await {
            drop(data);
        }
    });

    Ok(())
}

async fn start_publishers(topic: &'static str, values: &'static [&'static str]) -> Result<()> {
    let mut rng = rand::thread_rng();
    let v = values.choose(&mut rng).unwrap();
    let mut ctrl = connect().await.unwrap();
    let mut stream = ctrl.open_stream().await.unwrap();
    info!("C(publisher): stream opened");

    let cmd = CommandRequest::new_publish(topic.to_string(), vec![(*v).into()]);
    stream.execute_unary(&cmd).await.unwrap();

    Ok(())
}

fn pubsub(c: &mut Criterion) {
    tracing_subscriber::fmt::init();
    // 创建Tokio runtime
    let runtime = Builder::new_multi_thread()
        .worker_threads(4)
        .thread_name("pubsub")
        .enable_all()
        .build()
        .unwrap();
    let values = &["Hello", "Nyh", "Goodbye", "World"];
    let topic = "lobby";
    // 运行服务器和100个subscriber, 为测试准备
    runtime.block_on(async {
        eprint!("preparing server and subscribers");
        start_server().await.unwrap();
        time::sleep(Duration::from_millis(50)).await;
        for _ in 0..100 {
            start_subscribers(topic).await.unwrap();
            eprint!(".")
        }
        eprintln!("Done!");
    });

    c.bench_function("publishing", move |b| {
        b.to_async(&runtime)
            .iter(|| async { start_publishers(topic, values).await });
    });
}

criterion_group! {
  name = benches;
  config = Criterion::default().sample_size(10);
  targets = pubsub
}

criterion_main!(benches);
