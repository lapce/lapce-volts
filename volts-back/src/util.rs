use rand::{distributions::Uniform, rngs::OsRng, Rng};
use sha2::{Digest, Sha256};

pub struct SecureToken {
    plaintext: String,
    token: Vec<u8>,
}

impl SecureToken {
    pub fn new_token() -> Self {
        let plaintext = generate_secure_alphanumeric_string(32);
        let token = Sha256::digest(plaintext.as_bytes()).as_slice().to_vec();
        Self { plaintext, token }
    }

    pub fn parse(plaintext: &str) -> Self {
        let token = Sha256::digest(plaintext.as_bytes()).as_slice().to_vec();
        Self {
            plaintext: plaintext.to_string(),
            token,
        }
    }

    pub fn plaintext(&self) -> &str {
        &self.plaintext
    }

    pub fn token(&self) -> &[u8] {
        &self.token
    }
}

fn generate_secure_alphanumeric_string(len: usize) -> String {
    const CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

    OsRng
        .sample_iter(Uniform::from(0..CHARS.len()))
        .map(|idx| CHARS[idx] as char)
        .take(len)
        .collect()
}
