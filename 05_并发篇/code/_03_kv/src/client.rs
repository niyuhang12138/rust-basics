use anyhow::Result;
use kv::{CommandRequest, ProstClientStream};
use tokio::net::TcpStream;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let addr = "127.0.0.1:9527";

    let stream = TcpStream::connect(addr).await?;

    let mut client = ProstClientStream::new(stream);

    // HSET
    let cmd = CommandRequest::new_hset("table1", "hello", "world".to_string().into());

    // 发送命令
    let data = client.execute(cmd).await?;

    info!("Got response {data:?}");

    Ok(())
}
