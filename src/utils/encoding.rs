use serde::{Deserialize, Serialize};
use std::error::Error;

pub fn encode_to_base64<T: Serialize>(data: &T) -> Result<String, Box<dyn Error + Send + Sync>> {
    let json_str = serde_json::to_string(data)?;
    let engine = base64::engine::general_purpose::STANDARD;
    Ok(base64::Engine::encode(&engine, json_str.as_bytes()))
}

pub fn decode_from_base64<T: for<'de> Deserialize<'de>>(
    encoded: &str,
) -> Result<T, Box<dyn Error + Send + Sync>> {
    let engine = base64::engine::general_purpose::STANDARD;
    let bytes = base64::Engine::decode(&engine, encoded)?;
    let json_str = String::from_utf8(bytes)?;
    let data = serde_json::from_str(&json_str)?;
    Ok(data)
}

pub fn encode_string_to_base64(data: &str) -> String {
    let engine = base64::engine::general_purpose::STANDARD;
    base64::Engine::encode(&engine, data.as_bytes())
}

pub fn decode_base64_to_string(encoded: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
    let engine = base64::engine::general_purpose::STANDARD;
    let bytes = base64::Engine::decode(&engine, encoded)?;
    let decoded = String::from_utf8(bytes)?;
    Ok(decoded)
}
