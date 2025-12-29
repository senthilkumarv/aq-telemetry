// HTTP response utilities for Thrift+Brotli encoding
use axum::{
    body::Body,
    http::{header, HeaderValue, Response, StatusCode},
};
use thrift::protocol::{TBinaryOutputProtocol, TListIdentifier, TOutputProtocol, TSerializable, TType};
use async_compression::tokio::bufread::BrotliEncoder;
use tokio::io::AsyncReadExt;

/// Serialize a Thrift struct to binary format and compress with Brotli
pub async fn thrift_brotli_response<T: TSerializable>(
    data: T,
) -> Result<Response<Body>, StatusCode> {
    // Serialize to Thrift binary format using a Vec as transport
    let mut buffer: Vec<u8> = Vec::new();
    {
        let mut protocol = TBinaryOutputProtocol::new(&mut buffer, true);
        data.write_to_out_protocol(&mut protocol)
            .map_err(|e| {
                eprintln!("Thrift serialization error: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
        protocol.flush().map_err(|e| {
            eprintln!("Thrift flush error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    }

    // Get the serialized bytes
    let thrift_bytes = buffer;

    // Compress with Brotli
    let cursor = std::io::Cursor::new(thrift_bytes);
    let mut encoder = BrotliEncoder::new(cursor);
    let mut compressed = Vec::new();
    encoder.read_to_end(&mut compressed).await.map_err(|e| {
        eprintln!("Brotli compression error: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Build response
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/x-thrift")
        .header(header::CONTENT_ENCODING, "br")
        .header(
            header::CONTENT_LENGTH,
            HeaderValue::from_str(&compressed.len().to_string()).unwrap(),
        )
        .body(Body::from(compressed))
        .map_err(|e| {
            eprintln!("Response build error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })
}

/// Helper to serialize a list of Thrift structs with optional compression
pub async fn thrift_list_response<T: TSerializable>(
    items: Vec<T>,
    compress: bool,
) -> Result<Response<Body>, StatusCode> {
    // For lists, we need to wrap them in a proper Thrift structure
    let mut buffer: Vec<u8> = Vec::new();
    {
        let mut protocol = TBinaryOutputProtocol::new(&mut buffer, true);
        
        // Write list header
        protocol
            .write_list_begin(&TListIdentifier::new(TType::Struct, items.len() as i32))
            .map_err(|e| {
                eprintln!("Thrift list begin error: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        // Write each item
        for item in items {
            item.write_to_out_protocol(&mut protocol).map_err(|e| {
                eprintln!("Thrift item serialization error: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
        }

        // Write list end
        protocol.write_list_end().map_err(|e| {
            eprintln!("Thrift list end error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        protocol.flush().map_err(|e| {
            eprintln!("Thrift flush error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    }

    // Get the serialized bytes
    let thrift_bytes = buffer;

    // Optionally compress with Brotli
    let (body_bytes, content_encoding) = if compress {
        eprintln!("🔄 Compressing {} bytes with Brotli...", thrift_bytes.len());
        let cursor = std::io::Cursor::new(thrift_bytes.clone());
        let mut encoder = BrotliEncoder::new(cursor);
        let mut compressed = Vec::new();
        encoder.read_to_end(&mut compressed).await.map_err(|e| {
            eprintln!("❌ Brotli compression error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
        eprintln!("✅ Compressed: {} → {} bytes ({:.1}% reduction)",
                  thrift_bytes.len(), compressed.len(),
                  (1.0 - compressed.len() as f64 / thrift_bytes.len() as f64) * 100.0);
        (compressed, Some("br"))
    } else {
        eprintln!("📦 Sending uncompressed: {} bytes", thrift_bytes.len());
        (thrift_bytes, None)
    };

    // Build response
    let mut response_builder = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/x-thrift")
        .header(
            header::CONTENT_LENGTH,
            HeaderValue::from_str(&body_bytes.len().to_string()).unwrap(),
        );

    if let Some(encoding) = content_encoding {
        response_builder = response_builder.header(header::CONTENT_ENCODING, encoding);
    }

    response_builder
        .body(Body::from(body_bytes))
        .map_err(|e| {
            eprintln!("Response build error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })
}

/// Helper to serialize a list of Thrift structs (always compressed - deprecated)
#[deprecated(note = "Use thrift_list_response with compress parameter instead")]
pub async fn thrift_brotli_list_response<T: TSerializable>(
    items: Vec<T>,
) -> Result<Response<Body>, StatusCode> {
    thrift_list_response(items, true).await
}

