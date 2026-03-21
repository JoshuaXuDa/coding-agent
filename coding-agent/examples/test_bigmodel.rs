use genai::chat::{builder::Chat, ChatRequest};
use genai::{Client, ModelIden};
use genai::adapter::AdapterKind;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not set");

    println!("Testing BigModel Coding endpoint with genai library...");

    // Test 1: ZAI adapter with coding endpoint
    println!("\n=== Test 1: ZAI adapter with coding endpoint ===");
    let client1 = Client::builder()
        .with_service_target_resolver_fn(|mut service_target: genai::ServiceTarget| {
            if service_target.model.model_name.as_str() == "glm-4.7" {
                service_target.endpoint = genai::resolver::Endpoint::from_static(
                    "https://open.bigmodel.cn/api/coding/paas/v4/"
                );
                service_target.auth = genai::resolver::AuthData::from_single(api_key.as_str());
            }
            Ok(service_target)
        })
        .build();

    let req1 = ChatRequest::default()
        .with_system("You are a helpful assistant.")
        .with_message("Say hello in 3 words");

    match client1.exec_chat("glm-4.7", &req1, None).await {
        Ok(response) => println!("Success! Response: {:?}", response.content),
        Err(e) => println!("Error: {:?}", e),
    }

    // Test 2: OpenAI adapter with OPENAI_BASE_URL
    println!("\n=== Test 2: OpenAI adapter with OPENAI_BASE_URL ===");
    std::env::set_var("OPENAI_BASE_URL", "https://open.bigmodel.cn/api/coding/paas/v4/");

    let client2 = Client::builder()
        .with_service_target_resolver_fn(|mut service_target: genai::ServiceTarget| {
            if service_target.model.model_name.as_str() == "glm-4.7" {
                let new_model = ModelIden::new(AdapterKind::OpenAI, "glm-4.7");
                service_target.model = new_model;
                service_target.auth = genai::resolver::AuthData::from_single(api_key.as_str());
            }
            Ok(service_target)
        })
        .build();

    let req2 = ChatRequest::default()
        .with_system("You are a helpful assistant.")
        .with_message("Say hello in 3 words");

    match client2.exec_chat("glm-4.7", &req2, None).await {
        Ok(response) => println!("Success! Response: {:?}", response.content),
        Err(e) => println!("Error: {:?}", e),
    }

    Ok(())
}
