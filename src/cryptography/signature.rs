use ed25519_dalek::{
    ed25519::signature::SignerMut, Signature, SigningKey, VerifyingKey, PUBLIC_KEY_LENGTH,
    SECRET_KEY_LENGTH,
};

pub fn generate_keypair() -> (Vec<u8>, Vec<u8>) {
    let signing_key = SigningKey::from_bytes(&[0u8; SECRET_KEY_LENGTH]);
    let verifying_key = signing_key.verifying_key();
    let secret_key = signing_key.to_bytes().to_vec();
    let public_key = verifying_key.to_bytes().to_vec();
    (secret_key, public_key)
}

pub fn sign(message: &[u8], secret_key: &[u8; SECRET_KEY_LENGTH]) -> Vec<u8> {
    let mut signing_key = SigningKey::from_bytes(secret_key);
    let signature = signing_key.sign(message);
    signature.to_bytes().to_vec()
}

pub fn verify(
    message: &[u8],
    signature: &[u8; Signature::BYTE_SIZE],
    public_key: &[u8; PUBLIC_KEY_LENGTH],
) -> bool {
    let verifying_key = VerifyingKey::from_bytes(public_key).unwrap();
    let signature = Signature::from_bytes(signature);
    verifying_key.verify_strict(message, &signature).is_ok()
}
