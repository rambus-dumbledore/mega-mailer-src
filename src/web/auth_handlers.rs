use std::sync::Arc;

use axum::{
    response::IntoResponse,
    routing::{get, post},
    Json, Router, Extension,
};
use serde::Deserialize;

use common::{sessions::{SessionManager, WebAppInitData}, cfg::Cfg};
use common::types::{Error, Result};


async fn logout(mut sm: SessionManager) -> impl IntoResponse {
    sm.logout_v2().await;
}

#[derive(Debug, Deserialize)]
struct AuthParams {
    pub init_data: String,
}

async fn auth(
    mut sm: SessionManager,
    Extension(cfg): Extension<Arc<Cfg>>,
    Json(params): Json<AuthParams>,
) -> Result<impl IntoResponse> {
    let init_data = WebAppInitData::try_from(params.init_data.as_str())
        .map_err(|e| Error::InternalError(common::types::InternalError::RuntimeError(format!("invalid `init_data` value: {}", e))))?;

    let bot_token = &cfg.bot.token;
    init_data.validate(bot_token)
        .map_err(|e| Error::InternalError(common::types::InternalError::RuntimeError(format!("`init_data` is not valid: {}", e))))?;

    let ref user = match init_data.user {
        Some(user) => user,
        _ => return Err(Error::InternalError(common::types::InternalError::RuntimeError("no user in `init_data`".into())))
    };

    match sm.auth_v2(user).await {
        Err(e) => {
            return Err(Error::InternalError(common::types::InternalError::RuntimeError(format!("failed to auth_v2: {}", e))))
        },
        _ => {}
    };

    Ok(())
}

pub fn auth_routes() -> Router {
    Router::new()
        .route("/auth", post(auth))
        .route("/logout", get(logout))
}
