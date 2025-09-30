use utp_llm::{LlamaClient, ChatRequest, Message};

#[tokio::test]
async fn test_real_llm_call() {
    let client = LlamaClient::new("http://127.0.0.1:8080");
    
    let request = ChatRequest {
        model: "gpt-oss-20b".to_string(),
        messages: vec![Message {
            role: "user".to_string(),
            content: "Say 'test successful' and nothing else.".to_string(),
        }],
        temperature: Some(0.1),
        max_tokens: Some(10),
    };
    
    let response = client.chat(request).await.unwrap();
    
    println!("Response: {}", response.choices[0].message.content);
    assert!(!response.choices.is_empty());
}