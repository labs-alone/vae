use vae::core::llm::{OpenAI, LLMTrait};
use vae::core::llm::types::{Message, Response, ModelConfig};
use vae::utils::{logger, config};
use tokio;
use std::error::Error;
use std::time::Duration;

#[tokio::test]
async fn test_openai_initialization() -> Result<(), Box<dyn Error>> {
    let config = config::load_test_config()?;
    let logger = logger::setup_test_logger();
    
    let llm = OpenAI::new(&config, logger.clone());
    assert!(llm.is_initialized());
    assert_eq!(llm.get_model(), "gpt-4");
    
    Ok(())
}

#[tokio::test]
async fn test_basic_completion() -> Result<(), Box<dyn Error>> {
    let config = config::load_test_config()?;
    let logger = logger::setup_test_logger();
    let llm = OpenAI::new(&config, logger.clone());
    
    let messages = vec![
        Message::new("system", "You are a helpful AI assistant."),
        Message::new("user", "Hello, how are you?")
    ];
    
    let response = llm.complete(messages).await?;
    assert!(!response.content.is_empty());
    assert_eq!(response.role, "assistant");
    assert!(response.usage.total_tokens > 0);
    
    Ok(())
}

#[tokio::test]
async fn test_streaming_completion() -> Result<(), Box<dyn Error>> {
    let config = config::load_test_config()?;
    let logger = logger::setup_test_logger();
    let llm = OpenAI::new(&config, logger.clone());
    
    let messages = vec![
        Message::new("system", "You are a helpful AI assistant."),
        Message::new("user", "Write a short story.")
    ];
    
    let mut stream = llm.complete_stream(messages).await?;
    let mut chunks = Vec::new();
    
    while let Some(chunk) = stream.next().await {
        chunks.push(chunk?);
    }
    
    assert!(!chunks.is_empty());
    assert!(chunks.iter().all(|c| !c.content.is_empty()));
    
    Ok(())
}

#[tokio::test]
async fn test_model_configuration() -> Result<(), Box<dyn Error>> {
    let config = config::load_test_config()?;
    let logger = logger::setup_test_logger();
    let mut llm = OpenAI::new(&config, logger.clone());
    
    // Test model configuration
    let model_config = ModelConfig {
        temperature: 0.7,
        max_tokens: 100,
        top_p: 1.0,
        frequency_penalty: 0.0,
        presence_penalty: 0.0,
    };
    
    llm.set_model_config(model_config.clone());
    assert_eq!(llm.get_model_config(), model_config);
    
    // Test completion with config
    let messages = vec![Message::new("user", "Generate a random number.")];
    let response = llm.complete(messages).await?;
    assert!(response.usage.total_tokens <= model_config.max_tokens);
    
    Ok(())
}

#[tokio::test]
async fn test_error_handling() -> Result<(), Box<dyn Error>> {
    let config = config::load_test_config()?;
    let logger = logger::setup_test_logger();
    let llm = OpenAI::new(&config, logger.clone());
    
    // Test empty messages
    let result = llm.complete(vec![]).await;
    assert!(result.is_err());
    
    // Test invalid API key
    let mut invalid_config = config.clone();
    invalid_config.openai_key = "invalid_key".to_string();
    let invalid_llm = OpenAI::new(&invalid_config, logger.clone());
    let result = invalid_llm.complete(vec![Message::new("user", "test")]).await;
    assert!(result.is_err());
    
    Ok(())
}

#[tokio::test]
async fn test_rate_limiting() -> Result<(), Box<dyn Error>> {
    let config = config::load_test_config()?;
    let logger = logger::setup_test_logger();
    let llm = OpenAI::new(&config, logger.clone());
    
    let message = Message::new("user", "Quick test.");
    let mut handles = Vec::new();
    
    // Send multiple concurrent requests
    for _ in 0..10 {
        let llm_clone = llm.clone();
        let messages = vec![message.clone()];
        let handle = tokio::spawn(async move {
            llm_clone.complete(messages).await
        });
        handles.push(handle);
    }
    
    // All requests should complete without rate limit errors
    for handle in handles {
        let result = handle.await?;
        assert!(result.is_ok());
    }
    
    Ok(())
}

#[tokio::test]
async fn test_timeout_handling() -> Result<(), Box<dyn Error>> {
    let mut config = config::load_test_config()?;
    config.timeout = Duration::from_millis(1); // Unreasonably short timeout
    let logger = logger::setup_test_logger();
    let llm = OpenAI::new(&config, logger.clone());
    
    let messages = vec![
        Message::new("user", "Write a very long story with lots of details.")
    ];
    
    let result = llm.complete(messages).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("timeout"));
    
    Ok(())
}

#[tokio::test]
async fn test_context_length() -> Result<(), Box<dyn Error>> {
    let config = config::load_test_config()?;
    let logger = logger::setup_test_logger();
    let llm = OpenAI::new(&config, logger.clone());
    
    // Create a message that's too long
    let long_text = "test ".repeat(4000);
    let messages = vec![Message::new("user", &long_text)];
    
    let result = llm.complete(messages).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("context length"));
    
    Ok(())
}

#[tokio::test]
async fn test_response_validation() -> Result<(), Box<dyn Error>> {
    let config = config::load_test_config()?;
    let logger = logger::setup_test_logger();
    let llm = OpenAI::new(&config, logger.clone());
    
    let messages = vec![Message::new("user", "Hello")];
    let response = llm.complete(messages).await?;
    
    // Validate response structure
    assert!(!response.content.is_empty());
    assert_eq!(response.role, "assistant");
    assert!(response.usage.total_tokens > 0);
    assert!(response.usage.prompt_tokens > 0);
    assert!(response.usage.completion_tokens > 0);
    assert!(!response.model.is_empty());
    
    Ok(())
}