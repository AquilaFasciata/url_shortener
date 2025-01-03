use std::fs;

use serde::{Deserialize, Serialize};
use tracing::error;

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
}

impl Preferences {
    pub fn domain_name(&self) -> &String {
        &self.domain_name
    }
    pub fn port(&self) -> u32 {
        self.port
    }
    pub fn db_ip(&self) -> &String {
        &self.db_ip
    }
    pub fn db_user(&self) -> &String {
        &self.db_user
    }
    pub fn db_pass(&self) -> &String {
        &self.db_pass
    }
    pub fn db_port(&self) -> u32 {
        self.db_port
    }
    pub fn db_pool_size(&self) -> u32 {
        self.db_pool_size
    }
    pub fn db_name(&self) -> &String {
        &self.db_name
    }
    pub fn url_len(&self) -> usize {
        self.url_len
    }
    pub fn http_ip(&self) -> &String {
        &self.http_ip
    }
    pub fn load_config(path: &str) -> Result<Self, std::io::Error> {
        let path = path.trim_matches('/').to_owned() + "/config.toml";
        let file_buff = match fs::read_to_string(path.as_str()) {
            Ok(buff) => buff,
            Err(_) => return create_default_config(path.as_str()),
        };
        Ok(toml::from_str(file_buff.as_str()).expect("Unable to parse configuration file. {}"))
    }
    pub fn https_cert_path(&self) -> &Option<String> {
        &self.https_cert_path
    }
    pub fn https_key_path(&self) -> &Option<String> {
        &self.https_key_path
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
    };
    eprintln!("Using default password. \x1b[1mTHIS MUST BE CHANGED!!!\x1b[0m");
    error!("Using default password. \x1b[1mTHIS MUST BE CHANGED!!!\x1b[0m");
    fs::write(
        path,
        toml::to_string(&new_pref)
            .expect("Unable to convert preference to string. This shouldn't be possible!"),
    )?;
    Ok(new_pref)
}
