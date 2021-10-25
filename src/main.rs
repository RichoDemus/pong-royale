use std::{net::SocketAddr, time::Duration};
use std::env;
use std::net::{IpAddr, Ipv4Addr};
use std::str::FromStr;

use bevy::{
    app::{App, EventReader, ScheduleRunnerSettings},
    core::Time,
    ecs::prelude::*,
    MinimalPlugins,
};
use bevy::core::AsBytes;
use bevy::log::LogPlugin;
use bevy::prelude::*;
use bevy_networking_turbulence::{NetworkEvent, NetworkingPlugin, NetworkResource, Packet};
use serde::Serialize;

use crate::network::{ClientMessage, MovePaddle, ServerMessage};

mod network;

const SERVER_PORT: u16 = 14191;

fn main() {
    // env_logger::builder()
    //     .filter_level(LevelFilter::Info)
    //     // .filter_module("pong-royale", LevelFilter::Info)
    //     .init();
    info!("hello world!");

    let mut app = App::build();
    app
        // minimal plugins necessary for timers + headless loop
        .insert_resource(ScheduleRunnerSettings::run_loop(Duration::from_secs_f64(
            1.0 / 60.0,
        )))
    .insert_resource(bevy::log::LogSettings {
        level: bevy::log::Level::INFO,
        // filter: "pong-royale::*=info".to_string(),
        ..Default::default()
    })
    ;

     if is_server() {
         app.add_plugins(MinimalPlugins);
         app.add_plugin(LogPlugin::default());
         app.add_system(handle_packets_server.system());
         app.add_system(spawn_paddle_system_server.system());
    } else {
         #[cfg(not(feature = "headless"))] {
             app.add_plugins(DefaultPlugins);
             app.add_startup_system(client_startup.system());
             app.add_system(create_network_event_from_keyboard_input.system());
             app.add_system(handle_packets_client.system());
             app.add_system(spawn_paddle_system_client.system());
         }
     }

    #[cfg(target_arch = "wasm32")]
    app.add_plugin(bevy_webgl2::WebGL2Plugin);

    app
        // The NetworkingPlugin
        .add_plugin(NetworkingPlugin {
            ..Default::default()
        })
        .add_event::<PlayerEcsEvent>()
        // Our networking
        // .insert_resource(parse_simple_args())
        .add_startup_system(startup.system());

    app.run();
}

struct Paddle {
    speed: f32,
}

struct ControlledByPlayer {
    player_id: u32,
}

enum PlayerEcsEvent {
    Connected(u32),
    Disconnected(u32),
}

fn startup(
    mut net: ResMut<NetworkResource>
) {
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            // set the following address to your server address (i.e. local machine)
            // and remove compile_error! line
            let mut server_address: SocketAddr = "192.168.50.61:0".parse().unwrap(); // todo update this line
            // compile_error!("You need to set server_address.");
            server_address.set_port(SERVER_PORT);
        } else {
            let ip_address =
                bevy_networking_turbulence::find_my_ip_address().expect("can't find ip address");
            let server_address = SocketAddr::new(ip_address, SERVER_PORT);
        }
    }

    #[cfg(target_arch = "wasm32")]
    net.connect(server_address);

    #[cfg(not(target_arch = "wasm32"))]
    if is_server() {
        // let server_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::from_str("0.0.0.0").unwrap()), SERVER_PORT);
        // let server_address2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::from_str("0.0.0.0").unwrap()), SERVER_PORT+1);
        // let server_address3 = SocketAddr::new(IpAddr::V4(Ipv4Addr::from_str("0.0.0.0").unwrap()), SERVER_PORT+2);
        info!("Starting server on {:?}", server_address);
        net.listen(server_address, Some(server_address), Some(server_address));
    } else {
        info!("Starting client, connecting to: {:?}", server_address);
        net.connect(server_address);
    }


}

