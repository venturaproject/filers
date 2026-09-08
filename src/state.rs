use std::sync::Arc;

use crate::{
    application::processing::service::ProcessingService,
    config::Config,
};

pub struct AppState {
    pub config: Config,
    pub processing: Arc<ProcessingService>,
}
