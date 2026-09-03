use base64::Engine;
use minisign_verify::{PublicKey, Signature};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SignatureVerificationError {
    InvalidEncoding,
    InvalidSignature,
}

pub fn verify_tauri_signature(
    archive: &[u8],
    encoded_signature: &str,
    encoded_public_key: &str,
) -> Result<(), SignatureVerificationError> {
    let decoder = base64::engine::general_purpose::STANDARD;
    let public_key_text = decoder
        .decode(encoded_public_key.trim())
        .ok()
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .ok_or(SignatureVerificationError::InvalidEncoding)?;
    let signature_text = decoder
        .decode(encoded_signature.trim())
        .ok()
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .ok_or(SignatureVerificationError::InvalidEncoding)?;
    let public_key = PublicKey::decode(&public_key_text)
        .map_err(|_| SignatureVerificationError::InvalidEncoding)?;
    let signature = Signature::decode(&signature_text)
        .map_err(|_| SignatureVerificationError::InvalidEncoding)?;
    public_key
        .verify(archive, &signature, true)
        .map_err(|_| SignatureVerificationError::InvalidSignature)
}
