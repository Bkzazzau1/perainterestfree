use magic_crypt::{new_magic_crypt, MagicCrypt256, MagicCryptTrait};

#[derive(Clone)]
pub struct CryptoService {
    crypt: MagicCrypt256,
}

impl CryptoService {
    pub fn new(key: &str) -> Self {
        Self {
            crypt: new_magic_crypt!(key, 256),
        }
    }

    // Encrypts a string, returns a Base64 encoded string
    pub fn encrypt(&self, plain_text: &str) -> String {
        self.crypt.encrypt_str_to_base64(plain_text)
    }

    // Decrypts a Base64 string, returns an error if it fails
    pub fn decrypt(&self, base64_encrypted: &str) -> Result<String, String> {
        self.crypt.decrypt_base64_to_string(base64_encrypted)
            .map_err(|e| e.to_string())
    }
}