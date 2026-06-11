use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use sha2::{Digest, Sha256};

#[derive(Debug)]
pub enum VaultError {
    InvalidBase64,
    DecryptionFailed,
}

pub struct Vault {
    cipher: Aes256Gcm,
}

impl Vault {
    pub fn new(master_secret: &str) -> Vault {
        let key_bytes = Sha256::digest(master_secret.as_bytes());
        let cipher = Aes256Gcm::new_from_slice(&key_bytes)
            .expect("SHA-256 output is always 32 bytes");
        Vault { cipher }
    }

    pub fn encrypt(&self, plaintext: &str) -> String {
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let ciphertext = self
            .cipher
            .encrypt(&nonce, plaintext.as_bytes())
            .expect("AES-GCM encryption is infallible with a valid key");
        let mut combined = nonce.to_vec();
        combined.extend_from_slice(&ciphertext);
        STANDARD.encode(&combined)
    }

    pub fn decrypt(&self, encoded: &str) -> Result<String, VaultError> {
        let data = STANDARD
            .decode(encoded)
            .map_err(|_| VaultError::InvalidBase64)?;
        if data.len() < 12 {
            return Err(VaultError::DecryptionFailed);
        }
        let (nonce_bytes, ciphertext) = data.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);
        let plaintext = self
            .cipher
            .decrypt(nonce, ciphertext)
            .map_err(|_| VaultError::DecryptionFailed)?;
        String::from_utf8(plaintext).map_err(|_| VaultError::DecryptionFailed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let vault = Vault::new("test-master-secret");
        let plaintext = "sk-test-api-key-12345";
        let encrypted = vault.encrypt(plaintext);
        let decrypted = vault.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn wrong_key_fails_decryption() {
        let vault_a = Vault::new("secret-a");
        let vault_b = Vault::new("secret-b");
        let encrypted = vault_a.encrypt("sensitive-value");
        assert!(matches!(
            vault_b.decrypt(&encrypted),
            Err(VaultError::DecryptionFailed)
        ));
    }

    #[test]
    fn empty_string_roundtrip() {
        let vault = Vault::new("test-master-secret");
        let encrypted = vault.encrypt("");
        let decrypted = vault.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, "");
    }
}
