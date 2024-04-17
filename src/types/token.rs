use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ConfirmationToken {
    pub user_id: uuid::Uuid,
}
