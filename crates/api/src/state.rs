use sonar_db::{Database, KvStore};
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub kv_store: Arc<KvStore>,
    pub db: Arc<Database>,
}
