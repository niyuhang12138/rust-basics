use anyhow::Result;
use async_prost::AsyncProstStream;
use futures::prelude::*;
use kv::{CommandRequest, CommandResponse};
use tokio::net::TcpListener;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let addr = "127.0.0.1:9527";
    let listener = TcpListener::bind(addr).await?;
    info!("Start listener on {addr}");

    loop {
        let (stream, addr) = listener.accept().await?;
        let mut stream =
            AsyncProstStream::<_, CommandRequest, CommandResponse, _>::from(stream).for_async();
        while let Some(Ok(msg)) = stream.next().await {
            info!("Got a new command: {:?}", msg);
            let mut resp = CommandResponse::default();
            resp.status = 404;
            resp.message = "Not Found".to_string();
            stream.send(resp).await.unwrap();
        }
        info!("Client {:?} disconnected", addr);
    }
}
