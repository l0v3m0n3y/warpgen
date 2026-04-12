use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, HOST, CONNECTION, ACCEPT_LANGUAGE, USER_AGENT, ORIGIN, REFERER,CONTENT_TYPE};
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};
use std::fs;
use reqwest::{Client, redirect::Policy}; 
use std::path::{Path, PathBuf};
use chrono::{Local, Utc};
use regex::Regex;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use scraper::{Html, Selector};
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
    pub client_id: String,
    pub token: String,
    pub license: String,
    pub account_type: String,
    pub warp_plus: bool,
    pub expires: String,
    pub source: String, 
}



pub struct WarpGen {
    api: String,
    api_valokda: String,
    warp_portal: String,
    warpgen_api: String,
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
            warp_portal: "https://warp-vless.vercel.app/api".to_string(),
            warpgen_api: "https://warpgen.net".to_string(),
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
    
    pub async fn generate_warpgen_net(&self) -> Result<String, Box<dyn std::error::Error>> {
        let url = format!("{}/genconfig", self.warpgen_api);
        let client = Client::builder()
        .redirect(Policy::none()) 
        .build()?;
        let params = [("mode", "awg1.5")]; 
        let current_headers = self.get_headers_for(&self.warpgen_api);
        let response = client
                .post(&url)
                .headers(current_headers.clone())
                .form(&params)
                .send()
                .await?;
        let body = response.text().await?;
        let document = Html::parse_document(&body);
        let selector = Selector::parse("a[href^='/result/']").unwrap();
        if let Some(link) = document.select(&selector).next() {
            let href = link.value().attr("href").unwrap();
            let id = href.trim_start_matches("/result/").to_string();
            Ok(id)
        }else {
            Err("Result link not found in response".into())
        }
    }

    pub async fn warpgen_string(&self,id: &str) -> Result<String, Box<dyn std::error::Error>> {
        let url = format!("{}/sendconfig/{}?", self.warpgen_api,id);

        let  current_headers = self.headers.lock().unwrap().clone();
        let client = reqwest::Client::new();
    
        
        let response = client
            .get(url)
            .headers(current_headers.clone())
            .send()
            .await?;
        
        let response_text: String = response.text().await?;
        Ok(response_text)
    }
        
    pub async fn save_warpgen_config(&self) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let id = self.generate_warpgen_net().await?;
        let config_str = self.warpgen_string(&id).await?;
        
        let timestamp = Local::now().format("%Y%m%d_%H%M%S");
        let filename = format!("warp_warpgen_{}.conf", timestamp);
        
