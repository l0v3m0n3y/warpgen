use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, HOST, CONNECTION, ACCEPT_LANGUAGE, USER_AGENT, ORIGIN, REFERER,ACCEPT_ENCODING,CONTENT_TYPE};
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};
use std::fs;
use std::path::{Path, PathBuf};
use chrono::{Local, Utc};
use regex::Regex;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use std::collections::HashMap;
use rand::{distributions::Alphanumeric, Rng};
use tokio::time::sleep;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WarpConnectionInfo {
    pub private_key: String,
    pub public_key: String,
    pub address_v4: String,
    pub address_v6: String,
    pub dns: String,
    pub endpoint: String,
    pub port: u16,
    pub client_id: String,
    pub token: String,
    pub license: String,
    pub account_type: String,
    pub warp_plus: bool,
    pub expires: String,
    pub source: String, 
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WireGuardConfig {
    pub interface: InterfaceConfig,
    pub peer: PeerConfig,
    pub metadata: ConfigMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceConfig {
    pub private_key: String,
    pub addresses: Vec<String>,
    pub dns: Vec<String>,
    pub mtu: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerConfig {
    pub public_key: String,
    pub endpoint: String,
    pub allowed_ips: Vec<String>,
    pub persistent_keepalive: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigMetadata {
    pub source: String,
    pub created_at: String,
    pub warp_plus: bool,
    pub expires_at: Option<String>,
}

pub struct WarpGen {
    api: String,
    api_valokda: String,
    api_dev: String,
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
            api_dev: "https://warp-generation.vercel.app".to_string(),
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
    
    pub async fn get_key(&self) -> Result<String, Box<dyn std::error::Error>> {
        let url = format!("{}/keys", self.api_dev);
        let mut current_headers = self.get_headers_for(&self.api_dev);
        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .build()?;
    
        
        let response = client
            .get(url)
            .headers(current_headers.clone())
            .send()
            .await?;
        
        let response_text = response.text().await?;
        let re = Regex::new(r"PublicKey:\s*([^\n]+)")?;
        if let Some(caps) = re.captures(&response_text) {
            let public_key = caps[1].trim().to_string();
            Ok(public_key)
        } else {
            Err("PublicKey not found in response".into())
        }
    }

    fn generate_random_id(&self) -> String {
        rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(22)
            .map(char::from)
            .collect()
    }
    
    fn generate_fcm_token(&self, install_id: &str) -> String {
        let suffix: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(90)
            .map(char::from)
            .collect();
        
        format!("{}:APA91b{}", install_id, suffix)
    }
    pub async fn get_warp_workers(&self, locale: &str) -> Result<Value, Box<dyn std::error::Error>> {
        let max_retries = 5;
        let mut attempt = 0;
        let mut current_headers = self.get_headers_for(&self.api_dev);
        current_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        current_headers.insert("cf-client-version", HeaderValue::from_static("a-6.10-2158"));
        loop {
            attempt += 1;
            println!("🔄 Попытка {}/{} получить конфиг Warp Workers...", attempt, max_retries);
            
            let key = self.get_key().await?;
            let url = format!("{}/wg", self.api_dev);
            let install_id = self.generate_random_id();
            let serial_number = install_id.clone(); 
            let fcm_token = self.generate_fcm_token(&install_id);
        
            let tos = Utc::now().to_rfc3339();
            let body = json!({
            "key": key,
            "install_id": install_id,
            "fcm_token": fcm_token,
            "tos": tos,
            "model": "PC",
            "serial_number": serial_number,
            "locale": locale
            });
            
            let client = reqwest::Client::builder()
                .danger_accept_invalid_certs(true)
                .build()?;
            
            match client
                .post(&url)
                .headers(current_headers.clone())
                .json(&body)
                .send()
                .await
            {
                Ok(response) => {
                    match response.status() {
                        reqwest::StatusCode::OK => {
                            match response.json::<Value>().await {
                                Ok(body) => {
                                    println!("✅ Конфиг успешно получен!");
                                    return Ok(body);
                                }
                                Err(e) => {
                                    if attempt >= max_retries {
                                        return Err(format!("Ошибка парсинга JSON после {} попыток: {}", max_retries, e).into());
                                    }
                                    println!("⚠️ Ошибка парсинга JSON: {}. Повтор через {} сек...", e, 2u64.pow(attempt));
                                    sleep(Duration::from_secs(2u64.pow(attempt))).await;
                                    continue;
                                }
                            }
                        }
                        reqwest::StatusCode::TOO_MANY_REQUESTS => {
                            if attempt >= max_retries {
                                return Err("Слишком много запросов (429). Попробуйте позже.".into());
                            }
                            let wait_time = 5u64.pow(attempt);
                            println!("⚠️ Получен статус 429 (Too Many Requests). Ждем {} сек...", wait_time);
                            sleep(Duration::from_secs(wait_time)).await;
                            continue;
                        }
                        status => {
                            if attempt >= max_retries {
                                return Err(format!("Сервер вернул статус {} после {} попыток", status, max_retries).into());
                            }
                            let wait_time = 2u64.pow(attempt);
                            println!("⚠️ Неожиданный статус: {}. Повтор через {} сек...", status, wait_time);
                            
                            // Пробуем получить текст ошибки для диагностики
                            if let Ok(error_text) = response.text().await {
                                println!("📝 Текст ошибки: {}", error_text);
                            }
                            
                            sleep(Duration::from_secs(wait_time)).await;
                            continue;
                        }
                    }
                }
                Err(e) => {
                    if attempt >= max_retries {
                        return Err(format!("Сетевая ошибка после {} попыток: {}", max_retries, e).into());
                    }
                    let wait_time = 2u64.pow(attempt);
                    println!("⚠️ Сетевая ошибка: {}. Повтор через {} сек...", e, wait_time);
                    sleep(Duration::from_secs(wait_time)).await;
                    continue;
                }
            }
        }
    }

    // Извлечение информации из Warp Workers ответа
    pub async fn print_warp_workers_info(&self, locale: &str) -> Result<WarpConnectionInfo, Box<dyn std::error::Error>> {
        let response = self.get_warp_workers(&locale).await?;
        
        let endpoint_host = response["config"]["peers"][0]["endpoint"]["host"]
            .as_str()
            .ok_or("Missing endpoint host")?;
        
        let (endpoint, port_str) = endpoint_host.split_once(':')
            .ok_or("Invalid endpoint format")?;
        let port = port_str.parse::<u16>()?;
        
        Ok(WarpConnectionInfo {
            private_key: response["key"]
                .as_str()
                .ok_or("Missing private key")?
                .to_string(),
            
            public_key: response["config"]["peers"][0]["public_key"]
                .as_str()
                .ok_or("Missing public key")?
                .to_string(),
            
            address_v4: response["config"]["interface"]["addresses"]["v4"]
                .as_str()
                .ok_or("Missing IPv4 address")?
                .to_string(),
            
            address_v6: response["config"]["interface"]["addresses"]["v6"]
                .as_str()
                .ok_or("Missing IPv6 address")?
                .to_string(),
            
            dns: response["config"]["services"]["http_proxy"]
                .as_str()
                .ok_or("Missing DNS")?
                .split(':')
                .next()
                .unwrap_or("1.1.1.1")
                .to_string(),
            
            endpoint: endpoint.to_string(),
            port,
            
            client_id: response["config"]["client_id"]
                .as_str()
                .unwrap_or("")
                .to_string(),
            
            token: response["token"]
                .as_str()
                .unwrap_or("")
                .to_string(),
            
            license: response["account"]["license"]
                .as_str()
                .unwrap_or("")
                .to_string(),
            
            account_type: response["account"]["account_type"]
                .as_str()
                .unwrap_or("free")
                .to_string(),
            
            warp_plus: response["account"]["warp_plus"]
                .as_bool()
                .unwrap_or(false),
            
            expires: response["account"]["ttl"]
                .as_str()
                .unwrap_or("")
                .to_string(),
            
            source: "Warp Workers".to_string(),
        })
    }

    pub fn generate_wireguard_string(info: &WarpConnectionInfo) -> String {
        let mut config = String::new();
        
        config.push_str("[Interface]\n");
        config.push_str(&format!("PrivateKey = {}\n", info.private_key));
        config.push_str(&format!("Address = {}\n", info.address_v4));
        config.push_str(&format!("Address = {}\n", info.address_v6));
        config.push_str(&format!("DNS = {}\n", info.dns));
        config.push_str("MTU = 1280\n\n");
        
        config.push_str("Jmax = 70\n");
        config.push_str("Jmin = 40\n");
        config.push_str("S1 = 15\n");
        config.push_str("S2 = 91\n");
        config.push_str("H1 = 1\n");
        config.push_str("H2 = 2\n");
        config.push_str("H3 = 3\n");
        config.push_str("H4 = 4\n\n");
        
        config.push_str("[Peer]\n");
        config.push_str(&format!("PublicKey = {}\n", info.public_key));
        config.push_str(&format!("Endpoint = {}:{}\n", info.endpoint, info.port));
        config.push_str("AllowedIPs = 0.0.0.0/0\n");
        config.push_str("AllowedIPs = ::/0\n");
        config.push_str("PersistentKeepalive = 25\n");
        
        config
    }

    pub async fn save_warp_workers_config(&self, locale: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let info = self.print_warp_workers_info(&locale).await?;
        let config_str = Self::generate_wireguard_string(&info);
        
        let timestamp = Local::now().format("%Y%m%d_%H%M%S");
        let filename = format!("warp_workers_{}.conf", timestamp);
        
        self.save_config_to_file(&config_str, &filename).await
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
        let filename = format!("wireguard_valokda_{}.conf", timestamp);
        
        self.save_config_to_file(&config, &filename).await
    }
    
    pub async fn get_warp(&self) -> Result<Value, Box<dyn std::error::Error>> {
        let url = format!("{}/warp", self.api);
        let body = json!({
            "selectedServices": [],
            "siteMode": "all",
            "deviceType": "computer"
        });
        
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
        let filename = format!("wireguard_generator_{}.conf", timestamp);
        
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
