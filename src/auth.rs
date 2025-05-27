use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    sub: i64,      // User ID in Postgres
    name: String,  // Username
    email: String, // Email
    iat: u64,      // Issued at time
}

impl Claims {
    pub fn new(sub: i64, name: String, email: String, iat: u64) -> Self {
        Self {
            sub,
            name,
            email,
            iat,
        }
    }

    pub fn iat(&self) -> u64 {
        self.iat
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

