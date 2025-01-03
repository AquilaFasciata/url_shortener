use core::str;
use std::fmt::Display;

use askama::Result;
use base64::{engine::general_purpose::STANDARD_NO_PAD, Engine};
use hmac::{Hmac, Mac};
use serde::Deserialize;
use sha2::Sha256;

pub type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, PartialEq, Deserialize)]
pub enum JwtError {
    ParsingError,
    IncorrectLength,
    SerdeError(String),
}

impl Display for JwtError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SerdeError(msg) => write!(f, "SerdeError: {msg}"),
            _ => write!(f, "{:#?}", self),
        }
    }
}

impl std::error::Error for JwtError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            JwtError::SerdeError(err) => Some(Err(err).unwrap()),
            _ => None,
        }
    }
}

impl serde::de::Error for JwtError {
    fn custom<T>(msg: T) -> Self
    where
        T: Display,
    {
        JwtError::SerdeError(msg.to_string())
    }
}

#[derive(Debug, PartialEq, Deserialize, Clone, Copy)]
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
    payload: JwtPayload,
    signature: Option<String>,
}

impl Jwt {
    pub fn new(head: JwtHeader, payload: JwtPayload) -> Self {
        Jwt {
            header: head,
            payload,
            signature: None,
        }
    }
    pub fn finalize_hs256(&self, secret: &str) -> String {
        let header64 = STANDARD_NO_PAD.encode(self.header().to_string().as_str());
        let payload64 = STANDARD_NO_PAD.encode(self.payload().to_string().as_str());

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
    pub fn payload(&self) -> &JwtPayload {
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
    /// Creates a JWT object from a base64 string. This does *NOT* implement the FromStr trait
    /// because it returns a tuple with the calculated signature for convience when comparing with
    /// the signature in the provided JWT
    pub fn from_str(token: &str, secret: &str) -> Result<(Self, String), impl serde::de::Error> {
        let parts: Vec<&str> = token.split_terminator('.').collect();
        if parts.len() != 3 {
            return Err(JwtError::IncorrectLength);
        }

        let mut test_hash: HmacSha256 =
            HmacSha256::new_from_slice(secret.as_bytes()).expect("Error setting secret key");
        test_hash.update(format!("{{{}}}.{{{}}}", parts[0], parts[1]).as_bytes());

        let Ok(test_hash) = String::from_utf8(test_hash.finalize().into_bytes().to_vec()) else {
            return Err(JwtError::ParsingError);
        };
        let provided_hash = String::from(parts[2]);

        let header_decoded = STANDARD_NO_PAD.decode(parts[0]).unwrap();
        let header: JwtHeader =
            match serde_json::from_str(str::from_utf8(header_decoded.as_slice()).unwrap()) {
                Ok(val) => val,
                Err(e) => return Err(JwtError::SerdeError(e.to_string())),
            };
        let Ok(payload_decoded) = STANDARD_NO_PAD.decode(parts[1]) else {
            return Err(JwtError::ParsingError);
        };

        let payload: JwtPayload = match serde_json::from_slice(payload_decoded.as_slice()) {
            Ok(val) => val,
            Err(e) => return Err(JwtError::SerdeError(e.to_string())),
        };

        let signature = Some(provided_hash);

        let supplied_token = Self {
            header,
            payload,
            signature,
        };

        return Ok((supplied_token, test_hash));
    }
}

#[derive(Debug, PartialEq, Deserialize)]
struct JwtHeader {
    alg: SigAlgo,
    r#type: String,
}

impl JwtHeader {
    /// Creates a new header using the alogrithm specified by the [SigAlgo] enum and the type. Any
    /// type supported by javascript tokens *should* be supported; though JWT should be the only
    /// one used as of now, so that is all I test
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
struct JwtPayload {
    sub: i32,
    name: String,
    email: String,
    iat: u64,
}

impl JwtPayload {
    /// Creates a new payload with a provided subscriber, name, email, and the issued at time.
    pub fn new(sub: i32, name: String, email: String, iat: u64) -> Self {
        Self {
            sub,
            name,
            email,
            iat,
        }
    }
}

impl Display for JwtPayload {
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
    use std::time::{SystemTime, UNIX_EPOCH};

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
        let constructor_payload = JwtPayload::new(sub, name.clone(), email.clone(), iat);
        let control_payload = JwtPayload {
            sub,
            name,
            email,
            iat,
        };
        assert_eq!(control_payload, constructor_payload);
    }

    #[test]
    fn full_jwt() {
        let iat = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let header = JwtHeader::defaults();
        let payload = JwtPayload::new(
            143,
            String::from("John"),
            String::from("test@example.com"),
            iat,
        );
        let token = Jwt::new(header.clone(), payload.clone());

        assert_eq!(
            token,
            Jwt {
                header,
                payload,
                signature: None
            }
        );
    }
}
