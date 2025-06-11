use reqwest::blocking::Client;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};
use std::net::TcpListener;
use std::io::{Read, Write};
use std::sync::RwLock;

const APP_KEY : &str = "6wlx6pjakmcr5qg";
const APP_SECRET : &str = "fve04ck9z6umxjf";
const TOKEN_FILE_PATH : &str = "C:\\temp\\tk.json";
pub static  MYCODE : RwLock<&str> = RwLock::new("MpYKGK0kxjIAAAAAAAAAHD-NMPxC2oJeHgkfHigPib0");
pub static REFRESH_TOKEN: RwLock<String> = RwLock::new(String::new());
pub static ACCESS_TOKEN : RwLock<Option<TokenResponse>> = RwLock::new(None);
pub static TIME_GET_TOKEN: RwLock<Option<i64>> = RwLock::new(None);

pub fn listener() {
    let listener = TcpListener::bind("127.0.0.1:3388").unwrap();
    println!("Listening on http://localhost:3388");

    for stream in listener.incoming() {
        let mut stream = stream.unwrap();
        let mut buffer = [0; 1024];
        stream.read(&mut buffer).unwrap();

        let request = String::from_utf8_lossy(&buffer[..]);
        if request.contains("GET /oauth_callback?code=") {
            // Extract the code
            if let Some(start) = request.find("/oauth_callback?code=") {
                let code = &request[start + 19..];
                let code = code.split_whitespace().next().unwrap_or("").split('&').next().unwrap();
                println!("Authorization code: {}", code);

                let response = "HTTP/1.1 200 OK\r\n\r\nYou can close this tab.";
                stream.write_all(response.as_bytes()).unwrap();
                break;
            }
        }
    }
}

// https://www.dropbox.com/oauth2/authorize?client_id=6wlx6pjakmcr5qg&response_type=code&token_access_type=offline&redirect_uri=http://localhost:3388/oauth_callback&scope=files.content.write files.content.read
// MpYKGK0kxjIAAAAAAAAAGZXpifGG9yQf2mxoeVFBipQ

pub fn get_token () -> Result<String, Box<dyn std::error::Error>> {
    let redirect_uri = "http://localhost:3388/oauth_callback"; // hoặc "urn:ietf:wg:oauth:2.0:oob"

    // 2. Tạo header Authorization: Basic base64(app_key:app_secret)
    let credentials = format!("{}:{}", APP_KEY, APP_SECRET);
    let auth_header = format!(
        "Basic {}",
        general_purpose::STANDARD.encode(credentials)
    );

    // 3. Gửi POST request để lấy token
    let client = Client::new();
    let res = client
        .post("https://api.dropboxapi.com/oauth2/token")
        .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
        .header(AUTHORIZATION, auth_header)
        .body(format!(
            "code={}&grant_type=authorization_code&redirect_uri={}",
            MYCODE.read().unwrap(), redirect_uri
        ))
        .send()?;

    if !res.status().is_success() {
        let err_text = res.text()?;
        eprintln!("Failed to get token: {}", err_text);
        return Err(Box::new(
            std::io::Error::new(std::io::ErrorKind::Other, err_text),
        ));
    }

    let token_response: TokenResponse = res.json()?;

    // let mut t = REFRESH_TOKEN.write().unwrap();
    // *t = token_response.refresh_token.clone().unwrap_or_default();

    let mut at = ACCESS_TOKEN.write().unwrap();
    *at = Some(token_response.clone());

    let mut t = TIME_GET_TOKEN.write().unwrap();
    *t = Some(chrono::Utc::now().timestamp_millis());

    write_token_to_file(&token_response)?;
    println!("\n🎉 Token received:");
    println!("Access Token: {}", token_response.access_token);
    println!("Refresh Token: {}", token_response.refresh_token.as_deref().unwrap_or("None"));
    println!("Expires In: {} seconds", token_response.expires_in.unwrap_or(0));

    Ok(token_response.access_token)
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: Option<u64>,
    pub refresh_token: Option<String>,
    pub scope: Option<String>,
    pub uid: Option<String>,
    pub account_id: Option<String>,
}

pub fn refresh_token() -> Result<String, Box<dyn std::error::Error>> {
    // Tạo chuỗi Authorization: Basic base64(app_key:app_secret)
    let credentials = format!("{}:{}", APP_KEY, APP_SECRET);
    let auth_header = format!(
        "Basic {}",
        general_purpose::STANDARD.encode(credentials)
    );
    let ref_token = match ACCESS_TOKEN.read() {
        Ok(at) => match &*at {
            Some(token) => token.refresh_token.clone().unwrap_or_default(),
            None => {
                return Err(Box::new(
                    std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        "No access token found. Please authenticate first.",
                    ),
                ));
            }
        },
        Err(e) => {
            return Err(Box::new(e));
        }
    };
    let client = Client::new();
    let res = client
        .post("https://api.dropboxapi.com/oauth2/token")
        .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
        .header(AUTHORIZATION, auth_header)
        .body(format!(
            "grant_type=refresh_token&refresh_token={}",
            ref_token
        ))
        .send()?;

    if !res.status().is_success() {
        let err = res.text()?;
        eprintln!("❌ Refresh token failed: {}", err);
        return Err(Box::new(
            std::io::Error::new(std::io::ErrorKind::Other, err),
        ));
    }

    let token: TokenResponse = res.json()?;
    
    let mut at: std::sync::RwLockWriteGuard<'_, Option<TokenResponse>> = ACCESS_TOKEN.write().unwrap();
    *at = Some(token.clone());

    let mut t = TIME_GET_TOKEN.write().unwrap();
    *t = Some(chrono::Utc::now().timestamp_millis());

    write_token_to_file(&token)?;
    
    println!("\n🎉 Token received:");
    println!("Access Token: {}", token.access_token);
    println!("Refresh Token: {}", token.refresh_token.as_deref().unwrap_or("None"));
    println!("Expires In: {} seconds", token.expires_in.unwrap_or(0));
    Ok(token.access_token)
}

pub fn is_token_valid() -> bool {
    let expire_in = match ACCESS_TOKEN.read() {
        Ok(at) => match &*at {
            Some(token) => token.expires_in,
            None => return false,
        },
        Err(_) => return false,
    };
    let current_time = chrono::Utc::now().timestamp_millis();
    let time_get_token = match TIME_GET_TOKEN.read() {
        Ok(tgt) => match &*tgt {
            Some(time) => *time,
            None => return false,
        },
        Err(_) => return false,
    };
    if let Some(expire_in) = expire_in {
        let token_expiry_time = time_get_token + (expire_in as i64 * 1000);
        return current_time < token_expiry_time;
    }else{
        return false; 
    }
}

pub fn write_token_to_file(token: &TokenResponse) -> std::io::Result<()> {
    // Ensure the directory exists
    if let Some(parent) = std::path::Path::new(TOKEN_FILE_PATH).parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string(token)?;
    std::fs::write(TOKEN_FILE_PATH, json)?;
    Ok(())
}

pub fn read_token_from_file() -> std::io::Result<TokenResponse> {
    let json = std::fs::read_to_string(TOKEN_FILE_PATH)?;
    let token: TokenResponse = serde_json::from_str(&json)?;
    Ok(token)
}