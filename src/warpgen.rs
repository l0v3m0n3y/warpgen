use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, HOST, CONNECTION, ACCEPT_LANGUAGE, USER_AGENT, ORIGIN, REFERER};
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};
use std::fs;
use std::path::{Path, PathBuf};
use chrono::Local;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

pub struct WarpGen {
    api: String,
    api_valokda: String,
    headers: Arc<Mutex<HeaderMap>>,
}

impl WarpGen {
    pub fn new() -> Self {
        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT, HeaderValue::from_static("*/*"));
        headers.insert(CONNECTION, HeaderValue::from_static("keep-alive"));
        headers.insert(ACCEPT_LANGUAGE, HeaderValue::from_static("en-US,en;q=0.9"));
        headers.insert(USER_AGENT, HeaderValue::from_static("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/139.0.0.0 Safari/537.36"));

        Self {
            api_valokda: "https://valokda-amnezia.vercel.app/api".to_string(),
            api: "https://warp-generator.vercel.app/api".to_string(),
            headers: Arc::new(Mutex::new(headers)),
        }
    }

    pub fn get_headers_for(&self, base_url: &str) -> HeaderMap {
        let mut h = self.headers.lock().unwrap().clone();
        if let Ok(url) = reqwest::Url::parse(base_url) {
            if let Some(host) = url.host_str() {
                h.insert(HOST, HeaderValue::from_str(host).unwrap());
                h.insert(ORIGIN, HeaderValue::from_str(&format!("https://{}", host)).unwrap());
                h.insert(REFERER, HeaderValue::from_str(&format!("https://{}/", host)).unwrap());
            }
        }
        h
    }

    pub async fn get_warp_valokda(&self) -> Result<Value, Box<dyn std::error::Error>> {
        let url = format!("{}/warp", self.api_valokda);
        let client = reqwest::Client::new();
        let current_headers = self.get_headers_for(&self.api_valokda);
        let response = client
            .get(&url)
            .headers(current_headers)
            .send()
            .await?;
        let body = response.json().await?;

        Ok(body)
    }

    pub async fn decode_config_valokda(&self) -> Result<String, Box<dyn std::error::Error>> {
        let welcome_data = self.get_warp_valokda().await?;
        
        let config_base64 = welcome_data["content"].as_str().ok_or("configBase64 not found in response")?;
        
        let decoded_bytes = BASE64.decode(config_base64)?;
        let decoded_string = String::from_utf8(decoded_bytes)?;
        
        Ok(decoded_string)
    }

    pub async fn save_valokda_config(&self) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let config = self.decode_config_valokda().await?;
        
        let timestamp = Local::now().format("%Y%m%d_%H%M%S");
        let filename = format!("wireguard_config_{}.conf", timestamp);
        
        self.save_config_to_file(&config, &filename).await
    }

    pub async fn get_warp(&self) -> Result<Value, Box<dyn std::error::Error>> {
        let url = format!("{}/warp", self.api);
        let body = json!({"selectedServices": [],"siteMode": "all","deviceType": "computer"});
        let client = reqwest::Client::new();
        let current_headers = self.get_headers_for(&self.api);
        let response = client
            .post(&url)
            .headers(current_headers)
            .json(&body)
            .send()
            .await?;
        let body = response.json().await?;

        Ok(body)
    }

    pub async fn decode_config(&self) -> Result<String, Box<dyn std::error::Error>> {
        let welcome_data = self.get_warp().await?;
        
        let config_base64 = welcome_data
            .get("content")
            .and_then(|content| content.get("configBase64"))
            .and_then(|config| config.as_str())
            .ok_or("configBase64 not found in response")?;
        
        let decoded_bytes = BASE64.decode(config_base64)?;
        let decoded_string = String::from_utf8(decoded_bytes)?;
        
        Ok(decoded_string)
    }

    pub async fn save_config_auto(&self) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let config = self.decode_config().await?;
        
        let timestamp = Local::now().format("%Y%m%d_%H%M%S");
        let filename = format!("wireguard_config_{}.conf", timestamp);
        
        self.save_config_to_file(&config, &filename).await
    }


    pub async fn save_config_to_file(
        &self, 
        config: &str, 
        filename: &str
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let path = Path::new(filename);
        fs::write(path, config)?;
        Ok(path.canonicalize()?)

}
}
