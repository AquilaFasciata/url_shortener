use core::{panic, str};
use std::{error::Error, fmt::Display};

use axum::response::Response;
use base64::{engine::general_purpose, Engine};
use hmac::{Hmac, Mac};
use serde::Deserialize;
use sha2::Sha256;

pub type HmacSha256 = Hmac<Sha256>;

type Result<T> = std::result::Result<T, JwtError>;

pub enum JwtError {
    ParsingError,
}

#[derive(Debug, PartialEq, Deserialize)]
enum SigAlgo {
    HS256,
    HS384,
    HS512,
    RS256,
    RS384,
    RS512,
    ES256,
    ES384,
    ES512,
    PS256,
    PS384,
    PS512,
}

impl SigAlgo {
    pub fn as_str(&self) -> &str {
        match self {
            Self::HS256 => "HS256",
            Self::HS384 => "HS384",
            Self::HS512 => "HS512",
            Self::RS256 => "RS256",
            Self::RS384 => "RS384",
            Self::RS512 => "RS512",
            Self::ES256 => "ES256",
            Self::ES384 => "ES384",
            Self::ES512 => "ES512",
            Self::PS256 => "PS256",
            Self::PS384 => "PS384",
            Self::PS512 => "PS512",
        }
    }
}

impl Display for SigAlgo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string().as_str())
    }
}

#[derive(Debug, PartialEq)]
struct Jwt {
    header: JwtHeader,
    payload: Payload,
}

impl Jwt {
    pub fn new(head: JwtHeader, payload: Payload) -> Self {
        Jwt {
            header: head,
            payload,
        }
    }
    pub fn finalize_hs256(&self, secret: &str) -> String {
        let header64 = general_purpose::STANDARD.encode(self.header().as_str());
        let payload64 = general_purpose::STANDARD.encode(self.payload().as_str());

        let partial_token = format!("{}.{}", header64, payload64);
        let mut signature = HmacSha256::new_from_slice(secret.as_bytes())
            .expect("Error creating HMAC key; this shouldn't be possible!");
        signature.update(partial_token.as_bytes());

        let signature = signature.finalize().into_bytes();
        return format!(
            "{partial_token}.{}",
            str::from_utf8(&signature).expect("Unable to parse signature")
        );
    }
    pub fn header(&self) -> &JwtHeader {
        &self.header
    }
    pub fn payload(&self) -> &Payload {
        &self.payload
    }
    pub fn finalize(&self, secret: &str) -> String {
        match self.header().alg() {
            SigAlgo::HS256 => return self.finalize_hs256(secret),
            _ => {
                tracing::error!("not yet implemented!");
                return String::new();
            }
        }
    }
    pub fn from_str(token: &str, secret: &str) -> Result<(Self, String)> {
        let parts: Vec<&str> = token.split_terminator('.').collect();
        if parts.len() > 3 {
            panic!("This isn't a valid")
        }
        let provided_hash = parts.last();

        let mut test_hash: HmacSha256 =
            HmacSha256::new_from_slice(secret.as_bytes()).expect("Error setting secret key");
        test_hash.update(format!("{}.{}", parts.get(0), parts.get(1)).as_bytes());

        return Ok((Jwt {}, String::from("test")));
    }
}

#[derive(Debug, PartialEq, Deserialize)]
struct JwtHeader {
    alg: SigAlgo,
    r#type: String,
}

impl JwtHeader {
    pub fn new(alg: SigAlgo, r#type: String) -> Self {
        Self { alg, r#type }
    }
    pub fn defaults() -> Self {
        Self::new(SigAlgo::HS256, String::from("JWT"))
    }
    pub fn alg(&self) -> SigAlgo {
        self.alg
    }
    pub fn r#type(&self) -> &String {
        &self.r#type
    }
    pub fn as_str(&self) -> &str {
        format!("{{\"alg\":\"{}\",\"typ\":\"{}\"}}", &self.alg, &self.r#type).as_str()
    }
}

impl Display for JwtHeader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{{\"alg\":\"{}\",\"typ\":\"{}\"}}",
            self.alg(),
            self.r#type
        )
    }
}

#[derive(Debug, PartialEq, Deserialize)]
struct Payload {
    sub: i32,
    name: String,
    email: String,
    iat: u64,
}

impl Payload {
    pub fn new(sub: i32, name: String, email: String, iat: u64) -> Self {
        Self {
            sub,
            name,
            email,
            iat,
        }
    }
    pub fn as_str(&self) -> &str {
        self.to_string().as_str()
    }
}

impl Display for Payload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let sub_pair = format!("\"sub\":\"{}\"", self.sub);
        let name_pair = format!("\"name\":\"{}\"", self.name);
        let email_pair = format!("\"email\":\"{}\"", self.email);
        let iat_pair = format!("\"iat\":{}", self.iat);
        write!(f, "{{{sub_pair},{name_pair},{email_pair},{iat_pair}}}")
    }
}

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use super::*;

    #[test]
    fn header_construction() {
        let header = JwtHeader::new(SigAlgo::HS256, String::from("JWT"));
        let default = JwtHeader::defaults();
        assert_eq!(header, default);
    }

    #[test]
    fn payload() {
        let sub = 14;
        let email = "me@example.com".to_string();
        let name = "Test Man".to_string();
        let iat = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let constructor_payload = Payload::new(sub, name.clone(), email.clone(), iat);
        let control_payload = Payload {
            sub,
            name,
            email,
            iat,
        };
        assert_eq!(control_payload, constructor_payload);
    }
}
