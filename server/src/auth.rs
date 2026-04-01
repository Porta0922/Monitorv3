// JWT + Argon2id authentication
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation, Algorithm};
use serde::{Deserialize, Serialize};
use chrono::{Utc, Duration};
use argon2::{Argon2, PasswordHasher, PasswordHash, PasswordVerifier};
use argon2::password_hash::SaltString;
use rand::thread_rng;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,  // user_id
    pub exp: i64,     // expiration
    pub iat: i64,     // issued at
}

pub struct AuthManager {
    secret: String,
}

impl AuthManager {
    pub fn new(secret: &str) -> Self {
        Self {
            secret: secret.to_string(),
        }
    }

    /// Issue JWT token
    pub fn issue_token(&self, user_id: &str, expiration_hours: i64) -> Result<String, Box<dyn std::error::Error>> {
        let now = Utc::now();
        let exp = (now + Duration::hours(expiration_hours)).timestamp();
        let iat = now.timestamp();

        let claims = Claims {
            sub: user_id.to_string(),
            exp,
            iat,
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.secret.as_ref()),
        )?;

        Ok(token)
    }

    /// Verify JWT token and extract claims
    pub fn verify_token(&self, token: &str) -> Result<Claims, Box<dyn std::error::Error>> {
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.secret.as_ref()),
            &Validation::new(Algorithm::HS256),
        )?;

        Ok(token_data.claims)
    }

    /// Hash password using Argon2id
    pub fn hash_password(&self, password: &str) -> Result<String, Box<dyn std::error::Error>> {
        let argon2 = Argon2::default();
        let salt = SaltString::generate(thread_rng());
        let hash = argon2.hash_password(password.as_bytes(), &salt)?
            .to_string();
        Ok(hash)
    }

    /// Verify password against hash
    pub fn verify_password(&self, password: &str, hash: &str) -> Result<bool, Box<dyn std::error::Error>> {
        let parsed_hash = PasswordHash::new(hash)?;
        let argon2 = Argon2::default();
        
        Ok(argon2.verify_password(password.as_bytes(), &parsed_hash).is_ok())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenResponse {
    pub token: String,
    pub expires_in: i64,
    pub token_type: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_issuance() {
        let auth = AuthManager::new("test-secret");
        let token = auth.issue_token("user123", 24);
        assert!(token.is_ok());
    }

    #[test]
    fn test_token_verification() {
        let auth = AuthManager::new("test-secret");
        let token = auth.issue_token("user123", 24).expect("Failed to issue token");
        let claims = auth.verify_token(&token).expect("Failed to verify token");
        assert_eq!(claims.sub, "user123");
    }

    #[test]
    fn test_password_hashing() {
        let auth = AuthManager::new("test-secret");
        let password = "super-secret-password";
        let hash = auth.hash_password(password).expect("Failed to hash password");
        let verified = auth.verify_password(password, &hash).expect("Failed to verify password");
        assert!(verified);
    }

    #[test]
    fn test_wrong_password() {
        let auth = AuthManager::new("test-secret");
        let password = "correct-password";
        let hash = auth.hash_password(password).expect("Failed to hash password");
        let verified = auth.verify_password("wrong-password", &hash).expect("Failed to verify password");
        assert!(!verified);
    }
}