        self.save_config_to_file(&config_str, &filename).await
    }
    
    pub async fn get_key(&self) -> Result<String, Box<dyn std::error::Error>> {
        let url = format!("{}/keys", self.api_dev);
        let  current_headers = self.get_headers_for(&self.api_dev);
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
        let locale_prefix = locale.split('_').next().unwrap_or("ee");
        let custom_endpoint = format!("{}.tribukvy.ltd:955", locale_prefix);
        
        
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
            
            endpoint: custom_endpoint.to_string(),
            
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
        config.push_str("H4 = 4\n");
        config.push_str("I1 = <b0xce000000010897a297ecc34cd6dd000044d0ec2e2e1ea2991f467ace4222129b5a098823784694b4897b9986ae0b7280135fa85e196d9ad980b150122129ce2a9379531b0fd3e871ca5fdb883c369832f730e272d7b8b74f393f9f0fa43f11e510ecb2219a52984410c204cf875585340c62238e14ad04dff382f2c200e0ee22fe743b9c6b8b043121c5710ec289f471c91ee414fca8b8be8419ae8ce7ffc53837f6ade262891895f3f4cecd31bc93ac5599e18e4f01b472362b8056c3172b513051f8322d1062997ef4a383b01706598d08d48c221d30e74c7ce000cdad36b706b1bf9b0607c32ec4b3203a4ee21ab64df336212b9758280803fcab14933b0e7ee1e04a7becce3e2633f4852585c567894a5f9efe9706a151b615856647e8b7dba69ab357b3982f554549bef9256111b2d67afde0b496f16962d4957ff654232aa9e845b61463908309cfd9de0a6abf5f425f577d7e5f6440652aa8da5f73588e82e9470f3b21b27b28c649506ae1a7f5f15b876f56abc4615f49911549b9bb39dd804fde182bd2dcec0c33bad9b138ca07d4a4a1650a2c2686acea05727e2a78962a840ae428f55627516e73c83dd8893b02358e81b524b4d99fda6df52b3a8d7a5291326e7ac9d773c5b43b8444554ef5aea104a738ed650aa979674bbed38da58ac29d87c29d387d80b526065baeb073ce65f075ccb56e47533aef357dceaa8293a523c5f6f790be90e4731123d3c6152a70576e90b4ab5bc5ead01576c68ab633ff7d36dcde2a0b2c68897e1acfc4d6483aaaeb635dd63c96b2b6a7a2bfe042f6aed82e5363aa850aace12ee3b1a93f30d8ab9537df483152a5527faca21efc9981b304f11fc95336f5b9637b174c5a0659e2b22e159a9fed4b8e93047371175b1d6d9cc8ab745f3b2281537d1c75fb9451871864efa5d184c38c185fd203de206751b92620f7c369e031d2041e152040920ac2c5ab5340bfc9d0561176abf10a147287ea90758575ac6a9f5ac9f390d0d5b23ee12af583383d994e22c0cf42383834bcd3ada1b3825a0664d8f3fb678261d57601ddf94a8a68a7c273a18c08aa99c7ad8c6c42eab67718843597ec9930457359dfdfbce024afc2dcf9348579a57d8d3490b2fa99f278f1c37d87dad9b221acd575192ffae1784f8e60ec7cee4068b6b988f0433d96d6a1b1865f4e155e9fe020279f434f3bf1bd117b717b92f6cd1cc9bea7d45978bcc3f24bda631a36910110a6ec06da35f8966c9279d130347594f13e9e07514fa370754d1424c0a1545c5070ef9fb2acd14233e8a50bfc5978b5bdf8bc1714731f798d21e2004117c61f2989dd44f0cf027b27d4019e81ed4b5c31db347c4a3a4d85048d7093cf16753d7b0d15e078f5c7a5205dc2f87e330a1f716738dce1c6180e9d02869b5546f1c4d2748f8c90d9693cba4e0079297d22fd61402dea32ff0eb69ebd65a5d0b687d87e3a8b2c42b648aa723c7c7daf37abcc4bb85caea2ee8f55bec20e913b3324ab8f5c3304f820d42ad1b9f2ffc1a3af9927136b4419e1e579ab4c2ae3c776d293d397d575df181e6cae0a4ada5d67ecea171cca3288d57c7bbdaee3befe745fb7d634f70386d873b90c4d6c6596bb65af68f9e5121e67ebf0d89d3c909ceedfb32ce9575a7758ff080724e1ab5d5f43074ecb53a479af21ed03d7b6899c36631c0166f9d47e5e1d4528a5d3d3f744029c4b1c190cbfbad06f5f83f7ad0429fa9a2719c56ffe3783460e166de2d8>\n\n");
        
        config.push_str("[Peer]\n");
        config.push_str(&format!("PublicKey = {}\n", info.public_key));
        config.push_str(&format!("Endpoint = {}\n",info.endpoint ));
        config.push_str("AllowedIPs = 0.0.0.0/0\n");
        
        config
    }

    pub async fn save_warp_workers_config(&self, locale: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let info = self.print_warp_workers_info(&locale).await?;
        let config_str = Self::generate_wireguard_string(&info);
        
        let timestamp = Local::now().format("%Y%m%d_%H%M%S");
        let filename = format!("warp_workers_{}.conf", timestamp);
        
        self.save_config_to_file(&config_str, &filename).await
    }
    
    pub async fn get_warp_portal(&self) -> Result<Value, Box<dyn std::error::Error>> {
        let url = format!("{}/warp", self.warp_portal);
        let client = reqwest::Client::new();
        let body = json!({
  "selectedServices": [],
  "siteMode": "all",
  "deviceType": "phone",
  "selectedDns": "1.1.1.1",
  "amneziaMode": "default"
});
        let current_headers = self.get_headers_for(&self.warp_portal);
        let response = client
                .post(&url)
                .headers(current_headers.clone())
                .json(&body)
                .send()
                .await?;
        let body = response.json().await?;

        Ok(body)
    }

    pub async fn decode_config_portal(&self) -> Result<String, Box<dyn std::error::Error>> {
        let welcome_data = self.get_warp_portal().await?;
        
        let config_base64 = welcome_data.get("content")
            .and_then(|content| content.get("configBase64"))
            .and_then(|config| config.as_str())
            .ok_or("configBase64 not found in response")?;
        
        let decoded_bytes = BASE64.decode(config_base64)?;
        let decoded_string = String::from_utf8(decoded_bytes)?;
        
        Ok(decoded_string)
    }

    pub async fn save_portal_config(&self) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let config = self.decode_config_portal().await?;
        
        let timestamp = Local::now().format("%Y%m%d_%H%M%S");
        let filename = format!("wireguard_portal_{}.conf", timestamp);
        
        self.save_config_to_file(&config, &filename).await
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
