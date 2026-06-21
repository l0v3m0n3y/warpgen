use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, HOST, CONNECTION, ACCEPT_LANGUAGE, USER_AGENT, ORIGIN, REFERER};
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};
use std::fs;
use reqwest::{Client, redirect::Policy}; 
use std::path::{Path, PathBuf};
use chrono::{Local};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use scraper::{Html, Selector};


#[derive(Debug, Clone, Default)]
pub struct ValokdaAwg2Params {
    pub mode: String,
    pub template: String,
    pub dns: String,
    pub link: String,
    pub cps: String,
    pub port: String,
}

impl ValokdaAwg2Params {
    pub fn default_awg2() -> Self {
        Self {
            mode: "awg2".to_string(),
            template: "warp_amnezia_awg2".to_string(),
            dns: "cloudflare".to_string(),
            link: "1".to_string(),
            cps: "auto".to_string(),
            port: "4500".to_string(),
        }
    }
    
    pub fn new(mode: &str, template: &str, dns: &str, link: &str, cps: &str, port: &str) -> Self {
        Self {
            mode: mode.to_string(),
            template: template.to_string(),
            dns: dns.to_string(),
            link: link.to_string(),
            cps: cps.to_string(),
            port: port.to_string(),
        }
    }
    
    // Метод для преобразования в Query параметры
    pub fn to_query_params(&self) -> Vec<(String, String)> {
        vec![
            ("mode".to_string(), self.mode.clone()),
            ("template".to_string(), self.template.clone()),
            ("dns".to_string(), self.dns.clone()),
            ("link".to_string(), self.link.clone()),
            ("cps".to_string(), self.cps.clone()),
            ("port".to_string(), self.port.clone()),
        ]
    }
    
    // Удобные методы для изменения параметров
    pub fn with_dns(mut self, dns: &str) -> Self {
        self.dns = dns.to_string();
        self
    }
    
    pub fn with_port(mut self, port: &str) -> Self {
        self.port = port.to_string();
        self
    }
    
    pub fn with_mode(mut self, mode: &str) -> Self {
        self.mode = mode.to_string();
        self
    }
    
    pub fn with_template(mut self, template: &str) -> Self {
        self.template = template.to_string();
        self
    }
    
    pub fn with_link(mut self, link: &str) -> Self {
        self.link = link.to_string();
        self
    }
    
    pub fn with_cps(mut self, cps: &str) -> Self {
        self.cps = cps.to_string();
        self
    }
    
    // Предустановленные конфигурации
    pub fn with_cloudflare() -> Self {
        Self::default_awg2()
    }
    
    pub fn with_google() -> Self {
        Self::new("awg2", "warp_amnezia_awg2", "google", "1", "auto", "4500")
    }
    
    pub fn with_quad9() -> Self {
        Self::new("awg2", "warp_amnezia_awg2", "quad9", "1", "auto", "4500")
    }
}

pub struct WarpGen {
    api: String,
    api_valokda: String,
    warpgen_api: String,
    headers: Arc<Mutex<HeaderMap>>,
}

impl WarpGen {
    pub fn new() -> Self {
        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT, HeaderValue::from_static("*/*"));
        headers.insert(CONNECTION, HeaderValue::from_static("keep-alive"));
        headers.insert(ACCEPT_LANGUAGE, HeaderValue::from_static("en-US,en;q=0.9"));
        headers.insert(USER_AGENT, HeaderValue::from_static("Mozilla/5.0 (Macintosh; Intel Mac OS X 15_7_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/26.0 Safari/605.1.15"));

        Self {
            api_valokda: "https://valokda-amnezia.vercel.app/api".to_string(),
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
    

    pub async fn get_warp_valokda_with_params(&self, params: &ValokdaAwg2Params) -> Result<Value, Box<dyn std::error::Error>> {
        let url = format!("{}/warp", self.api_valokda);
        let client = reqwest::Client::new();
        let current_headers = self.get_headers_for(&self.api_valokda);
        
        let query_params = params.to_query_params();
        
        let response = client
            .get(&url)
            .headers(current_headers)
            .query(&query_params)
            .send()
            .await?;
        
        let body = response.json().await?;
        Ok(body)
    }
    
    pub async fn get_warp_valokda_awg2(&self) -> Result<Value, Box<dyn std::error::Error>> {
        let params = ValokdaAwg2Params::default_awg2();
        self.get_warp_valokda_with_params(&params).await
    }
    
    pub async fn decode_config_valokda_awg2_custom(
        &self, 
        params: &ValokdaAwg2Params
    ) -> Result<String, Box<dyn std::error::Error>> {
        let welcome_data = self.get_warp_valokda_with_params(params).await?;
        
        let config_base64 = welcome_data["content"].as_str();
        
        let decoded_bytes = BASE64.decode(config_base64.unwrap())?;
        let decoded_string = String::from_utf8(decoded_bytes)?;
        
        Ok(decoded_string)
    }
    
    pub async fn decode_config_valokda_awg2(&self) -> Result<String, Box<dyn std::error::Error>> {
        let params = ValokdaAwg2Params::default_awg2();
        self.decode_config_valokda_awg2_custom(&params).await
    }
    
    pub async fn save_valokda_awg2_config_custom(
        &self, 
        params: &ValokdaAwg2Params,
        prefix: Option<&str>
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let config = self.decode_config_valokda_awg2_custom(params).await?;
        
        let timestamp = Local::now().format("%Y%m%d_%H%M%S");
        let prefix_str = prefix.unwrap_or("wireguard_valokda_awg2");
        let filename = format!("{}_{}.conf", prefix_str, timestamp);
        
        self.save_config_to_file(&config, &filename).await
    }
    
    pub async fn save_valokda_awg2_config(&self) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let params = ValokdaAwg2Params::default_awg2();
        self.save_valokda_awg2_config_custom(&params, None).await
    }

    pub async fn get_valokda_awg2_configs_custom(
        &self, 
        params: &ValokdaAwg2Params
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let response = self.get_warp_valokda_with_params(params).await?;
        let mut configs = Vec::new();
        
        if let Some(configs_array) = response["configs"].as_array() {
            for config_item in configs_array {
                if let Some(content) = config_item["content"].as_str() {
                    let decoded_bytes = BASE64.decode(content)?;
                    let decoded_string = String::from_utf8(decoded_bytes)?;
                    configs.push(decoded_string);
                }
            }
        } else if let Some(content) = response["content"].as_str() {
            let decoded_bytes = BASE64.decode(content)?;
            let decoded_string = String::from_utf8(decoded_bytes)?;
            configs.push(decoded_string);
        }
        
        Ok(configs)
    }

    pub async fn save_valokda_awg2_configs_custom(
        &self, 
        params: &ValokdaAwg2Params,
        prefix: Option<&str>
    ) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
        let configs = self.get_valokda_awg2_configs_custom(params).await?;
        let mut saved_paths = Vec::new();
        
        let prefix_str = prefix.unwrap_or("wireguard_valokda_awg2");
        
        for (index, config) in configs.iter().enumerate() {
            let timestamp = Local::now().format("%Y%m%d_%H%M%S");
            let filename = format!("{}_{}_{}.conf", prefix_str, timestamp, index + 1);
            let path = self.save_config_to_file(config, &filename).await?;
            saved_paths.push(path);
        }
        
        Ok(saved_paths)
    }

    pub async fn save_valokda_awg2_configs(&self) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
        let params = ValokdaAwg2Params::default_awg2();
        self.save_valokda_awg2_configs_custom(&params, None).await
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
