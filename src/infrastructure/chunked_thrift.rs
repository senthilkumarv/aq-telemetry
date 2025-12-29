// Chunked Thrift streaming utilities
use axum::body::Body;
use axum::http::{header, Response, StatusCode};
use axum::response::IntoResponse;
use bytes::{BufMut, Bytes, BytesMut};
use futures::stream::Stream;
use futures::StreamExt;
use telemetry_thrift::StreamMessage;
use thrift::protocol::{TBinaryOutputProtocol, TOutputProtocol, TSerializable};
use async_compression::tokio::bufread::BrotliEncoder;
use tokio::io::AsyncReadExt;

/// Create a chunked Thrift streaming response
pub async fn chunked_thrift_stream<S>(
    stream: S,
    compress: bool,
) -> Result<Response<Body>, StatusCode>
where
    S: Stream<Item = StreamMessage> + Send + 'static,
{
    let byte_stream = stream.then(move |msg| async move { serialize_chunk(msg, compress).await });

    let body = Body::from_stream(byte_stream);

    // NOTE: We do NOT set Content-Encoding header for chunked streaming
    // because we compress individual chunks, not the entire HTTP response.
    // Setting Content-Encoding would cause URLSession to try to decompress
    // the HTTP stream, which breaks our custom chunk protocol.
    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/x-thrift")
        .header(header::TRANSFER_ENCODING, "chunked");

    response
        .body(body)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

/// Serialize a single StreamMessage to a chunk
async fn serialize_chunk(msg: StreamMessage, compress: bool) -> Result<Bytes, std::io::Error> {
    // 1. Serialize to Thrift binary
    let mut buffer: Vec<u8> = Vec::new();
    {
        let mut protocol = TBinaryOutputProtocol::new(&mut buffer, true);
        msg.write_to_out_protocol(&mut protocol)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        protocol
            .flush()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    }

    // 2. Optionally compress
    let payload = if compress {
        let cursor = std::io::Cursor::new(buffer);
        let mut encoder = BrotliEncoder::new(cursor);
        let mut compressed = Vec::new();
        encoder.read_to_end(&mut compressed).await?;
        compressed
    } else {
        buffer
    };

    // 3. Prepend length (4 bytes, big-endian)
    let length = payload.len() as u32;
    let mut chunk = BytesMut::with_capacity(4 + payload.len());
    chunk.put_u32(length);
    chunk.put_slice(&payload);

    Ok(chunk.freeze())
}

/// Helper to create a streaming response from a receiver
pub async fn stream_from_receiver(
    mut rx: tokio::sync::mpsc::Receiver<StreamMessage>,
    compress: bool,
) -> impl IntoResponse {
    let stream = async_stream::stream! {
        while let Some(msg) = rx.recv().await {
            yield msg;
        }
    };

    match chunked_thrift_stream(stream, compress).await {
        Ok(response) => response,
        Err(status) => status.into_response(),
    }
}

