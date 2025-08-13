use serde::{Deserialize, Serialize};
use socketioxide::{
    adapter::Adapter,
    extract::{Data, SocketRef},
};

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenTrade {
    tokens: Vec<String>,
}

pub async fn on_token_trade<A: Adapter>(socket: SocketRef<A>, Data(req): Data<TokenTrade>) {
    let rooms: Vec<String> = req.tokens.clone();
    socket.join(rooms);
}
