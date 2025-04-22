use base64::{engine::general_purpose::STANDARD, Engine as _};
use std::error::Error;

pub fn encode_string_to_base64(data: &str) -> String {
    STANDARD.encode(data.as_bytes())
}

pub fn decode_base64_to_string(encoded: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
    let bytes = STANDARD.decode(encoded)?;
    String::from_utf8(bytes).map_err(|e| e.into())
}
