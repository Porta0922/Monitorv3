// JWT + Argon2id authentication
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation, Algorithm};
use serde::{Deserialize, Serialize};
use chrono::{Utc, Duration};
use argon2::{Argon2, PasswordHasher, PasswordHash, PasswordVerifier};
use argon2::password_hash::SaltString;
use rand::thread_rng;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role { #[default] Viewer, Operator, Admin }
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Account { pub username: String, pub password_hash: String, pub role: Role, #[serde(default)] pub disabled: bool, #[serde(default)] pub totp_secret: Option<String>, #[serde(default)] pub session_epoch: u64 }

pub fn configured_accounts() -> Result<Option<Vec<Account>>, Box<dyn std::error::Error>> {
    let Some(path)=std::env::var_os("AUTH_USERS_FILE") else {return Ok(None);};
    let content=std::fs::read(path)?;
    if content.len()>1048576 {return Err("Account configuration too large".into());}
    let accounts:Vec<Account>=serde_json::from_slice(&content)?;
    if accounts.is_empty() || accounts.len()>1000 {return Err("Configure 1..1000 accounts".into());}
    let mut names=std::collections::HashSet::new();
    for account in &accounts {
        if account.username.trim()!=account.username || account.username.is_empty() || account.username.chars().any(char::is_control) || account.username.len()>100 || !names.insert(&account.username) {
            return Err("Invalid or duplicate account name".into());
        }
        let parsed=PasswordHash::new(&account.password_hash).map_err(|e|e.to_string())?;
        if parsed.version!=Some(19) || !parsed.params.get_decimal("p").is_some_and(|v|(1..=4).contains(&v)) || parsed.algorithm.as_str()!="argon2id" {return Err("Account hashes must use Argon2id".into());}
        if !parsed.params.get_decimal("m").is_some_and(|v|(19456..=262144).contains(&v)) || !parsed.params.get_decimal("t").is_some_and(|v|(2..=6).contains(&v)) {return Err("Unsupported Argon2id cost parameters".into());}
        if let Some(secret)=&account.totp_secret {crate::human_sessions::totp_key(secret).map_err(|_|"Invalid TOTP secret")?;}
        if crate::human_sessions::production() && account.role==Role::Admin && !account.disabled && account.totp_secret.is_none(){return Err("Production administrators require TOTP".into());}
    }
    Ok(Some(accounts))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,  // user_id
    pub exp: i64,     // expiration
    pub iat: i64,     // issued at
    #[serde(default)]
    pub role: Role,
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
        self.issue_role_token(user_id, Role::Admin, expiration_hours)
    }

    pub fn issue_role_token(&self, user_id: &str, role: Role, expiration_hours: i64) -> Result<String, Box<dyn std::error::Error>> {
        let now = Utc::now();
        let exp = (now + Duration::hours(expiration_hours)).timestamp();
        let iat = now.timestamp();

        let claims = Claims {
            sub: user_id.to_string(),
            exp,
            iat,
            role,
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.secret.as_ref()),
        )?;

        Ok(token)
    }

    pub(crate) fn csrf(&self,token:&str)->String {crate::human_sessions::mac(self.secret.as_bytes(),token.as_bytes())}

    /// Verify JWT token and extract claims
    #[allow(dead_code)]
    pub fn verify_token(&self, token: &str) -> Result<Claims, Box<dyn std::error::Error>> {
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.secret.as_ref()),
            &Validation::new(Algorithm::HS256),
        )?;

        let mut claims=token_data.claims;
        if let Some(accounts)=configured_accounts()? {
            let account=accounts.iter().find(|a|a.username==claims.sub && !a.disabled).ok_or("Account revoked")?;
            // Configuration changes immediately restrict existing sessions; promotion requires login.
            claims.role=match (claims.role,account.role) {
                (Role::Viewer,_)|(_,Role::Viewer)=>Role::Viewer,
                (Role::Operator,_)|(_,Role::Operator)=>Role::Operator,
                _=>Role::Admin,
            };
        }
        Ok(claims)
    }

    /// Hash password using Argon2id
    #[allow(dead_code)]
    pub fn hash_password(&self, password: &str) -> Result<String, Box<dyn std::error::Error>> {
        let argon2 = Argon2::default();
        let salt = SaltString::generate(thread_rng());
        let hash = argon2.hash_password(password.as_bytes(), &salt)
            .map_err(|e| e.to_string())?
            .to_string();
        Ok(hash)
    }

    /// Verify password against hash
    #[allow(dead_code)]
    pub fn verify_password(&self, password: &str, hash: &str) -> Result<bool, Box<dyn std::error::Error>> {
        let parsed_hash = PasswordHash::new(hash)
            .map_err(|e| e.to_string())?;
        let argon2 = Argon2::default();

        Ok(argon2.verify_password(password.as_bytes(), &parsed_hash).is_ok())
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)]
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