#[cfg(not(feature = "headless"))]
fn client_startup(
    mut commands: Commands,
    mut materials: ResMut<Assets<ColorMaterial>>,
    asset_server: Res<AssetServer>,
) {
    if !is_server() {

        commands
            .spawn_bundle(SpriteBundle {
                material: materials.add(Color::rgb(0.5, 0.5, 1.0).into()),
                transform: Transform::from_xyz(0.0, 1. as f32 * 40. - 200., 0.0),
                sprite: Sprite::new(Vec2::new(120.0, 30.0)),
                ..Default::default()
            })
            .insert(Paddle { speed: 500.0 })
            .insert(ControlledByPlayer { player_id: 1 });

        commands
            .spawn_bundle(SpriteBundle {
                material: materials.add(Color::rgb(0.5, 0.5, 1.0).into()),
                transform: Transform::from_xyz(0.0, 2. as f32 * 40. - 200., 0.0),
                sprite: Sprite::new(Vec2::new(120.0, 30.0)),
                ..Default::default()
            })
            .insert(Paddle { speed: 500.0 })
            .insert(ControlledByPlayer { player_id: 2 });


        commands.spawn_bundle(OrthographicCameraBundle::new_2d());
        commands.spawn_bundle(UiCameraBundle::default());

        commands.spawn_bundle(TextBundle {
                text: Text {
                    sections: vec![
                        TextSection {
                            value: "Player: ".to_string(),
                            style: TextStyle {
                                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                                font_size: 40.0,
                                color: Color::rgb(0.5, 0.5, 1.0),
                            },
                        },
                        TextSection {
                            value: get_player_number().to_string(),
                            style: TextStyle {
                                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                                font_size: 40.0,
                                color: Color::rgb(1.0, 0.5, 0.5),
                            },
                        },
                    ],
                    ..Default::default()
                },
                style: Style {
                    position_type: PositionType::Absolute,
                    position: Rect {
                        top: Val::Px(5.0),
                        left: Val::Px(5.0),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            });
    }
}

fn is_server() -> bool {
    let args: Vec<String> = env::args().collect();

    if let Some(arg) = args.get(1) {
        if arg.eq("--server") {
            return true;
        }
    }
    return false;
}

fn get_player_number() -> u32 {
    #[cfg(target_arch = "wasm32")]
    return 1;
    let args: Vec<String> = env::args().collect();

    if let Some(arg) = args.get(2) {
        return arg.parse().unwrap();
    }
    panic!("no arg 2 :(");
}

fn create_network_event_from_keyboard_input(
    time: Res<Time>,
    keyboard_input: Res<Input<KeyCode>>,
    mut query: Query<(&Paddle, &mut Transform, &ControlledByPlayer)>,
    mut net: ResMut<NetworkResource>,
) {
    for (paddle, mut transform, controlled_by_player) in query.iter_mut() {
        if controlled_by_player.player_id != get_player_number() {
            continue;
        }

        let mut direction = 0.0;
        if keyboard_input.pressed(KeyCode::A) {
            direction -= 3.0;
        }

        if keyboard_input.pressed(KeyCode::D) {
            direction += 3.0;
        }

        let translation = &mut transform.translation;
        // move the paddle horizontally
        let x = translation.x + time.delta_seconds() * direction * paddle.speed;
        // translation.x += time.delta_seconds() * direction * paddle.speed;
        // bound the paddle within the walls
        // translation.x = translation.x.min(380.0).max(-380.0);
        let msg = ClientMessage::MovePaddle(MovePaddle {
            player_id: get_player_number(),
            position: x,
        });
        let bytes = serde_json::to_vec(&msg).expect("unable to serialize json");
        net.broadcast(Packet::from(bytes));
    }
}

fn handle_packets_client(
    mut network_event_reader: EventReader<NetworkEvent>,
    mut query_to_move_paddles: Query<(&Paddle, &mut Transform, &ControlledByPlayer)>,
    mut player_events: EventWriter<PlayerEcsEvent>,
) {
    for event in network_event_reader.iter() {
        let event: &NetworkEvent = event;
        match event {
            NetworkEvent::Packet(handle, packet) => {
                let server_message: ServerMessage = serde_json::from_slice(packet.as_bytes()).expect("unable to deserialize json");
                match server_message {
                    ServerMessage::PlayerStateUpdate(move_paddle) => {
                        let player_id = move_paddle.player_id;
                        for (_paddle, mut transform, controlled_by_player) in query_to_move_paddles.iter_mut() {
                            if controlled_by_player.player_id != player_id {
                                continue;
                            }
                            let translation = &mut transform.translation;

                            translation.x = move_paddle.position;
                        }
                    }
                    ServerMessage::PlayerConnected(id) => {
                        player_events.send(PlayerEcsEvent::Connected(id));
                    }
                    ServerMessage::PlayerDisconnected(id) => {
                        player_events.send(PlayerEcsEvent::Disconnected(id));
                    }
                }

            }

            NetworkEvent::Connected(handle) => {
                println!("Connected event: {:?}, is_server: {}", handle, is_server());
            }
            NetworkEvent::Disconnected(_) => {}
            NetworkEvent::Error(_, _) => {}
        }
    }

}

fn handle_packets_server(
    mut net: ResMut<NetworkResource>,
    mut network_event_reader: EventReader<NetworkEvent>,
    mut player_events: EventWriter<PlayerEcsEvent>,
) {
    for event in network_event_reader.iter() {
        let event: &NetworkEvent = event;
        info!("Received event: {:?}", event);
        match event {
            NetworkEvent::Packet(handle, packet) => {
                let client_message: ClientMessage = serde_json::from_slice(packet.as_bytes()).expect("unable to deserialize json");
                match client_message {
                    ClientMessage::MovePaddle(move_paddle) => {
                        let message = ServerMessage::PlayerStateUpdate(move_paddle);
                        let bytes = serde_json::to_vec(&message).expect("unable to serialize json");
                        net.broadcast(Packet::from(bytes));
                    }
                }
            }
            NetworkEvent::Connected(handle) => {
                println!("Connected event: {:?}", handle);
                let player_id: &u32 = handle.into();
                let player_id:u32 = player_id.clone();
                player_events.send(PlayerEcsEvent::Connected(player_id));

                let message = ServerMessage::PlayerConnected(player_id);
                let bytes = serde_json::to_vec(&message).expect("unable to serialize json");
                net.broadcast(Packet::from(bytes));
            }
            NetworkEvent::Disconnected(handle) => {
                println!("Disconnected event: {:?}", handle);
                let player_id: &u32 = handle.into();
                let player_id:u32 = player_id.clone();
                player_events.send(PlayerEcsEvent::Disconnected(player_id));

                let message = ServerMessage::PlayerDisconnected(player_id);
                let bytes = serde_json::to_vec(&message).expect("unable to serialize json");
                net.broadcast(Packet::from(bytes));
            }
            NetworkEvent::Error(_, _) => {}
        }
    }
}

fn spawn_paddle_system_server(mut events: EventReader<PlayerEcsEvent>) {
    for my_event in events.iter() {
        let my_event: &PlayerEcsEvent = my_event;
        if let &PlayerEcsEvent::Connected(id) = my_event {
            println!("time to spawn a fucking paddle yo player_id: {}", id);
        }
    }
}

#[cfg(not(feature = "headless"))]
fn spawn_paddle_system_client(
    mut commands: Commands,
    mut events: EventReader<PlayerEcsEvent>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    for my_event in events.iter() {
        let my_event: &PlayerEcsEvent = my_event;
        if let &PlayerEcsEvent::Connected(id) = my_event {
            commands
                .spawn_bundle(SpriteBundle {
                    material: materials.add(Color::rgb(0.5, 0.5, 1.0).into()),
                    transform: Transform::from_xyz(0.0, id as f32 * 40. - 200., 0.0),
                    sprite: Sprite::new(Vec2::new(120.0, 30.0)),
                    ..Default::default()
                })
                .insert(Paddle { speed: 500.0 })
                .insert(ControlledByPlayer { player_id: id });
        }
    }
}
