use std::fs;

use serde::{Deserialize, Serialize};
use tracing::error;

#[derive(Debug)]
pub enum PrefError {
    IoError(std::io::Error),
    TomlError(toml::de::Error),
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Preferences {
    url_len: usize,
    domain_name: String,
    http_ip: String,
    port: u32,
    db_ip: String,
    db_name: String,
    db_user: String,
    db_pass: String,
    db_port: u32,
    db_pool_size: u32,
    https_cert_path: Option<String>,
    https_key_path: Option<String>,
    jwt_secret: String,
    // TODO: Log verbosity
}

impl Preferences {
    pub fn domain_name(&self) -> &String {
        &self.domain_name
    }
    pub fn port(&self) -> u32 {
        self.port
    }
    pub fn db_ip(&self) -> &str {
        self.db_ip.as_str()
    }
    pub fn db_user(&self) -> &str {
        self.db_user.as_str()
    }
    pub fn db_pass(&self) -> &str {
        self.db_pass.as_str()
    }
    pub fn db_port(&self) -> u32 {
        self.db_port
    }
    pub fn db_pool_size(&self) -> u32 {
        self.db_pool_size
    }
    pub fn db_name(&self) -> &str {
        self.db_name.as_str()
    }
    pub fn url_len(&self) -> usize {
        self.url_len
    }
    pub fn http_ip(&self) -> &str {
        self.http_ip.as_str()
    }
    pub fn load_config(path: &str) -> Result<Self, PrefError> {
        eprintln!("Config path is {}", path);
        let file_buff = match fs::read_to_string(path) {
            Ok(buff) => buff,
            Err(_) => return create_default_config(path).map_err(|err| PrefError::IoError(err)),
        };
        match toml::from_str(file_buff.as_str()) {
            Ok(ret) => Ok(ret),
            Err(err) => {
                if err.message().contains("missing field") {
                    fs::write(
                        path,
                        format!(
                            "{}\n{} = ",
                            file_buff.trim_end(),
                            err.message()
                                .split_terminator('`')
                                .last()
                                .expect("Error adding field to config file")
                        ),
                    )
                    .expect("Error adding field to config file");
                    return Self::load_config(path);
                } else if err.message().contains("invalid string") {
                    fs::write(
                        path,
                        format!("{}\"{}\"", file_buff.trim_end(), "DEFAULTPLEASECHANGE"),
                    )
                    .expect("Error changing field in config file");
                    return Self::load_config(path);
                } else {
                    return Err(PrefError::TomlError(err));
                }
            }
        }
    }
    pub fn https_cert_path(&self) -> &Option<String> {
        &self.https_cert_path
    }
    pub fn https_key_path(&self) -> &Option<String> {
        &self.https_key_path
    }
    pub fn jwt_secret(&self) -> &str {
        self.jwt_secret.as_str()
    }
}

fn create_default_config(path: &str) -> Result<Preferences, std::io::Error> {
    let new_pref = Preferences {
        url_len: 6,
        domain_name: String::from("localhost"),
        http_ip: String::from("127.0.0.1"),
        port: 8080,
        db_ip: String::from("127.0.0.1"),
        db_name: String::from("shortener"),
        db_user: String::from("postgres"),
        db_pass: String::from("THISISVERYBAD PLEASE CHANGE ME"),
        db_port: 5432,
        db_pool_size: 10,
        https_cert_path: None,
        https_key_path: None,
        jwt_secret: String::from("THISISALSOVERYBAD CHANGE!!"),
    };
    eprintln!("Using default passwords. \x1b[1mTHIS MUST BE CHANGED!!!\x1b[0m");
    error!("Using default passwords. \x1b[1mTHIS MUST BE CHANGED!!!\x1b[0m");
    fs::write(
        path,
        toml::to_string(&new_pref)
            .expect("Unable to convert preference to string. This shouldn't be possible!"),
    )?;
    Ok(new_pref)
}
