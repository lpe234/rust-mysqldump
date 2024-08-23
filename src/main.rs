use std::fs;
use std::fs::File;
use std::io::Write;
use std::time::Instant;

use chrono::Local;
use colored::Colorize;
use mysql::*;
use mysql::prelude::*;
use tokio::process::Command;

use config::database::DatabaseConfig;
use log::{error, info, warn};
use utils::output::print_databases;

mod config;
mod utils;
async fn run_mysqldump(config: &DatabaseConfig, databases: Vec<String>) -> std::io::Result<Vec<(usize, String, u128)>> {
    if !std::path::Path::new(&config.db_folder).exists() {
        fs::create_dir_all(&config.db_folder)?;
    }

    let dbs_to_dump = if config.db_exports.contains(&"*".to_string()) {
        databases
            .iter()
            .filter(|db| !config.db_forgets.contains(db))
            .collect::<Vec<_>>()
    } else {
        config
            .db_exports
            .iter()
            .filter(|db| databases.contains(db))
            .collect::<Vec<_>>()
    };

    let mut successful_dumps = Vec::new();

    if dbs_to_dump.is_empty() {
        warn!("{}", "No databases to dump.");
    }
    
    for (i, db) in dbs_to_dump.iter().enumerate() {
        let start = Instant::now();

        let command = format!(
            "mysqldump --host={} --port={} --user={} --password={} {}",
            &config.db_host,
            config.db_port,
            &config.db_username,
            &config.db_password,
            db
        );

        let args: Vec<&str> = command.split_whitespace().collect();

        let output = Command::new(&args[0])
            .args(&args[1..])
            .output()
            .await?;

        if output.status.success() {
            let duration = start.elapsed().as_micros();
            info!("Successfully dumped database: {} (took {} microseconds)", db, duration);

            let mut filename = format!("{}/{}.sql", &config.db_folder, db);
            let mut zip_filename = format!("{}/{}.zip", &config.db_folder, db);
            //
            if !&config.db_backup_file_time_format.is_empty() {
                let time_str = Local::now().format(&config.db_backup_file_time_format);
                filename = format!("{}/{}_{}.sql", &config.db_folder, db, time_str);
                zip_filename = format!("{}/{}_{}.zip", &config.db_folder, db, time_str);
            }

            let mut file = File::create(&filename)?;
            file.write_all(&output.stdout)?;

            utils::output::zip_file(&filename, &zip_filename)?;
            info!("Successfully zipped database: {}", zip_filename);
            // remove raw file
            fs::remove_file(&filename).ok();

            // try remove old files
            utils::output::remove_old_files(&config.db_folder, config.db_backup_file_keep_size);
            
            successful_dumps.push((i, db.to_string(), duration));
        } else {
            error!("{}", format!("Failed to dump database: {}", db).red());

            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            info!("STDOUT: {}", stdout);
            info!("STDERR: {}", stderr);
        }
    }

    Ok(successful_dumps)
}

async fn get_databases(config: &DatabaseConfig) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let opts = config.mysql_opts();
    let pool = Pool::new(opts)?;
    let mut conn = pool.get_conn()?;

    let databases: Vec<String> = conn.query_map("SHOW DATABASES", |database: String| database)?;

    Ok(databases)
}

#[tokio::main]
async fn main() {
    log4rs::init_file("log4rs.yml", Default::default()).unwrap();
    info!("");
    info!("Starting mysqldump...");
    //
    match DatabaseConfig::from_env() {
        Ok(config) => {
            match get_databases(&config).await {
                Ok(databases) => {
                    match run_mysqldump(&config, databases).await {
                        Ok(mut successful_dumps) => {
                            successful_dumps.sort_by(|a, b| a.2.cmp(&b.2));
                            print_databases(&successful_dumps);
                        }
                        Err(e) => error!("{}", format!("Failed to run mysqldump: {}", e).red()),
                    }
                }
                Err(e) => error!("{}", format!("Failed to get databases: {}", e).red()),
            }
        }
        Err(e) => error!("{}", format!("Failed to read .env file: {}", e).red()),
    }
}

#[test]
fn test_get_databases() {
    let rt = tokio::runtime::Runtime::new().unwrap();

    rt.block_on(async {
        let config = DatabaseConfig::from_env().unwrap();
        // println!("Config: {:?}", config);

        let opts = config.mysql_opts();
        let pool = Pool::new(opts).unwrap();
        let mut conn = pool.get_conn().unwrap();

        // Seed data
        conn.query_drop("CREATE DATABASE IF NOT EXISTS db1").unwrap();
        conn.query_drop("CREATE DATABASE IF NOT EXISTS db2").unwrap();

        // Run the function to test
        let databases = get_databases(&config).await.unwrap();
        // println!("Databases: {:?}", databases);

        // Check the results
        // assert_eq!(databases, vec!["db1", "db2"]);
        assert!(databases.contains(&"db1".to_string()));
        assert!(databases.contains(&"db2".to_string()));

        // Cleanup
        conn.query_drop("DROP DATABASE db1").unwrap();
        conn.query_drop("DROP DATABASE db2").unwrap();
    });
}
