use anyhow::Result;
use bytes::BytesMut;
pub use frame::*;
use futures::{SinkExt, StreamExt};
pub use stream_result::StreamResult;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};
use tracing::info;

use crate::{CommandRequest, CommandResponse, KvError, Service};

mod frame;
mod multiplex;
mod stream;
mod stream_result;
mod tls;

pub use multiplex::*;
pub use stream::*;
pub use stream_result::*;
pub use tls::*;

/// 处理服务器端某个accept下来的socket读写
// pub struct ProstServerStream<S> {
//     inner: S,
//     service: Service,
// }

pub struct ProstServerStream<S> {
    inner: ProstStream<S, CommandRequest, CommandResponse>,
    service: Service,
}

// impl<S> ProstServerStream<S>
// where
//     S: AsyncRead + AsyncWrite + Unpin + Send,
// {
//     pub fn new(stream: S, service: Service) -> Self {
//         Self {
//             inner: stream,
//             service,
//         }
//     }

//     pub async fn process(mut self) -> Result<(), KvError> {
//         while let Ok(cmd) = self.recv().await {
//             info!("Got a new command: {cmd:?}");
//             let res = self.service.execute(cmd);
//             self.send(res).await?;
//         }
//         Ok(())
//     }

//     async fn send(&mut self, msg: CommandResponse) -> Result<(), KvError> {
//         let mut buf = BytesMut::new();
//         msg.encode_frame(&mut buf)?;
//         let encoded = buf.freeze();
//         self.inner.write_all(&encoded[..]).await?;
//         Ok(())
//     }

//     async fn recv(&mut self) -> Result<CommandRequest, KvError> {
//         let mut buf = BytesMut::new();
//         let stream = &mut self.inner;
//         read_frame(stream, &mut buf).await;
//         CommandRequest::decode_frame(&mut buf)
//     }
// }

impl<S> ProstServerStream<S>
where
    S: AsyncRead + AsyncWrite + Unpin + Send,
{
    pub fn new(stream: S, service: Service) -> Self {
        Self {
            inner: ProstStream::new(stream),
            service,
        }
    }

    pub async fn process(mut self) -> Result<(), KvError> {
        let stream = &mut self.inner;
        while let Some(Ok(cmd)) = stream.next().await {
            info!("Got a new command: {cmd:?}");
            let mut res = self.service.execute(cmd);
            while let Some(data) = res.next().await {
                // 目前 data 是 Arc<CommandResponse>，
                // 所以我们 send 最好用 &CommandResponse
                stream.send(&data).await.unwrap();
            }
        }
        Ok(())
    }
}

/// 处理客户端socket读写
// pub struct ProstClientStream<S> {
//     inner: S,
// }

pub struct ProstClientStream<S> {
    inner: ProstStream<S, CommandResponse, CommandRequest>,
}

// impl<S> ProstClientStream<S>
// where
//     S: AsyncRead + AsyncWrite + Unpin + Send,
// {
//     pub fn new(stream: S) -> Self {
//         Self { inner: stream }
//     }

//     pub async fn execute(&mut self, cmd: CommandRequest) -> Result<CommandResponse, KvError> {
//         self.send(cmd).await?;
//         Ok(self.recv().await?)
//     }

//     async fn send(&mut self, msg: CommandRequest) -> Result<(), KvError> {
//         let mut buf = BytesMut::new();
//         msg.encode_frame(&mut buf)?;
//         let encoded = buf.freeze();
//         self.inner.write_all(&encoded[..]).await?;
//         Ok(())
//     }

//     async fn recv(&mut self) -> Result<CommandResponse, KvError> {
//         let mut buf = BytesMut::new();
//         let stream = &mut self.inner;
//         read_frame(stream, &mut buf).await?;
//         CommandResponse::decode_frame(&mut buf)
//     }
// }

