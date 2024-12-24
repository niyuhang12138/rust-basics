use std::task::Poll;

use anyhow::Result;
use bytes::{BufMut, BytesMut};
use futures::{Sink, SinkExt};
use pin_project::pin_project;
use tokio::{fs::File, io::AsyncWrite};

#[pin_project]
struct FileSink {
    #[pin]
    file: File,
    buf: BytesMut,
}

impl FileSink {
    pub fn new(file: File) -> Self {
        Self {
            file,
            buf: BytesMut::new(),
        }
    }
}

impl Sink<&str> for FileSink {
    type Error = std::io::Error;

    fn poll_ready(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::result::Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn start_send(
        self: std::pin::Pin<&mut Self>,
        item: &str,
    ) -> std::result::Result<(), Self::Error> {
        let this = self.project();
        eprint!("{item}");
        this.buf.put(item.as_bytes());
        Ok(())
    }

    fn poll_flush(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<std::result::Result<(), Self::Error>> {
        // 如果向project多次, 需要先把self reborrow一下
        let this = self.as_mut().project();
        let buf = this.buf.split_to(this.buf.len());
        if buf.is_empty() {
            return Poll::Ready(Ok(()));
        }

        // 写入文件
        if let Err(e) = futures::ready!(this.file.poll_write(cx, &buf[..])) {
            return Poll::Ready(Err(e));
        }

        // 刷新文件
        self.project().file.poll_flush(cx)
    }

    fn poll_close(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<std::result::Result<(), Self::Error>> {
        let this = self.project();
        // 结束写入
        this.file.poll_shutdown(cx)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let file_sink = FileSink::new(File::create("tmp/hello.txt").await?);
    // pin_mut可以吧变量pin住
    futures::pin_mut!(file_sink);
    file_sink.send("hello\\n").await?;
    file_sink.send("world\\n").await?;
    file_sink.send("Nyh\\n").await?;
    Ok(())
}
