use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use ed25519_dalek::SigningKey;

fn main() {
    let first = uuid::Uuid::new_v4();
    let second = uuid::Uuid::new_v4();
    let mut seed = [0_u8; 32];
    seed[..16].copy_from_slice(first.as_bytes());
    seed[16..].copy_from_slice(second.as_bytes());
    let key = SigningKey::from_bytes(&seed);
    println!("LICENSE_SIGNING_KEY_B64={}", STANDARD.encode(seed));
    println!(
        "COMMERCECTRL_LICENSE_PUBLIC_KEY_B64={}",
        STANDARD.encode(key.verifying_key().to_bytes())
    );
}
