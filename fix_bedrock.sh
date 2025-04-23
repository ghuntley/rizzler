#!/bin/bash

# Fix the AWS Bedrock provider implementation
sed -i 's/AIProviderError::ApiError/AIProviderError::ResponseError/g' src/providers/bedrock.rs
sed -i 's/aws_sdk_bedrockruntime::types::Blob::new/Blob::new/g' src/providers/bedrock.rs
sed -i 's/AIProviderError::ApiError(format("AWS Bedrock API error: {}", e))/AIProviderError::RequestError(format("AWS Bedrock API error: {}", e))/g' src/providers/bedrock.rs
sed -i 's/aws_config::from_env()/aws_config::defaults()/g' src/providers/bedrock.rs