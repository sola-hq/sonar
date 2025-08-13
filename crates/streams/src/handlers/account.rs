use serde::{Deserialize, Serialize};
use socketioxide::{
    adapter::Adapter,
    extract::{Data, SocketRef},
};

#[derive(Debug, Serialize, Deserialize)]
pub struct AccountChange {
    accounts: Vec<String>,
}

/// Subscribe on account change events for the given accounts.
///
/// This handler is used to subscribe on account change events for the given accounts.
/// It will join the socket to the given accounts.
///
/// # Arguments
/// * `socket` - The socket to join the rooms to.
pub async fn subscribe_on_account_change<A: Adapter>(
    socket: SocketRef<A>,
    Data(req): Data<AccountChange>,
) {
    let rooms: Vec<String> = req.accounts;
    socket.join(rooms);
}
