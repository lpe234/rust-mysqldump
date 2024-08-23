use std::env;

use colored::Colorize;
use dotenv::dotenv;
use log::{error, info};
use mysql::OptsBuilder;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct DatabaseConfig {
    pub db_host: String,
    pub db_port: u16,
    pub db_username: String,
    pub db_password: String,
    pub db_exports: Vec<String>,
    pub db_forgets: Vec<String>,
    pub db_folder: String,
    pub db_backup_file_time_format: String,
}

impl DatabaseConfig {
    pub fn from_env() -> Result<Self, env::VarError> {
        // try read .env file
        match dotenv().ok() {
            None => {
                // cannot find.env file in working directory
                match env::current_exe() {
                    Ok(exe_path) => {
                        match exe_path.parent() {
                            None => {
                                error!("{}", "Failed to get exe dir.");
                            }
                            Some(exe_dir) => {
                                let env_path = exe_dir.join(".env");
                                let _ = dotenv::from_path(&env_path);
                                match dotenv::from_path(&env_path).ok() {
                                    None => {
                                        error!("{} -> {:?}", "Failed to load .env file.", env_path);
                                    }
                                    Some(_) => {
                                        info!("{} -> {:?}", "Success to load .env file.", env_path);
                                    }
                                }
                            }
                        }
                    }
                    Err(_) => {}
                }
            }
            Some(_) => {}
        }

        // FIXME: hardcode toumen
        let db_exports: Vec<String> = "toumen"
            .split(',')
            .map(|s| s.to_string())
            .collect();

        let db_forgets: Vec<String> = env::var("DB_FORGETS")?
            .split(',')
            .map(|s| s.to_string())
            .collect();

        Ok(Self {
            db_host: env::var("DB_HOST")?,
            db_port: env::var("DB_PORT")?.parse::<u16>().map_err(|_| env::VarError::NotPresent)?,
            db_username: env::var("DB_USERNAME")?,
            db_password: env::var("DB_PASSWORD")?,
            db_folder: env::var("DB_FOLDER")?,
            db_exports,
            db_forgets,
            db_backup_file_time_format: env::var("DB_BACKUP_FILE_TIME_FORMAT")?,
        })
    }

    #[allow(dead_code)]
    pub fn mysql_url(&self) -> String {
        format!(
            "mysql://{}:{}@{}:{}",
            self.db_username, self.db_password, self.db_host, self.db_port
        )
    }

    pub fn mysql_opts(&self) -> OptsBuilder {
        let builder = OptsBuilder::new()
            .ip_or_hostname(Some(&self.db_host))
            .tcp_port(self.db_port)
            .user(Some(&self.db_username))
            .pass(Some(&self.db_password));
        builder
    }
}
