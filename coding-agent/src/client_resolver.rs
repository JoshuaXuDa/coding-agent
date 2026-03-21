//! Custom client resolver for BigModel Coding endpoint
//!
//! This module provides a custom genai client that forces GLM models
//! to use the BigModel Coding endpoint instead of the default ZAI endpoint.

use genai::{Client, ModelIden};
use genai::adapter::AdapterKind;

/// Create a custom client that forces GLM models to use the BigModel Coding endpoint
///
/// The genai library auto-detects model names starting with "glm" and routes them
/// to the ZAI adapter. The BigModel Coding endpoint is OpenAI-compatible and
/// requires streaming mode for proper billing with the coding plan.
pub fn create_bigmodel_coding_client() -> Client {
    // Clear any conflicting base URLs - we'll set the endpoint in the resolver
    std::env::remove_var("OPENAI_BASE_URL");
    std::env::remove_var("ANTHROPIC_BASE_URL");

    // Enable debug logging for genai to see actual HTTP requests
    std::env::set_var("RUST_LOG", "genai=debug,genai::client=debug");

    Client::builder()
        .with_service_target_resolver_fn(|mut service_target: genai::ServiceTarget| {
            let model_name = service_target.model.model_name.as_str();

            eprintln!("🔧 [RESOLVER] Model: {}", model_name);
            eprintln!("🔧 [RESOLVER] Original adapter: {:?}", service_target.model.adapter_kind);
            eprintln!("🔧 [RESOLVER] Original endpoint: {:?}", service_target.endpoint);

            // Check if this is a GLM model
            if model_name.starts_with("glm") {
                // Change to OpenAI adapter since BigModel Coding API is OpenAI-compatible
                service_target.model.adapter_kind = genai::adapter::AdapterKind::OpenAI;

                // Override endpoint to BigModel Coding API
                // The full path should be: https://open.bigmodel.cn/api/coding/paas/v4/chat/completions
                // OpenAI adapter will append /chat/completions, so we set base to /v4/
                let coding_endpoint = "https://open.bigmodel.cn/api/coding/paas/v4/";
                service_target.endpoint = genai::resolver::Endpoint::from_static(coding_endpoint);
                eprintln!("🔧 [RESOLVER] Set endpoint to: {}", coding_endpoint);
                eprintln!("🔧 [RESOLVER] Changed adapter to OpenAI, endpoint to: {}", coding_endpoint);

                // Ensure auth is set from OPENAI_API_KEY
                if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
                    let api_key_masked = format!("{}...{}", &api_key[..8], &api_key[api_key.len()-4..]);
                    eprintln!("🔧 [RESOLVER] Using OPENAI_API_KEY: {}", api_key_masked);
                    service_target.auth = genai::resolver::AuthData::from_single(api_key.as_str());
                }
            }

            eprintln!("🔧 [RESOLVER] Final endpoint: {:?}", service_target.endpoint);
            eprintln!("🔧 [RESOLVER] Final adapter: {:?}", service_target.model.adapter_kind);
            Ok(service_target)
        })
        .build()
}