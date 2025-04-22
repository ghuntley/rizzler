use rizzler_ai_resolver::{BedrockProvider, AIProvider};
use std::collections::HashMap;
use std::env;

#[test]
fn test_bedrock_provider_configuration() {
    // Skip test if AWS credentials not available
    if env::var("AWS_ACCESS_KEY_ID").is_err() || env::var("AWS_SECRET_ACCESS_KEY").is_err() {
        println!("Skipping Bedrock provider test because AWS credentials are not set");
        return;
    }

    // Create a mock environment with the necessary AWS Bedrock provider config
    let mut env_vars = HashMap::new();
    env_vars.insert("AWS_REGION".to_string(), "us-east-1".to_string());
    env_vars.insert("RIZZLER_BEDROCK_MODEL".to_string(), "anthropic.claude-3-sonnet-20240229-v1:0".to_string());

    // Initialize Bedrock provider
    let provider = BedrockProvider::new_with_config(env_vars);
    
    // Check provider configuration
    assert!(provider.is_available());
    assert_eq!(provider.name(), "AWS Bedrock");
}

// We don't test actual API calls since they would require real AWS credentials
// Instead, we mock the response in the implementation
#[test]
fn test_bedrock_provider_request_building() {
    // Skip test if AWS credentials not available
    if env::var("AWS_ACCESS_KEY_ID").is_err() || env::var("AWS_SECRET_ACCESS_KEY").is_err() {
        println!("Skipping Bedrock provider test because AWS credentials are not set");
        return;
    }

    // Create a mock environment with the necessary AWS Bedrock provider config
    let mut env_vars = HashMap::new();
    env_vars.insert("AWS_REGION".to_string(), "us-east-1".to_string());
    env_vars.insert("RIZZLER_BEDROCK_MODEL".to_string(), "anthropic.claude-3-sonnet-20240229-v1:0".to_string());

    // Initialize Bedrock provider
    let provider = BedrockProvider::new_with_config(env_vars);
    
    // Test creating a request for the Claude model
    let system_prompt = "You are a helpful assistant for resolving Git merge conflicts.";
    let user_prompt = "Please resolve this conflict:
<<<<<<< HEAD
user code
=======
their code
>>>>>>> branch";
    
    let request = provider.create_request(system_prompt, user_prompt);
    
    // Since we can't easily test the AWS-specific request directly, we test the internal structure
    // through our own accessor method (create_request should be implemented to return a testable value)
    assert!(request.contains("anthropic.claude-3-sonnet"));
    assert!(request.contains("You are a helpful assistant"));
    assert!(request.contains("Please resolve this conflict"));
}