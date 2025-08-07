pub use crate::ws::{event::RequestEvent, token::on_token_trade};
use socketioxide::{adapter::Adapter, extract::SocketRef};
use tracing::{info, warn};

/// Called when a client connects to the server
pub async fn on_connect<A: Adapter>(
    socket: SocketRef<A>,
    // Data(data): Data<Value>, // auth data
    // State(state): State<AppState>,
) {
    info!(ns = socket.ns(), ?socket.id, "Websocket connected");
    socket.on(RequestEvent::TokenTrade.to_string(), on_token_trade);
    socket.on_disconnect(on_disconnect);
}

/// Called when a client disconnects from the server
pub async fn on_disconnect<A: Adapter>(socket: SocketRef<A>) {
    warn!(ns = socket.ns(), ?socket.id, "Websocket disconnected");
}
