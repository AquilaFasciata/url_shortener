use std::fs;

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct Preferences {
    domain_name: String,
    http_ip: String,
    port: u32,
    db_ip: String,
    db_name: String,
    db_user: String,
    db_pass: String,
    db_port: u32,
    db_pool_size: u32,
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
    pub fn load_config(path: &str) -> Self {
        let file_buff = fs::read_to_string(path).expect("Unable to read configuration file");
        toml::from_str(file_buff.as_str()).expect("Unable to parse configuration file. {}")
    }
}

fn create_default_config(path: String) -> Result<Preferences, std::io::Error> {
    let new_pref = Preferences {
        domain_name: String::from("localhost"),
        http_ip: String::from("127.0.0.1"),
        port: 8080,
        db_ip: String::from("127.0.0.1"),
        db_name: String::from("shortener"),
        db_user: String::from("postgres"),
        db_pass: String::from("THISISVERYBAD PLEASE CHANGE ME"),
        db_port: 5432,
        db_pool_size: 10,
    };
    eprintln!("Using default password. \x1b[1mTHIS MUST BE CHANGED!!!\x1b[0m");
    fs::write(
        path,
        toml::to_string(&new_pref)
            .expect("Unable to convert preference to string. This shouldn't be possible!"),
    )?;
    Ok(new_pref)
}
