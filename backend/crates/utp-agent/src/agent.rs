use utp_llm::LlamaClient;
use utp_core::Frame;
use anyhow::Result;
use std::sync::Arc;
use crate::MessageBus;

pub struct Agent {
    name: String,
    llama_client: LlamaClient,
    message_bus: Arc<MessageBus>,
}

impl Agent {
    pub fn new(name: String, llama_client: LlamaClient, message_bus: Arc<MessageBus>) -> Self {
        Self { name, llama_client: llama_client, message_bus }
    }

    pub async fn send_frame(&self, frame: Frame) -> Result<()> {
        self.message_bus.send(frame).await
    }

    pub async fn receive_frame(&self) -> Result<Option<Frame>> {
        self.message_bus.receive(&self.name).await
    }

    pub async fn process_and_send(&self, prompt: &str, model: &str) -> Result<()> {
        let chat_request = utp_llm::ChatRequest {
            model: model.to_string(),
            messages: vec![utp_llm::Message {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            temperature: Some(0.1),
            max_tokens: Some(10),
        };
        let chat_response = self.llama_client.chat(chat_request).await?;
        let response_text = chat_response.choices[0].message.content.clone();
        let compressed = utp_core::Compressor::new().compress(&response_text.as_bytes())?;
        let frame = Frame::new(compressed, self.name.clone());
        self.send_frame(frame).await?;
        Ok(())
    }
}