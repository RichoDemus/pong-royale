use bevy::prelude::Vec2;
use serde::Deserialize;
use serde::Serialize;

// #[derive(Serialize, Deserialize, Debug)]
// enum SimpleNetworkMessage {
//     MovePaddle(MovePaddle),
// }

#[derive(Serialize, Deserialize, Debug)]
pub struct MovePaddle {
    pub player_id: u64,
    // todo should only be sent by server
    pub position: f32,
}

// client to server
// client can stop, move left and move right
#[derive(Serialize, Deserialize, Debug)]
pub enum ClientMessage {
    //StopMoving, StartMovingLeft, StartMovingRight
    MovePaddle(MovePaddle),
}

#[derive(Serialize, Deserialize, Debug)]
struct PlayerState {
    id: u32,
    position: Vec2,
    velocity: Vec2,
}

// server
#[derive(Serialize, Deserialize, Debug)]
pub enum ServerMessage {
    PlayerStateUpdate(MovePaddle),
    PlayerConnected(u64),
    PlayerDisconnected(u64),
}
