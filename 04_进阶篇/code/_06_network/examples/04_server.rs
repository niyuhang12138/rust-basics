use anyhow::Result;
use bytes::Bytes;
use futures::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio_util::codec::{Framed, LengthDelimitedCodec};

#[tokio::main]
async fn main() -> Result<()> {
    let listener = TcpListener::bind("127.0.0.1:9527").await?;

    loop {
        let (stream, addr) = listener.accept().await?;
        println!("accepted: {addr}");
        // LengthDelimitedCodec默认4字节长度
        let mut stream = Framed::new(stream, LengthDelimitedCodec::new());
        tokio::spawn(async move {
            while let Some(Ok(data)) = stream.next().await {
                println!("Got: {:?}", String::from_utf8_lossy(&data));
                // 发送消息也需要发送主题信息, 不需要提供长度
                // Framed/LengthDelimitedCodec会自动计算并添加
                stream.send(Bytes::from("goodbye world!")).await.unwrap();
            }
        });
    }
}
