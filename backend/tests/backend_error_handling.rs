use std::str::FromStr;
use reqwest::{Response, StatusCode};
use mockito::{mock, Matcher};
use reqwest::header::{HeaderMap, AUTHORIZATION};
use reqwest::Client;
use serde_json::json;
use std::env;
use std::error::Error;
use log::{error, info};
use serde_json::Value;
use httpmock::Method;


// Code for the backend Logic - Jesica PLEASE DO NOT TOUCH
pub async fn backend_translation_logic(
    source_code: &str,
    source_language: &str,
    target_language: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let api_key = env::var("GPT3_API_KEY").expect("GPT3_API_KEY must be set");
    let client = Client::new();
    let mut headers = HeaderMap::new();
    headers.insert(AUTHORIZATION, format!("Bearer {}", api_key).parse().unwrap());

    let prompt = format!(
        "Translate the following code from {} to {}: \n\n{}",
        source_language, target_language, source_code
    );

    let payload = json!({
        "model": "gpt-3.5-turbo-instruct",
        "prompt": prompt,
        "temperature": 0.5,
        "max_tokens": 1024
    });

    match client
        .post("https://api.openai.com/v1/completions")
        .headers(headers.clone())
        .json(&payload)
        .send()
        .await {
            Ok(response) => {
                match response.text().await {
                    Ok(response_body) => {
                        let response_json: Value = serde_json::from_str(&response_body)?;
                        let translated_text = response_json["choices"][0]["text"]
                            .as_str()
                            .ok_or("Failed to extract translated text")?
                            .to_string();
                        info!("Translation successful");
                        Ok(translated_text)
                    },
                    Err(err) => {
                        error!("Error reading response text: {}", err);
                        Err(Box::new(err))
                    }
                }
            },
            Err(err) => {
                if err.status().map_or(false, |s| s == reqwest::StatusCode::TOO_MANY_REQUESTS) {
                    error!("Rate Limit Exceeded: {}", err);
                    return Err("Rate Limit Exceeded".into());
                }
                error!("Error sending request: {}", err);
                Err(Box::new(err))
            }
        }
}

pub async fn translate_and_collect(
    source_code: &str,
    source_language: &str,
    target_language: &str,
) -> Result<Vec<String>, Box<dyn Error>> {
    let mut results = Vec::new();

    match backend_translation_logic(source_code, source_language, target_language).await {
        Ok(translated_text) => results.push(translated_text),
        Err(err) => {
            error!("Translation error: {}", err);
            return Err(err);
        }
    }

    Ok(results)
}

#[tokio::test]
async fn test_rate_limit_exceeded_error_logged() {
    // Arrange
    dotenv::dotenv().ok(); // Load the .env file

    let server = httpmock::MockServer::start();

    let _mock = server.mock(|when, then| {
        when.method(Method::POST)
            .path("/v1/completions")
            .header_exists("authorization")
            .body("{}");
        then.status(429).json_body(json!({"error": "Rate Limit Exceeded"}));
    });

    // Act
    let result = backend_translation_logic("source_code", "source_lang", "target_lang").await;

    // Assert
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().to_string(),
        "Rate Limit Exceeded".to_string()
    );
}