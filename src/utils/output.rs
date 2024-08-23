use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use cli_table::{Cell, Color, print_stdout, Style, Table};
use log::{info, warn};
use zip::AesMode::Aes256;
use zip::CompressionMethod;
use zip::write::SimpleFileOptions;

#[allow(dead_code)]
pub fn print_databases(databases: &Vec<(usize, String, u128)>) {
    let table_rows: Vec<_> = databases
        .into_iter()
        .map(|(i, db, duration)| vec![
            (i + 1).to_string().cell().foreground_color(Some(Color::Yellow)),
            db.cell().foreground_color(Some(Color::Yellow)),
            duration.to_string().cell().foreground_color(Some(Color::Yellow)),
        ])
        .collect();

    let table = table_rows
        .table()
        .title(vec![
            "Index".cell().bold(true),
            "Database Name".cell().bold(true),
            "Export Duration (microseconds)".cell().bold(true),
        ]);

    assert!(print_stdout(table).is_ok());
}

pub fn zip_file(file_path: &str, zip_path: &str) -> std::io::Result<()> {
    let data = std::fs::read(file_path)?;

    let zip_file = std::fs::File::create(zip_path)?;
    let mut zip = zip::ZipWriter::new(zip_file);

    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        // FIXME: hardcode metareal.cn
        .with_aes_encryption(Aes256, "metareal.cn")
        .unix_permissions(0o755);
    zip.start_file(Path::new(file_path).file_name().unwrap().to_str().unwrap(), options)?;
    zip.write_all(&data)?;

    Ok(())
}

pub fn remove_old_files(db_folder: &str, keep_count: u16) {
    let db_path = Path::new(db_folder);

    // Check if the directory exists
    if !db_path.exists() {
        info!("Directory '{}' does not exist.", db_folder);
        return;
    }

    // Get all .zip files in the directory
    let mut zip_files: Vec<(PathBuf, std::time::SystemTime)> = fs::read_dir(db_path)
        .unwrap()
        .filter_map(|entry| {
            let path = entry.unwrap().path();
            if path.extension().and_then(|e| e.to_str()) == Some("zip") {
                let metadata = fs::metadata(&path).ok()?;
                Some((path, metadata.created().unwrap_or(std::time::UNIX_EPOCH)))
            } else {
                None
            }
        })
        .collect();

    // Sort the .zip files by creation time
    zip_files.sort_by(|(_, time1), (_, time2)| time1.cmp(time2));

    // Keep the most recent files and delete the rest
    let mut delete_size = zip_files.len() - keep_count as usize;
    while delete_size > 0 {
        let (zfile, _) = zip_files.remove(0);
        info!("Deleting file: {}", &zfile.display());
        fs::remove_file(&zfile).unwrap_or_else(|err| {
            warn!("Error deleting file {}: {}", &zfile.display(), err);
        });
        delete_size -= 1;
    }
}
