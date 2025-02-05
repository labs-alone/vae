use vae::core::agent::{Lilith, Memory, AgentTrait};
use vae::core::llm::types::{Message, Response};
use vae::utils::{logger, config};
use tokio;
use std::error::Error;

#[tokio::test]
async fn test_lilith_initialization() -> Result<(), Box<dyn Error>> {
    let config = config::load_test_config()?;
    let logger = logger::setup_test_logger();
    
    let lilith = Lilith::new(&config, logger.clone());
    assert!(lilith.is_initialized());
    
    Ok(())
}

#[tokio::test]
async fn test_memory_management() -> Result<(), Box<dyn Error>> {
    let config = config::load_test_config()?;
    let logger = logger::setup_test_logger();
    let mut lilith = Lilith::new(&config, logger.clone());
    
    // Test memory storage
    let message = Message::new("user", "Test memory storage");
    lilith.memory.store(message.clone())?;
    
    // Test memory retrieval
    let retrieved = lilith.memory.get_recent(1)?;
    assert_eq!(retrieved.len(), 1);
    assert_eq!(retrieved[0].content, message.content);
    
    // Test memory cleanup
    lilith.memory.cleanup()?;
    assert!(lilith.memory.is_within_limits());
    
    Ok(())
}

#[tokio::test]
async fn test_llm_interaction() -> Result<(), Box<dyn Error>> {
    let config = config::load_test_config()?;
    let logger = logger::setup_test_logger();
    let mut lilith = Lilith::new(&config, logger.clone());
    
    // Test basic completion
    let response = lilith.process_message("Hello, Lilith!").await?;
    assert!(!response.content.is_empty());
    
    // Test streaming response
    let mut stream = lilith.process_message_stream("Tell me about VAE").await?;
    let mut responses = Vec::new();
    
    while let Some(chunk) = stream.next().await {
        responses.push(chunk?);
    }
    
    assert!(!responses.is_empty());
    
    Ok(())
}

#[tokio::test]
async fn test_error_handling() -> Result<(), Box<dyn Error>> {
    let config = config::load_test_config()?;
    let logger = logger::setup_test_logger();
    let mut lilith = Lilith::new(&config, logger.clone());
    
    // Test invalid input handling
    let result = lilith.process_message("").await;
    assert!(result.is_err());
    
    // Test memory overflow handling
    for i in 0..1000 {
        let message = Message::new("user", &format!("Test message {}", i));
        lilith.memory.store(message)?;
    }
    
    assert!(lilith.memory.is_within_limits());
    
    Ok(())
}

#[tokio::test]
async fn test_state_management() -> Result<(), Box<dyn Error>> {
    let config = config::load_test_config()?;
    let logger = logger::setup_test_logger();
    let mut lilith = Lilith::new(&config, logger.clone());
    
    // Test state persistence
    lilith.set_state("test_key", "test_value")?;
    let value = lilith.get_state("test_key")?;
    assert_eq!(value, Some("test_value".to_string()));
    
    // Test state cleanup
    lilith.clear_state()?;
    let value = lilith.get_state("test_key")?;
    assert_eq!(value, None);
    
    Ok(())
}

#[tokio::test]
async fn test_concurrent_processing() -> Result<(), Box<dyn Error>> {
    let config = config::load_test_config()?;
    let logger = logger::setup_test_logger();
    let lilith = Lilith::new(&config, logger.clone());
    
    // Test concurrent message processing
    let mut handles = Vec::new();
    
    for i in 0..5 {
        let mut agent = lilith.clone();
        let handle = tokio::spawn(async move {
            agent.process_message(&format!("Concurrent test {}", i)).await
        });
        handles.push(handle);
    }
    
    for handle in handles {
        let result = handle.await??;
        assert!(!result.content.is_empty());
    }
    
    Ok(())
}

#[tokio::test]
async fn test_metrics_collection() -> Result<(), Box<dyn Error>> {
    let config = config::load_test_config()?;
    let logger = logger::setup_test_logger();
    let mut lilith = Lilith::new(&config, logger.clone());
    
    // Process some messages
    for i in 0..5 {
        lilith.process_message(&format!("Test message {}", i)).await?;
    }
    
    // Check metrics
    let metrics = lilith.get_metrics()?;
    assert!(metrics.messages_processed > 0);
    assert!(metrics.average_response_time > 0.0);
    assert!(metrics.memory_usage > 0);
    
    Ok(())
}