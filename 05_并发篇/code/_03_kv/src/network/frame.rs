use std::io::{Read, Write};

use bytes::{Buf, BufMut, BytesMut};
use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use prost::Message;
use tokio::io::{AsyncRead, AsyncReadExt};
use tracing::debug;

use crate::{CommandRequest, CommandResponse, KvError};

/// 长度整个占用4个字节
pub const LEN_LEN: usize = 4;

/// 长度占31bit, 所以最大的frame是2G
pub const MAX_FRAME: usize = 2 * 1024 * 1024 * 1024;

/// 如果payload超过1436字节 就压缩
pub const COMPRESSION_LIMIT: usize = 1436;

/// 代表压缩的bit(整个4字节的最高位)
pub const COMPRESSION_BIT: usize = 1 << 31;

pub trait FrameCoder
where
    Self: Message + Sized + Default,
{
    /// 把一个Message encode成一个frame
    fn encode_frame(&self, buf: &mut BytesMut) -> Result<(), KvError> {
        let size = self.encoded_len();

        if size > MAX_FRAME {
            return Err(KvError::FrameError);
        }

        // 我们先写入长度, 如果需要压缩, 在重写压缩后的长度
        buf.put_u32(size as _);

        if size > COMPRESSION_LIMIT {
            let mut buf1 = Vec::with_capacity(size);
            self.encode(&mut buf1)?;

            // BytesMut支持逻辑上的split(之后还能unsplit)
            // 所以我们先把长度中4字节长度, 清除
            let payload = buf.split_off(LEN_LEN);
            buf.clear();

            // 处理gzip压缩, 具体可以参考flate2文档
            let mut encoder = GzEncoder::new(payload.writer(), Compression::default());
            encoder.write_all(&buf1[..])?;

            // 压缩完成后, 从gzip encode中吧BytesMut在拿回来
            let payload = encoder.finish()?.into_inner();
            debug!("Encode a frame: size: {}({})", size, payload.len());

            // 写入压缩后的长度
            buf.put_u32((payload.len() | COMPRESSION_BIT) as _);

            // 把BytesMut在合并回来
            buf.unsplit(payload);

            Ok(())
        } else {
            self.encode(buf)?;
            Ok(())
        }
    }

    /// 把一个完成的frame decode成一个Message
    fn decode_frame(buf: &mut BytesMut) -> Result<Self, KvError> {
        // 先取四个字节
        let header = buf.get_u32() as usize;
        let (len, compressed) = decode_header(header);
        debug!("Got a frame: msg len {}, compressed {}", len, compressed);

        if compressed {
            // 解压缩
            let mut decoder = GzDecoder::new(&buf[..len]);
            let mut buf1 = Vec::with_capacity(len * 2);
            decoder.read_to_end(&mut buf1)?;
            buf.advance(len);

            // decode成相应详细
            Ok(Self::decode(&buf1[..buf1.len()])?)
        } else {
            let msg = Self::decode(&buf[..len])?;
            buf.advance(len);
            Ok(msg)
        }
    }
}

impl FrameCoder for CommandRequest {}

impl FrameCoder for CommandResponse {}

pub fn decode_header(header: usize) -> (usize, bool) {
    let len = header & !COMPRESSION_BIT;
    let compressed = header & COMPRESSION_BIT == COMPRESSION_BIT;
    (len, compressed)
}

/// 从stream中读取一个完整的stream
pub async fn read_frame<S>(stream: &mut S, buf: &mut BytesMut) -> Result<(), KvError>
where
    S: AsyncRead + Unpin + Send,
{
    let header = stream.read_u32().await? as usize;
    let (len, _compressed) = decode_header(header);
    // 如果美欧那么大的内存, 就分配至少一个frame的内存, 保证它可用
    buf.reserve(LEN_LEN + len);
    buf.put_u32(header as _);
    // advance_mut是unsafe的原因是, 从当前我盒子pos到pos + len
    // 这段内存目前没有初始化, 我们就是为了reserve这段内存, 然后从stream里读取, 读取完, 它就是初始化的, 所以我们这么用是安全的
    unsafe { buf.advance_mut(len) }

    stream.read_exact(&mut buf[LEN_LEN..]).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Value;

    struct DummyStream {
        buf: BytesMut,
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

    #[test]
    fn command_request_encode_decode_should_work() {
        let mut buf = BytesMut::new();

        let cmd = CommandRequest::new_hdel("t1", "k1");

        cmd.encode_frame(&mut buf).unwrap();

        // 最高位没设置
        assert_eq!(is_compressed(&buf), false);

        let cmd1 = CommandRequest::decode_frame(&mut buf).unwrap();
        assert_eq!(cmd, cmd1);
    }

    #[test]
    fn command_response_encode_decode_should_work() {
        let mut buf = BytesMut::new();

        let values: Vec<Value> = vec![1.into(), "hello".into(), b"data".into()];

        let res: CommandResponse = values.into();

        res.encode_frame(&mut buf).unwrap();

        // 最高位长度没设置
        assert!(!is_compressed(&buf));

        let res1 = CommandResponse::decode_frame(&mut buf).unwrap();

        assert_eq!(res, res1);
    }

    #[tokio::test]
    async fn read_frame_should_work() {
        let mut buf = BytesMut::new();
        let cmd = CommandRequest::new_hdel("t1", "k1");
        cmd.encode_frame(&mut buf).unwrap();
        let mut stream = DummyStream { buf };

        let mut data = BytesMut::new();
        read_frame(&mut stream, &mut data).await.unwrap();

        let cmd1 = CommandRequest::decode_frame(&mut data).unwrap();

        assert_eq!(cmd, cmd1);
    }

    fn is_compressed(data: &[u8]) -> bool {
        if let [v] = data[..1] {
            v >> 7 == 1
        } else {
            false
        }
    }
}
