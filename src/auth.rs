use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    sub: i64,      // User ID in Postgres
    name: String,  // Username
    email: String, // Email
    exp: u64,      // Issued at time
}

impl Claims {
    pub fn new(sub: i64, name: String, email: String, exp: u64) -> Self {
        Self {
            sub,
            name,
            email,
            exp,
        }
    }

    pub fn iat(&self) -> u64 {
        self.exp
    }

    pub fn sub(&self) -> i64 {
        self.sub
    }

    pub fn email(&self) -> &str {
        &self.email
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}
