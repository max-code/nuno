#[derive(Default)]
pub enum OperationStatusType {
    #[default]
    Info,
    Success,
    Error,
}

impl OperationStatusType {
    pub fn get_emoji(&self) -> &'static str {
        match self {
            OperationStatusType::Info => "ℹ️ ",
            OperationStatusType::Success => "✅",
            OperationStatusType::Error => "❌",
        }
    }
}
pub struct OperationStatus {
    pub message: String,
    pub status_type: OperationStatusType,
    timestamp: std::time::Instant,
}

impl Default for OperationStatus {
    fn default() -> Self {
        Self {
            message: String::new(),
            status_type: OperationStatusType::default(),
            timestamp: std::time::Instant::now(),
        }
    }
}

impl OperationStatus {
    pub fn is_expired_or_empty(&self) -> bool {
        self.timestamp.elapsed().as_secs() > 3 && !self.message.is_empty()
    }

    pub fn set(&mut self, message: String, status_type: OperationStatusType) {
        self.message = message;
        self.status_type = status_type;
        self.timestamp = std::time::Instant::now();
    }
}
