use vae::api::{Router, handlers};
use vae::core::agent::Lilith;
use vae::utils::{logger, config};
use actix_web::{test, web, App};
use serde_json::{json, Value};
use std::error::Error;

#[actix_web::test]
async fn test_health_endpoint() -> Result<(), Box<dyn Error>> {
    let config = config::load_test_config()?;
    let logger = logger::setup_test_logger();
    
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(config.clone()))
            .service(handlers::health::health_check)
    ).await;
    
    let req = test::TestRequest::get().uri("/health").to_request();
    let resp = test::call_service(&app, req).await;
    
    assert!(resp.status().is_success());
    
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "healthy");
    
    Ok(())
}

#[actix_web::test]
async fn test_agent_completion() -> Result<(), Box<dyn Error>> {
    let config = config::load_test_config()?;
    let logger = logger::setup_test_logger();
    let lilith = web::Data::new(Lilith::new(&config, logger.clone()));
    
    let app = test::init_service(
        App::new()
            .app_data(lilith.clone())
            .service(handlers::agent::complete)
    ).await;
    
    let payload = json!({
        "messages": [
            {"role": "user", "content": "Hello, Lilith!"}
        ]
    });
    
    let req = test::TestRequest::post()
        .uri("/v1/agent/complete")
        .set_json(&payload)
        .to_request();
        
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    
    let body: Value = test::read_body_json(resp).await;
    assert!(!body["content"].as_str().unwrap().is_empty());
    
    Ok(())
}

#[actix_web::test]
async fn test_agent_streaming() -> Result<(), Box<dyn Error>> {
    let config = config::load_test_config()?;
    let logger = logger::setup_test_logger();
    let lilith = web::Data::new(Lilith::new(&config, logger.clone()));
    
    let app = test::init_service(
        App::new()
            .app_data(lilith.clone())
            .service(handlers::agent::stream)
    ).await;
    
    let payload = json!({
        "messages": [
            {"role": "user", "content": "Tell me a story"}
        ]
    });
    
    let req = test::TestRequest::post()
        .uri("/v1/agent/stream")
        .set_json(&payload)
        .to_request();
        
    let mut resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    
    let mut bytes = web::BytesMut::new();
    while let Some(chunk) = test::load_stream(resp.take_body()).await? {
        bytes.extend_from_slice(&chunk);
    }
    
    assert!(!bytes.is_empty());
    
    Ok(())
}

#[actix_web::test]
async fn test_authentication() -> Result<(), Box<dyn Error>> {
    let config = config::load_test_config()?;
    let logger = logger::setup_test_logger();
    let lilith = web::Data::new(Lilith::new(&config, logger.clone()));
    
    let app = test::init_service(
        App::new()
            .app_data(lilith.clone())
            .wrap(middleware::auth::Auth::new(&config))
            .service(handlers::agent::complete)
    ).await;
    
    // Test without token
    let req = test::TestRequest::post()
        .uri("/v1/agent/complete")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
    
    // Test with invalid token
    let req = test::TestRequest::post()
        .uri("/v1/agent/complete")
        .insert_header(("Authorization", "Bearer invalid_token"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
    
    // Test with valid token
    let token = generate_test_token(&config)?;
    let req = test::TestRequest::post()
        .uri("/v1/agent/complete")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({"messages": [{"role": "user", "content": "test"}]}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    
    Ok(())
}

#[actix_web::test]
async fn test_error_handling() -> Result<(), Box<dyn Error>> {
    let config = config::load_test_config()?;
    let logger = logger::setup_test_logger();
    let lilith = web::Data::new(Lilith::new(&config, logger.clone()));
    
    let app = test::init_service(
        App::new()
            .app_data(lilith.clone())
            .service(handlers::agent::complete)
    ).await;
    
    // Test invalid JSON
    let req = test::TestRequest::post()
        .uri("/v1/agent/complete")
        .set_payload("invalid json")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);
    
    // Test empty messages
    let req = test::TestRequest::post()
        .uri("/v1/agent/complete")
        .set_json(json!({"messages": []}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);
    
    Ok(())
}

#[actix_web::test]
async fn test_rate_limiting() -> Result<(), Box<dyn Error>> {
    let config = config::load_test_config()?;
    let logger = logger::setup_test_logger();
    let lilith = web::Data::new(Lilith::new(&config, logger.clone()));
    
    let app = test::init_service(
        App::new()
            .app_data(lilith.clone())
            .wrap(middleware::ratelimit::RateLimit::new(10, 60)) // 10 requests per minute
            .service(handlers::agent::complete)
    ).await;
    
    let payload = json!({
        "messages": [{"role": "user", "content": "test"}]
    });
    
    // Send requests until rate limit is hit
    for i in 0..15 {
        let req = test::TestRequest::post()
            .uri("/v1/agent/complete")
            .set_json(&payload)
            .to_request();
        let resp = test::call_service(&app, req).await;
        
        if i < 10 {
            assert!(resp.status().is_success());
        } else {
            assert_eq!(resp.status(), 429);
        }
    }
    
    Ok(())
}

#[actix_web::test]
async fn test_metrics_endpoint() -> Result<(), Box<dyn Error>> {
    let config = config::load_test_config()?;
    let logger = logger::setup_test_logger();
    
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(config.clone()))
            .service(handlers::metrics::get_metrics)
    ).await;
    
    let req = test::TestRequest::get().uri("/metrics").to_request();
    let resp = test::call_service(&app, req).await;
    
    assert!(resp.status().is_success());
    
    let body: Value = test::read_body_json(resp).await;
    assert!(body.get("requests_total").is_some());
    assert!(body.get("response_time_ms").is_some());
    assert!(body.get("errors_total").is_some());
    
    Ok(())
}

// Helper function to generate test JWT token
fn generate_test_token(config: &config::Config) -> Result<String, Box<dyn Error>> {
    use jsonwebtoken::{encode, EncodingKey, Header};
    
    let claims = json!({
        "sub": "test_user",
        "exp": chrono::Utc::now().timestamp() + 3600
    });
    
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(config.jwt_secret.as_bytes())
    )?;
    
    Ok(token)
}