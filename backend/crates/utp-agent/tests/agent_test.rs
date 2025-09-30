use utp_core::Compressor;
use utp_agent::{Agent, MessageBus};
use utp_llm::LlamaClient;
use utp_core::Frame;
use std::sync::Arc;

#[tokio::test]
async fn test_agents_share_message_bus() {
    let bus = MessageBus::new();
    
    let agent1 = Agent::new(
        "agent-1".to_string(),
        LlamaClient::new("http://localhost:8080"),
        Arc::clone(&bus),
    );
    
    let agent2 = Agent::new(
        "agent-2".to_string(),
        LlamaClient::new("http://localhost:8080"),
        Arc::clone(&bus),
    );
    
    // Agent 1 sends a frame
    let frame = Frame::new(vec![1, 2, 3], "agent-1".to_string());
    agent1.send_frame(frame).await.unwrap();
    
    // Agent 2 receives it
    let received = agent2.receive_frame().await.unwrap();
    assert!(received.is_some());
    assert_eq!(received.unwrap().source, "agent-1");
}

#[tokio::test]
async fn test_agent_processes_and_sends_compressed_frame() {
    // Start your llama.cpp server first if not running
    
    let bus = MessageBus::new();
    
    let agent1 = Agent::new(
        "agent-1".to_string(),
        LlamaClient::new("http://localhost:8080"),
        Arc::clone(&bus),
    );
    
    let agent2 = Agent::new(
        "agent-2".to_string(),
        LlamaClient::new("http://localhost:8080"),
        Arc::clone(&bus),
    );
    
    // Agent 1 processes prompt and sends compressed frame
    agent1.process_and_send("Say hello in one word", "gpt-oss-20b")
        .await
        .unwrap();
    
    // Agent 2 receives the compressed frame
    let frame = agent2.receive_frame().await.unwrap().unwrap();
    
    // Decompress and verify
    let compressor = Compressor::new();
    let decompressed = compressor.decompress(&frame.payload).unwrap();
    let text = String::from_utf8(decompressed).unwrap();
    
    println!("Agent 1 sent: {}", text);
    assert!(!text.is_empty());
    assert_eq!(frame.source, "agent-1");
}