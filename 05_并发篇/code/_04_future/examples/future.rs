use anyhow::Result;
use futures::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio_util::codec::{Framed, LinesCodec};

#[tokio::main]
async fn main() -> Result<()> {
    let addr = "0.0.0.0:8000";
    let listener = TcpListener::bind(addr).await?;
    println!("listener to :{addr}");
    loop {
        let (stream, addr) = listener.accept().await?;
        println!("Accepted :{addr}");
        tokio::spawn(async move {
            // 使用LinesCodec把TCP数据切成一行行字符串处理
            let frame = Framed::new(stream, LinesCodec::new());
            // split成writer和reader
            let (mut w, mut r) = frame.split();

            for line in r.next().await {
                // 每读到一行就价格前缀返回
                w.send(format!("I got: {}", line?)).await?;
            }

            Ok::<_, anyhow::Error>(())
        });
    }
}
