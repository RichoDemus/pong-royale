#[cfg(test)]
mod figure_out_bevy_async;
#[cfg(test)]
#[cfg(test)]
mod tests {
    use bevy::app::AppExit;
    use bevy::prelude::*;
    use log::info;
    use log::LevelFilter;
    use serde::Deserialize;
    use serde::Serialize;

    use bevy_ws::client::{WebsocketPlugin, WebsocketResource};
    use bevy_ws::server::{WebsocketServerPlugin, WebsocketServerResource};
    use bevy_ws::{WebsocketClientEvent, WebsocketServerEvent};

    const GOAL: u32 = 10;

    #[derive(Serialize, Deserialize, Debug)]
    struct Message {
        counter: u32,
        sender: String,
    }

    struct NameResource(String);

    struct PortResource(u32);

    fn startup_server(mut ws: ResMut<WebsocketServerResource>, port: Res<PortResource>) {
        info!("Server started");
        ws.listen(format!("localhost:{}", port.0).as_str());
    }

    fn startup_client(mut ws: ResMut<WebsocketResource>, port: Res<PortResource>) {
        info!("Client started");
        ws.open(format!("ws://localhost:{}", port.0).as_str());
    }

    fn network_bounce_client_single(
        ws: Res<WebsocketResource>,
        name: Res<NameResource>,
        mut ws_events: EventReader<WebsocketClientEvent>,
        mut exit: EventWriter<AppExit>,
    ) {
        for event in ws_events.iter() {
            let event: &WebsocketClientEvent = event;
            match event {
                WebsocketClientEvent::OnOpen(client_id) => {
                    info!("Client {} got assigned id {}", name.0, client_id);
                    if name.0.eq("Client 1") {
                        ws.broadcast(
                            serde_json::to_string(&Message {
                                counter: 0,
                                sender: name.0.clone(),
                            })
                            .unwrap(),
                        );
                        info!("\t{} sending {}", name.0, 0);
                    }
                }
                WebsocketClientEvent::OnMessage(str) => {
                    info!("{} received {}", name.0, str);
                    let msg: Message = serde_json::from_str(str).unwrap();
                    ws.broadcast(
                        serde_json::to_string(&Message {
                            counter: msg.counter + 1,
                            sender: name.0.clone(),
                        })
                        .unwrap(),
                    );
                    if msg.counter > GOAL {
                        info!("closing ws");
                        ws.close();
                        exit.send(AppExit);
                    }
                }
                WebsocketClientEvent::OnClose => {}
            }
        }
    }

    fn network_bounce_client_multiple(
        ws: Res<WebsocketResource>,
        name: Res<NameResource>,
        mut ws_events: EventReader<WebsocketClientEvent>,
        mut exit: EventWriter<AppExit>,
    ) {
        for event in ws_events.iter() {
            let event: &WebsocketClientEvent = event;
            match event {
                WebsocketClientEvent::OnOpen(client_id) => {
                    info!("Client {} got assigned id {}", name.0, client_id);
                    if name.0.eq("Client 1") {
                        ws.broadcast(
                            serde_json::to_string(&Message {
                                counter: 0,
                                sender: name.0.clone(),
                            })
                            .unwrap(),
                        );
                        info!("\t{} sending {}", name.0, 0);
                    }
                }
                WebsocketClientEvent::OnMessage(str) => {
                    info!("{} received {}", name.0, str);
                    let msg: Message = serde_json::from_str(str).unwrap();
                    match (msg.counter % 2, name.0.as_str()) {
                        (0, "Client 1") => {
                            ws.broadcast(
                                serde_json::to_string(&Message {
                                    counter: msg.counter + 1,
                                    sender: name.0.clone(),
                                })
                                .unwrap(),
                            );

                            info!("\t{} sending {}", name.0, msg.counter + 1);
                        }
                        (1, "Client 2") => {
                            ws.broadcast(
                                serde_json::to_string(&Message {
                                    counter: msg.counter + 1,
                                    sender: name.0.clone(),
                                })
                                .unwrap(),
                            );
                            info!("\t{} sending {}", name.0, msg.counter + 1);
                        }
                        _ => {}
                    }
                    if msg.counter > GOAL {
                        info!("closing ws");
                        ws.close();
                        exit.send(AppExit);
                    }
                }
                WebsocketClientEvent::OnClose => {}
            }
        }
    }

