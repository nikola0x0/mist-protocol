use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

pub fn generate_crc_response(crc_token: &str, consumer_secret: &str) -> Result<String, String> {
    let mut mac = HmacSha256::new_from_slice(consumer_secret.as_bytes())
        .map_err(|e| format!("Invalid key: {}", e))?;

    mac.update(crc_token.as_bytes());
    let result = mac.finalize();
    let signature = result.into_bytes();

    Ok(format!("sha256={}", BASE64.encode(signature)))
}
