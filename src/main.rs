use std::env;
use std::ops::Not;
use std::time::Duration;

use bevy::prelude::*;
use bevy::{
    app::{App, EventReader, ScheduleRunnerSettings},
    core::Time,
    MinimalPlugins,
};
use log::info;
use log::warn;
#[cfg(not(feature = "web"))]
use log::LevelFilter;

use bevy_ws::WebsocketClientEvent;
use bevy_ws::WebsocketPlugin;
use bevy_ws::WebsocketResource;
#[cfg(not(feature = "web"))]
use bevy_ws::WebsocketServerEvent;
#[cfg(not(feature = "web"))]
use bevy_ws::{WebsocketServerPlugin, WebsocketServerResource};

use crate::network::{ClientMessage, MovePaddle, ServerMessage};

mod network;

const PORT: u16 = 8080;

struct TenTimesPerSecond {
    timer: Timer,
}

impl Default for TenTimesPerSecond {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(0.1, true),
        }
    }
}

fn main() {
    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();
    // env_logger::builder()
    //     .filter_level(LevelFilter::Info)
    //     // .filter_module("pong-royale", LevelFilter::Info)
    //     .init();
    #[cfg(not(target_arch = "wasm32"))]
    let _res = env_logger::builder()
        .filter_level(LevelFilter::Info)
        .try_init();
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
        .insert_resource(TenTimesPerSecond::default());

    if is_server() {
        app.add_plugins(MinimalPlugins);
        // app.add_plugin(LogPlugin::default());
        #[cfg(not(feature = "web"))]
        app.add_plugin(WebsocketServerPlugin);
        #[cfg(not(feature = "web"))]
        app.add_startup_system(startup_server.system());
        #[cfg(not(feature = "web"))]
        app.add_system(handle_packets_server.system());
        #[cfg(not(feature = "web"))]
        app.add_system(spawn_paddle_system_server.system());
    } else {
        #[cfg(not(feature = "headless"))]
        {
            app.add_plugins(DefaultPlugins);
            app.add_plugin(WebsocketPlugin);
            app.add_startup_system(client_startup.system());
            app.add_startup_system(startup_client.system());
            app.add_system(create_network_event_from_keyboard_input.system());
            app.add_system(handle_packets_client.system());
            app.add_system(spawn_paddle_system_client.system());
            app.insert_resource(PlayerId::default());
        }
    }

    #[cfg(target_arch = "wasm32")]
    app.add_plugin(bevy_webgl2::WebGL2Plugin);

    app
        // The NetworkingPlugin
        // .add_plugin(NetworkingPlugin {
        //     ..Default::default()
        // })
        .add_event::<PlayerEcsEvent>();
    // Our networking
    // .insert_resource(parse_simple_args())
    // .add_startup_system(startup.system());

    app.run();
}

struct Paddle {
    speed: f32,
}

struct ControlledByPlayer {
    player_id: u64,
}

enum PlayerEcsEvent {
    Connected(u64),
    Disconnected(u64),
}

#[derive(Default)]
struct PlayerId(Option<u64>);

fn startup_client(mut ws: ResMut<WebsocketResource>) {
    ws.open(format!("ws://localhost:{}", PORT).as_str());
}