    fn network_bounce_server(
        mut ws: ResMut<WebsocketServerResource>,
        mut ws_events: EventReader<WebsocketServerEvent>,
        mut exit: EventWriter<AppExit>,
    ) {
        for event in ws_events.iter() {
            let event: &WebsocketServerEvent = event;
            match event {
                WebsocketServerEvent::OnOpen(client_id) => {
                    info!("User {} connected", client_id);
                }
                WebsocketServerEvent::OnMessage(client_id, str) => {
                    info!("Server received {} from client {}", str, client_id);
                    let msg: Message = serde_json::from_str(str).unwrap();
                    ws.broadcast(
                        serde_json::to_string(&Message {
                            counter: msg.counter,
                            sender: "Server".to_string(),
                        })
                        .unwrap(),
                    );
                    if msg.counter > GOAL {
                        info!("not closing ws");
                        exit.send(AppExit);
                        ws.close();
                    }
                }
                WebsocketServerEvent::OnClose(client_id) => {
                    info!("User {} disconnected", client_id);
                }
            }
        }
    }

    #[test]
    #[ignore]
    fn wtf() {
        let _res = env_logger::builder()
            .filter_level(LevelFilter::Info)
            .try_init();

        let server = std::thread::spawn(move || {
            App::new()
                .add_plugins(MinimalPlugins)
                .add_plugin(WebsocketServerPlugin)
                .add_startup_system(startup_server.system())
                .add_system(network_bounce_server.system())
                .insert_resource(NameResource("Server".to_string()))
                .run();
            info!("Server thread done");
        });

        server.join().unwrap();
    }

    #[test]
    #[ignore]
    fn only_client() {
        let _res = env_logger::builder()
            .filter_level(LevelFilter::Info)
            .try_init();

        let client = std::thread::spawn(move || {
            App::new()
                .add_plugins(MinimalPlugins)
                .add_plugin(WebsocketPlugin)
                .add_startup_system(startup_client.system())
                .add_system(network_bounce_client_single.system())
                .insert_resource(NameResource("Client 1".to_string()))
                .run();
            info!("Client 1 thread done");
        });
        client.join().unwrap();
    }

    #[test]
    #[ignore]
    fn only_server() {
        let _res = env_logger::builder()
            .filter_level(LevelFilter::Info)
            .try_init();

        let server = std::thread::spawn(move || {
            App::new()
                .add_plugins(MinimalPlugins)
                .add_plugin(WebsocketServerPlugin)
                .add_startup_system(startup_server.system())
                .add_system(network_bounce_server.system())
                .insert_resource(NameResource("Server".to_string()))
                .run();
            info!("Server thread done");
        });
        server.join().unwrap();
    }

    #[test]
    // #[tokio::test(flavor = "multi_thread", worker_threads = 10)]
    fn two_clients_one_server_count_to_ten() {
        let _res = env_logger::builder()
            .filter_level(LevelFilter::Info)
            .try_init();

        let server = std::thread::spawn(move || {
            App::new()
                .add_plugins(MinimalPlugins)
                .add_plugin(WebsocketServerPlugin)
                .add_startup_system(startup_server.system())
                .add_system(network_bounce_server.system())
                .insert_resource(NameResource("Server".to_string()))
                .insert_resource(PortResource(8080))
                .run();
            info!("Server thread done");
        });

        let client = std::thread::spawn(move || {
            App::new()
                .add_plugins(MinimalPlugins)
                .add_plugin(WebsocketPlugin)
                .add_startup_system(startup_client.system())
                .add_system(network_bounce_client_multiple.system())
                .insert_resource(NameResource("Client 1".to_string()))
                .insert_resource(PortResource(8080))
                .run();
            info!("Client 1 thread done");
        });

        let client2 = std::thread::spawn(move || {
            App::new()
                .add_plugins(MinimalPlugins)
                .add_plugin(WebsocketPlugin)
                .add_startup_system(startup_client.system())
                .add_system(network_bounce_client_multiple.system())
                .insert_resource(NameResource("Client 2".to_string()))
                .insert_resource(PortResource(8080))
                .run();
            info!("Client 2 thread done");
        });

        server.join().unwrap();
        client.join().unwrap();
        client2.join().unwrap();
    }

    #[test]
    // #[tokio::test(flavor = "multi_thread", worker_threads = 10)]
    fn one_client_one_server_count_to_ten() {
        let _res = env_logger::builder()
            .filter_level(LevelFilter::Info)
            .try_init();

        let server = std::thread::spawn(move || {
            App::new()
                .add_plugins(MinimalPlugins)
                .add_plugin(WebsocketServerPlugin)
                .add_startup_system(startup_server.system())
                .add_system(network_bounce_server.system())
                .insert_resource(NameResource("Server".to_string()))
                .insert_resource(PortResource(8081))
                .run();
            info!("Server thread done");
        });

        let client = std::thread::spawn(move || {
            App::new()
                .add_plugins(MinimalPlugins)
                .add_plugin(WebsocketPlugin)
                .add_startup_system(startup_client.system())
                .add_system(network_bounce_client_single.system())
                .insert_resource(NameResource("Client 1".to_string()))
                .insert_resource(PortResource(8081))
                .run();
            info!("Client 1 thread done");
        });

        server.join().unwrap();
        client.join().unwrap();
    }
}
