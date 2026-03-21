use genai::Client;
use genai::resolver::AuthData;
use genai::chat::{ChatRequest, ChatResponse};

#[tokio::main]
async fn main() {
    // Clear environment variables
    std::env::remove_var("OPENAI_BASE_URL");
    std::env::remove_var("ANTHROPIC_BASE_URL");

    let api_key = std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not set");
    eprintln!("API Key: {}...", &api_key[..20]);

    // Test 1: Direct client creation with endpoint
    eprintln!("\n=== Test 1: Client with custom resolver ===");
    let client = Client::builder()
        .with_service_target_resolver_fn(|mut service_target: genai::ServiceTarget| {
            eprintln!("Resolver: Original endpoint: {:?}", service_target.endpoint);
            service_target.endpoint =
                genai::resolver::Endpoint::from_static("https://open.bigmodel.cn/api/coding/paas/v4");
            eprintln!("Resolver: New endpoint: {:?}", service_target.endpoint);
            Ok(service_target)
        })
        .build();

    let req = ChatRequest::default()
        .with_model("glm-4.7")
        .with_system("You are a helpful assistant.")
        .with_user("Say hi in 5 words");

    match client.exec(&req, Some(&AuthData::from_single(api_key.as_str()))).await {
        Ok(response) => eprintln!("Success: {:?}", response),
        Err(e) => eprintln!("Error: {}", e),
    }

    // Test 2: Using environment variable
    eprintln!("\n=== Test 2: Client with OPENAI_BASE_URL ===");
    std::env::set_var("OPENAI_BASE_URL", "https://open.bigmodel.cn/api/coding/paas/v4");
    let client2 = Client::builder().build();

    let req2 = ChatRequest::default()
        .with_model("glm-4.7")
        .with_system("You are a helpful assistant.")
        .with_user("Say hi in 5 words");

    match client2.exec(&req2, Some(&AuthData::from_single(api_key.as_str()))).await {
        Ok(response) => eprintln!("Success: {:?}", response),
        Err(e) => eprintln!("Error: {}", e),
    }
}