#[cfg(not(feature = "web"))]
fn startup_server(mut ws: ResMut<WebsocketServerResource>) {
    ws.listen(format!("localhost:{}", PORT).as_str());
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
            .insert(ControlledByPlayer { player_id: 0 });

        commands
            .spawn_bundle(SpriteBundle {
                material: materials.add(Color::rgb(0.5, 0.5, 1.0).into()),
                transform: Transform::from_xyz(0.0, 2. as f32 * 40. - 200., 0.0),
                sprite: Sprite::new(Vec2::new(120.0, 30.0)),
                ..Default::default()
            })
            .insert(Paddle { speed: 500.0 })
            .insert(ControlledByPlayer { player_id: 1 });

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
                        value: "0".to_string(), // todo get player number
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

fn create_network_event_from_keyboard_input(
    time: Res<Time>,
    mut ten_times_per_second: Local<TenTimesPerSecond>,
    keyboard_input: Res<Input<KeyCode>>,
    mut query: Query<(&Paddle, &mut Transform, &ControlledByPlayer)>,
    net: Res<WebsocketResource>,
    player_id: Res<PlayerId>,
) {
    if ten_times_per_second
        .timer
        .tick(time.delta())
        .just_finished()
        .not()
    {
        return;
    }
    for (paddle, mut transform, controlled_by_player) in query.iter_mut() {
        let player_id = match player_id.0 {
            None => {
                warn!("No player id, discarding key movement");
                continue;
            }
            Some(id) => id,
        };
        if controlled_by_player.player_id != player_id {
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
        if 1.0 > x && x > -1.0 {
            continue;
        }
        // translation.x += time.delta_seconds() * direction * paddle.speed;
        // bound the paddle within the walls
        // translation.x = translation.x.min(380.0).max(-380.0);
        let msg = ClientMessage::MovePaddle(MovePaddle {
            player_id,
            position: x,
        });
        let str = serde_json::to_string(&msg).expect("unable to serialize json");
        info!("Sending {}", str);
        net.broadcast(str.to_string());
    }
}

fn handle_packets_client(
    mut network_event_reader: EventReader<WebsocketClientEvent>,
    mut query_to_move_paddles: Query<(&Paddle, &mut Transform, &ControlledByPlayer)>,
    mut player_events: EventWriter<PlayerEcsEvent>,
    mut player_client_id: ResMut<PlayerId>,
) {
    for event in network_event_reader.iter() {
        let event: &WebsocketClientEvent = event;
        match event {
            WebsocketClientEvent::OnMessage(msg) => {
                info!("Received msg: {}", msg);
                let server_message: ServerMessage =
                    serde_json::from_str(msg).expect("unable to deserialize json");
                match server_message {
                    ServerMessage::PlayerStateUpdate(move_paddle) => {
                        let player_id = move_paddle.player_id;
                        for (_paddle, mut transform, controlled_by_player) in
                            query_to_move_paddles.iter_mut()
                        {
                            if controlled_by_player.player_id != player_id {
                                continue;
                            }
                            let translation = &mut transform.translation;
                            info!(
                                "Updating paddle posiition from {} to {}",
                                translation.x, move_paddle.position
                            );
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

            WebsocketClientEvent::OnOpen(client_id) => {
                println!(
                    "Connected event: is_server: {} got id {}",
                    is_server(),
                    client_id
                );
                player_client_id.0.replace(*client_id);
            }
            WebsocketClientEvent::OnClose => {}
        }
    }
}

#[cfg(not(feature = "web"))]
fn handle_packets_server(
    mut net: ResMut<WebsocketServerResource>,
    mut network_event_reader: EventReader<WebsocketServerEvent>,
    mut player_events: EventWriter<PlayerEcsEvent>,
) {
    for event in network_event_reader.iter() {
        let event: &WebsocketServerEvent = event;
        info!("Received event: {:?}", event);
        match event {
            WebsocketServerEvent::OnMessage(client_id, msg) => {
                info!("Server received message from {}: {}", client_id, msg);
                let client_message: ClientMessage =
                    serde_json::from_str(msg).expect("unable to deserialize json");
                match client_message {
                    ClientMessage::MovePaddle(move_paddle) => {
                        let message = ServerMessage::PlayerStateUpdate(move_paddle);
                        let msg =
                            serde_json::to_string(&message).expect("unable to serialize json");
                        net.broadcast(msg);
                    }
                }
            }
            WebsocketServerEvent::OnOpen(client_id) => {
                println!("Connected id: {:?}", client_id);
                player_events.send(PlayerEcsEvent::Connected(*client_id));

                let message = ServerMessage::PlayerConnected(*client_id);
                let str = serde_json::to_string(&message).expect("unable to serialize json");
                net.broadcast(str);
            }
            WebsocketServerEvent::OnClose(client_id) => {
                println!("Client {} disconnected", client_id);
                player_events.send(PlayerEcsEvent::Disconnected(*client_id));

                let message = ServerMessage::PlayerDisconnected(*client_id);
                let str = serde_json::to_string(&message).expect("unable to serialize json");
                net.broadcast(str);
            }
        }
    }
}

#[cfg(not(feature = "web"))]
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
