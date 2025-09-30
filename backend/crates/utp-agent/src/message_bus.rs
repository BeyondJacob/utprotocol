use utp_core::Frame;
use anyhow::Result;
use tokio::sync::RwLock;
use std::sync::Arc;

pub struct MessageBus {
    frames: RwLock<Vec<Frame>>,
}

impl MessageBus {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            frames: RwLock::new(Vec::new()),
        })
    }

    pub async fn send(&self, frame: Frame) -> Result<()> {
        let mut frames = self.frames.write().await;
        frames.push(frame);
        Ok(())
    }

    pub async fn receive(&self, agent_name: &str) -> Result<Option<Frame>> {
        let mut frames = self.frames.write().await;
        if let Some(pos) = frames.iter().position(|f| f.source != agent_name) {
            Ok(Some(frames.remove(pos)))
        } else {
            Ok(None)
        }
    }
}