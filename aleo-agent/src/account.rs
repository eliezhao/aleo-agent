//! Account implementations

use std::fmt::{Debug, Formatter};
use std::str::FromStr;

use super::*;
use anyhow::{anyhow, Result};
use indexmap::IndexMap;
use once_cell::sync::OnceCell;
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaChaRng;

#[derive(Clone)]
pub struct Account {
    private_key: PrivateKey,
    view_key: ViewKey,
    address: Address,
}

impl Debug for Account {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Account")
            .field("private_key", &self.private_key.to_string())
            .field("view_key", &self.view_key.to_string())
            .field("address", &self.address.to_string())
            .finish()
    }
}

impl Default for Account {
    fn default() -> Account {
        Self::from_seed(Default::default()).unwrap()
    }
}

impl Account {
    /// Generates a new `Account` using a random seed.
    pub fn new() -> Result<Self> {
        let (private_key, view_key, address) = generate_keypair()?;
        Ok(Account {
            private_key,
            view_key,
            address,
        })
    }

    /// Returns the private key of the account.
    pub fn private_key(&self) -> &PrivateKey {
        &self.private_key
    }

    /// Returns the address of the account.
    pub fn address(&self) -> &Address {
        &self.address
    }

    /// Returns the view key of the account.
    pub fn view_key(&self) -> &ViewKey {
        &self.view_key
    }

    /// Encrypts the private key into a ciphertext using a secret.
    ///
    /// # Arguments
    /// * `secret` - The secret used for encryption.
    ///
    /// # Returns
    /// The ciphertext.
    ///
    /// # Example
    /// ```ignore
    /// use std::str::FromStr;
    /// use aleo_agent::account::Account;
    /// use aleo_agent::PrivateKey;
    ///
    /// let acc = Account::from_private_key("PRIVATE KEY").unwrap();
    /// let encrypted_key = acc.get_encrypted_key("secret").expect("failed to encrypt key");
    /// let recover_account = Account::from_encrypted_key(&encrypted_key, "secret").expect("failed to decrypt key");
    ///
    /// assert_eq!(acc.private_key().to_string(), recover_account.private_key().to_string());
    /// ```
    pub fn get_encrypted_key(&self, secret: &str) -> Result<Ciphertext> {
        encrypt_field(&self.private_key.seed(), secret, "private_key")
    }

    /// Signs a message with the private key.
    ///
    /// # Arguments
    /// * `msg` - The message to sign.
    ///
    /// # Returns
    /// The signature.
    ///
    /// # Example
    /// ```
    /// use aleo_agent::account::Account;
    ///
    /// let acc = Account::new().unwrap();
    /// let sig = acc.sign("hello".as_bytes()).expect("failed to sign message");
    ///
    /// assert!(acc.verify("hello".as_bytes(), &sig));
    /// ```
    pub fn sign(&self, msg: &[u8]) -> Result<Signature> {
        let mut rng = ChaChaRng::from_entropy();
        self.private_key.sign_bytes(msg, &mut rng)
    }

    /// Verifies a message signature.
    ///
    /// # Arguments
    /// * `msg` - The message to verify.
    /// * `signature` - The signature to verify.
    ///
    /// # Returns
    /// `true` if the signature is valid, `false` otherwise.
    pub fn verify(&self, msg: &[u8], signature: &Signature) -> bool {
        signature.verify_bytes(&self.address, msg)
    }
}

impl Account {
    /// Generates a new `Account` from a seed.
    ///
    /// # Example
    /// ```
    /// use rand::Rng;
    /// use rand_chacha::ChaChaRng;
    /// use rand_chacha::rand_core::{SeedableRng};
    /// use aleo_agent::account::Account;
    /// use aleo_agent::PrivateKey;
    ///
    /// let mut rng = ChaChaRng::from_entropy();
    /// let seed : u64 = rng.gen();
    /// let account = Account::from_seed(seed).unwrap();
    ///
    /// let mut rng_from_seed = ChaChaRng::seed_from_u64(seed);
    /// let private_key = PrivateKey::new(&mut rng_from_seed).expect("failed to recover private key from seed");
    ///
    /// assert_eq!(account.private_key().to_string(), private_key.to_string());
    /// ```
    pub fn from_seed(seed: u64) -> Result<Self> {
        let (private_key, view_key, address) = generate_keypair_from_seed(seed)?;
        Ok(Account {
            private_key,
            view_key,
            address,
        })
    }

