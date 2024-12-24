use anyhow::Result;
use futures::{ready, FutureExt, Sink, Stream};
use std::{marker::PhantomData, pin::Pin, task::Poll};
use tokio::io::{AsyncRead, AsyncWrite};

use bytes::BytesMut;

use crate::{read_frame, KvError};

use super::FrameCoder;

/// 处理KV Server prost frame的stream
pub struct ProstStream<S, In, Out> {
    // inner stream
    stream: S,
    // write cache
    wbuf: BytesMut,
    written: usize,
    // read cache
    rbuf: BytesMut,

    // 类型占位符
    _in: PhantomData<In>,
    _out: PhantomData<Out>,
}

impl<S, In, Out> ProstStream<S, In, Out>
where
    S: AsyncRead + AsyncWrite + Unpin + Send,
{
    pub fn new(stream: S) -> Self {
        Self {
            stream,
            written: 0,
            wbuf: BytesMut::new(),
            rbuf: BytesMut::new(),
            _in: PhantomData::default(),
            _out: PhantomData::default(),
        }
    }
}

// 一般来说, 我们的Stream是Unpin, 最好实现一下
impl<S, Req, Res> Unpin for ProstStream<S, Req, Res> where S: Unpin {}

impl<S, In, Out> Stream for ProstStream<S, In, Out>
where
    S: AsyncRead + AsyncWrite + Unpin + Send,
    In: Unpin + Send + FrameCoder,
    Out: Unpin + Send,
{
    type Item = Result<In, KvError>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Result<In, KvError>>> {
        // 上一次调用结束后rbuf应该Wie空
        assert!(self.rbuf.len() == 0);

        // 从rbuf中分离出rset(摆脱对self的引用)
        let mut rest = self.rbuf.split_off(0);

        // 使用read_frame来获取数据
        let fut = read_frame(&mut self.stream, &mut rest);
        ready!(Box::pin(fut).poll_unpin(cx))?;

        // 拿到一个frame的数据, 把buff合并回去
        self.rbuf.unsplit(rest);

        // 调用decode_frame获取解包之后的数据
        Poll::Ready(Some(In::decode_frame(&mut self.rbuf)))
    }
}

/// 当调用send的时候, 会把Out发出去
impl<S, In, Out> Sink<Out> for ProstStream<S, In, Out>
where
    S: AsyncRead + AsyncWrite + Unpin,
    In: Unpin + Send,
    Out: Unpin + Send + FrameCoder,
{
    type Error = KvError;

    fn poll_ready(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), KvError>> {
        Poll::Ready(Ok(()))
    }

    fn start_send(self: std::pin::Pin<&mut Self>, item: Out) -> Result<(), KvError> {
        let this = self.get_mut();
        item.encode_frame(&mut this.wbuf)?;
        Ok(())
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), KvError>> {
        let this = self.get_mut();

        // 循环写入stream中
        while this.written != this.wbuf.len() {
            let n = ready!(Pin::new(&mut this.stream).poll_write(cx, &this.wbuf[this.written..]))?;
            this.written += n;
        }

        // 清除wbuf
        this.wbuf.clear();
        this.written = 0;

        // 调用stream的pull_flush确保写入
        ready!(Pin::new(&mut this.stream).poll_flush(cx)?);
        Poll::Ready(Ok(()))
    }

    fn poll_close(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), KvError>> {
        // 调用stream的pull_flush确保写入
        ready!(self.as_mut().poll_flush(cx))?;

        // 调用stream的pull_close确保stream关闭
        ready!(Pin::new(&mut self.stream).poll_shutdown(cx))?;
        Poll::Ready(Ok(()))
    }
}

#[cfg(test)]
mod tests {
    use super::super::utils::DummyStream;
    use super::*;
    use crate::{CommandRequest, CommandResponse};
    use anyhow::Result;
    use futures::{Sink, SinkExt, Stream, StreamExt};

    #[tokio::test]
    async fn prost_stream_should_work() -> Result<()> {
        let buf = BytesMut::new();
        let stream = DummyStream { buf };
        let mut stream = ProstStream::<_, CommandRequest, CommandResponse>::new(stream);
        let cmd: CommandRequest = CommandRequest::new_hdel("t1", "k1");
        stream.send(cmd.clone()).await?;
        if let Some(Ok(s)) = stream.next().await {
            assert_eq!(s, cmd);
        } else {
            assert!(false)
        }

        Ok(())
    }
}
