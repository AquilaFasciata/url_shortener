enum SigAlgo {
    HS256,
}

impl SigAlgo {
    pub fn as_str(&self) -> &str {
        match self {
            Self::HS256 => "HS256",
        }
    }
}

struct Jwt {}

struct JwtHeader {
    alg: SigAlgo,
}