    /// Generates a new `Account` from a private key string.
    ///
    /// # Example
    /// ```ignore
    /// use std::str::FromStr;
    /// use aleo_agent::account::Account;
    /// use aleo_agent::PrivateKey;
    ///
    /// let private_key = PrivateKey::from_str("YOUR PRIVATE KEY").unwrap();
    /// let account = Account::from_private_key("YOUR PRIVATE KEY").unwrap();
    ///
    /// assert_eq!(account.private_key().to_string(), private_key.to_string());
    /// ```
    pub fn from_private_key(key: &str) -> Result<Self> {
        let private_key = PrivateKey::from_str(key)?;
        let view_key = ViewKey::try_from(&private_key)?;
        let address = Address::try_from(&private_key)?;
        Ok(Account {
            private_key,
            view_key,
            address,
        })
    }

    /// Decrypts a private key from ciphertext using a secret.
    ///
    /// # Arguments
    /// * `ciphertext` - The ciphertext of the encrypted private key.
    /// * `secret` - The secret used for decryption.
    ///
    /// # Returns
    /// The decrypted `Account`.
    ///
    /// # Example
    /// ```ignore
    /// use std::str::FromStr;
    /// use aleo_agent::account::Account;
    /// use aleo_agent::PrivateKey;
    ///
    /// let acc = Account::from_private_key("YOUR PRIVATE KET").unwrap();
    /// let encrypted_key = acc.get_encrypted_key("SECRET").expect("failed to encrypt key");
    /// let recover_account = Account::from_encrypted_key(&encrypted_key, "secret").expect("failed to decrypt key");
    ///
    /// assert_eq!(acc.private_key().to_string(), recover_account.private_key().to_string());
    /// ```
    pub fn from_encrypted_key(ciphertext: &Ciphertext, secret: &str) -> Result<Self> {
        let seed = decrypt_field(ciphertext, secret, "private_key")?;
        let private_key = PrivateKey::try_from(seed)?;
        let view_key = ViewKey::try_from(&private_key)?;
        let address = Address::try_from(&private_key)?;
        Ok(Account {
            private_key,
            view_key,
            address,
        })
    }
}

// Encrypted a field element into a ciphertext representation
fn encrypt_field(field: &Field, secret: &str, domain: &str) -> Result<Ciphertext> {
    // Derive the domain separators and the secret.
    let domain = Field::new_domain_separator(domain);
    let secret = Field::new_domain_separator(secret);

    // Generate a nonce
    let mut rng = rand::thread_rng();
    let nonce = Uniform::rand(&mut rng);

    // Derive a blinding factor and create an encryption target
    let blinding = CurrentNetwork::hash_psd2(&[domain, nonce, secret])?;
    let key = blinding * field;
    let plaintext = Plaintext::Struct(
        IndexMap::from_iter(vec![
            (
                Identifier::from_str("key")?,
                Plaintext::from(Literal::Field(key)),
            ),
            (
                Identifier::from_str("nonce")?,
                Plaintext::from(Literal::Field(nonce)),
            ),
        ]),
        OnceCell::new(),
    );
    plaintext.encrypt_symmetric(secret)
}

// Recover a field element encrypted within ciphertext
fn decrypt_field(ciphertext: &Ciphertext, secret: &str, domain: &str) -> Result<Field> {
    let domain = Field::new_domain_separator(domain);
    let secret = Field::new_domain_separator(secret);
    let decrypted = ciphertext.decrypt_symmetric(secret)?;
    let recovered_key = extract_value(&decrypted, "key")?;
    let recovered_nonce = extract_value(&decrypted, "nonce")?;
    let recovered_blinding = CurrentNetwork::hash_psd2(&[domain, recovered_nonce, secret])?;
    Ok(recovered_key / recovered_blinding)
}

// Extract a field element from a plaintext
fn extract_value(plaintext: &Plaintext, identifier: &str) -> Result<Field> {
    let identity = Identifier::from_str(identifier)?;
    let value = plaintext.find(&[identity])?;
    match value {
        Plaintext::Literal(literal, ..) => match literal {
            Literal::Field(recovered_value) => Ok(recovered_value),
            _ => Err(anyhow!("Wrong literal type")),
        },
        _ => Err(anyhow!("Expected literal")),
    }
}

fn generate_keypair_from_seed(seed: u64) -> Result<(PrivateKey, ViewKey, Address)> {
    let mut rng = ChaChaRng::seed_from_u64(seed);
    let private_key = PrivateKey::new(&mut rng)?;
    let view_key = ViewKey::try_from(&private_key)?;
    let address = Address::try_from(&private_key)?;
    Ok((private_key, view_key, address))
}

fn generate_keypair() -> Result<(PrivateKey, ViewKey, Address)> {
    let mut rng = ChaChaRng::from_entropy();
    let private_key = PrivateKey::new(&mut rng)?;
    let view_key = ViewKey::try_from(&private_key)?;
    let address = Address::try_from(&private_key)?;
    Ok((private_key, view_key, address))
}