impl<S> ProstClientStream<S>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    pub fn new(stream: S) -> Self {
        Self {
            inner: ProstStream::new(stream),
        }
    }

    pub async fn execute_unary(
        &mut self,
        cmd: &CommandRequest,
    ) -> Result<CommandResponse, KvError> {
        let stream = &mut self.inner;
        stream.send(cmd).await?;

        match stream.next().await {
            Some(v) => v,
            None => Err(KvError::Internal("Didn't get any response".into())),
        }
    }

    pub async fn execute_streaming(self, cmd: &CommandRequest) -> Result<StreamResult, KvError> {
        let mut stream = self.inner;

        stream.send(cmd).await?;
        stream.close().await?;

        StreamResult::new(stream).await
    }
}

#[cfg(test)]
pub mod utils {
    use std::task::Poll;

    use bytes::BufMut;

    use super::*;

    pub struct DummyStream {
        pub buf: BytesMut,
    }

    impl AsyncRead for DummyStream {
        fn poll_read(
            self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
            buf: &mut tokio::io::ReadBuf<'_>,
        ) -> std::task::Poll<std::io::Result<()>> {
            // 看看ReadBuf需要多大的数据
            let len = buf.capacity();

            // split 出这么大的数据
            let data = self.get_mut().buf.split_to(len);

            // 拷贝给ReadBuf
            buf.put_slice(&data);

            // 完工
            std::task::Poll::Ready(Ok(()))
        }
    }

    impl AsyncWrite for DummyStream {
        fn poll_write(
            self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
            buf: &[u8],
        ) -> std::task::Poll<std::result::Result<usize, std::io::Error>> {
            self.get_mut().buf.put_slice(buf);
            Poll::Ready(Ok(buf.len()))
        }

        fn poll_flush(
            self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
        ) -> Poll<std::result::Result<(), std::io::Error>> {
            Poll::Ready(Ok(()))
        }

        fn poll_shutdown(
            self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
        ) -> Poll<std::result::Result<(), std::io::Error>> {
            Poll::Ready(Ok(()))
        }
    }
}

#[cfg(test)]
mod tests {

    use std::{net::SocketAddr, vec};

    use bytes::Bytes;
    use tokio::net::{TcpListener, TcpStream};

    use crate::{assert_res_ok, MemTable, ServiceInner, Value};

    use super::*;

    #[tokio::test]
    async fn client_server_basic_communication_should_work() -> anyhow::Result<()> {
        let addr = start_server().await?;

        let stream = TcpStream::connect(addr).await?;
        let mut client = ProstClientStream::new(stream);

        // 发送HSET等待回应
        let cmd = CommandRequest::new_hset("t1", "k1", "v1".into());
        let res = client.execute_unary(&cmd).await.unwrap();

        // 第一个HSET服务器应该返回None
        assert_res_ok(&res, &[Value::default()], &[]);

        // 再发一个HGET
        let cmd = CommandRequest::new_hget("t1", "k1");
        let res = client.execute_unary(&cmd).await?;

        // 服务器应该返回上一次的结果
        assert_res_ok(&res, &["v1".into()], &[]);

        Ok(())
    }

    #[tokio::test]
    async fn client_server_compression_should_work() -> anyhow::Result<()> {
        let addr = start_server().await?;

        let stream = TcpStream::connect(addr).await?;
        let mut client = ProstClientStream::new(stream);

        let v: Value = Bytes::from(vec![0u8; 16384]).into();
        let cmd = CommandRequest::new_hset("t2", "k2", v.clone());
        let res = client.execute_unary(&cmd).await?;

        assert_res_ok(&res, &[Value::default()], &[]);

        let cmd = CommandRequest::new_hget("t2", "k2");
        let res = client.execute_unary(&cmd).await?;

        assert_res_ok(&res, &[v], &[]);

        Ok(())
    }

    async fn start_server() -> Result<SocketAddr> {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            loop {
                let (stream, _) = listener.accept().await.unwrap();
                let service: Service = ServiceInner::new(MemTable::new()).into();
                let server = ProstServerStream::new(stream, service);
                tokio::spawn(server.process());
            }
        });
        Ok(addr)
    }
}
