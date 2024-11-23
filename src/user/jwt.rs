use std::{
    fmt::{write, Display},
    time::SystemTime,
};

#[derive(Debug, PartialEq)]
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
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, PartialEq)]
struct Jwt {
    header: JwtHeader,
    payload: Payload,
}

#[derive(Debug, PartialEq)]
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
}

#[derive(Debug, PartialEq)]
struct Payload {
    sub: i32,
    name: String,
    email: String,
    iat: SystemTime,
}

impl Payload {
    pub fn new(sub: i32, name: String, email: String, iat: SystemTime) -> Self {
        Self {
            sub,
            name,
            email,
            iat,
        }
    }
}

#[cfg(test)]
mod tests {
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
        let iat = SystemTime::now();
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
