use crate::errors::Result;
use aes_gcm::aead::rand_core::RngCore;
use aes_gcm::aead::{Aead, KeyInit, OsRng};
use aes_gcm::{Aes256Gcm, Nonce};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone)]
pub struct EncryptedValue {
    pub nonce: Vec<u8>,
    pub ciphertext: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct SecretStore {
    key: [u8; 32],
}

impl SecretStore {
    pub fn from_key_material(material: impl AsRef<[u8]>) -> Self {
        let digest = Sha256::digest(material.as_ref());
        let mut key = [0_u8; 32];
        key.copy_from_slice(&digest);
        Self { key }
    }

    fn cipher(&self) -> Result<Aes256Gcm> {
        Aes256Gcm::new_from_slice(&self.key)
            .map_err(|_| crate::errors::TelegramCliError::Message("invalid encryption key".into()))
    }

    pub fn encrypt_optional(&self, value: Option<&str>) -> Result<Option<EncryptedValue>> {
        let Some(value) = value else {
            return Ok(None);
        };

        let cipher = self.cipher()?;
        let mut nonce = [0_u8; 12];
        OsRng.fill_bytes(&mut nonce);
        let ciphertext = cipher
            .encrypt(Nonce::from_slice(&nonce), value.as_bytes())
            .map_err(|_| {
                crate::errors::TelegramCliError::Message("failed to encrypt secret".into())
            })?;

        Ok(Some(EncryptedValue {
            nonce: nonce.to_vec(),
            ciphertext,
        }))
    }

    pub fn decrypt_optional(&self, value: Option<&EncryptedValue>) -> Result<Option<String>> {
        let Some(value) = value else {
            return Ok(None);
        };

        let cipher = self.cipher()?;
        let plaintext = cipher
            .decrypt(Nonce::from_slice(&value.nonce), value.ciphertext.as_ref())
            .map_err(|_| {
                crate::errors::TelegramCliError::Message("failed to decrypt secret".into())
            })?;

        let decoded = String::from_utf8(plaintext).map_err(|_| {
            crate::errors::TelegramCliError::Message("secret is not valid utf-8".into())
        })?;
        Ok(Some(decoded))
    }

    pub fn encrypt_bytes(&self, value: &[u8]) -> Result<EncryptedValue> {
        let cipher = self.cipher()?;
        let mut nonce = [0_u8; 12];
        OsRng.fill_bytes(&mut nonce);
        let ciphertext = cipher
            .encrypt(Nonce::from_slice(&nonce), value)
            .map_err(|_| {
                crate::errors::TelegramCliError::Message("failed to encrypt secret".into())
            })?;

        Ok(EncryptedValue {
            nonce: nonce.to_vec(),
            ciphertext,
        })
    }

    pub fn decrypt_bytes(&self, value: &EncryptedValue) -> Result<Vec<u8>> {
        let cipher = self.cipher()?;
        cipher
            .decrypt(Nonce::from_slice(&value.nonce), value.ciphertext.as_ref())
            .map_err(|_| {
                crate::errors::TelegramCliError::Message("failed to decrypt secret".into())
            })
    }
}

#[cfg(test)]
mod tests {
    use super::SecretStore;

    #[test]
    fn secret_store_roundtrips_binary_payloads() {
        let store = SecretStore::from_key_material("test-key");
        let encrypted = store.encrypt_bytes(&[0, 1, 2, 254, 255]).unwrap();
        let decrypted = store.decrypt_bytes(&encrypted).unwrap();
        assert_eq!(decrypted, vec![0, 1, 2, 254, 255]);
    }
}
