#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use bevy::app::AppExit;
    use bevy::prelude::*;
    use log::{info, LevelFilter};

    struct RunResource {
        running: Arc<Mutex<bool>>,
    }

    struct NameResource(String);

    fn run_bevy(name: String, running: Arc<Mutex<bool>>) {
        App::build()
            .add_plugins(MinimalPlugins)
            .add_startup_system(startup.system())
            .add_system(shutdown.system())
            .insert_resource(RunResource { running })
            .insert_resource(NameResource(name))
            .run();

        fn startup() {
            println!("Server started");
            info!("Server started");
        }

        fn shutdown(
            mut exit: EventWriter<AppExit>,
            run: Res<RunResource>,
            name: Res<NameResource>,
        ) {
            if *run.running.lock().expect("aquire lock") {
                return;
            }
            println!("Shutting down {}", name.0);
            exit.send(AppExit);
        }
    }

    #[test]
    fn run_multiple_bevy_instances() {
        let _res = env_logger::builder()
            .filter_level(LevelFilter::Info)
            .try_init();

        info!("main");

        println!("main");
        let running = Arc::new(Mutex::new(true));

        let t1_running = running.clone();
        let thread = std::thread::spawn(move || {
            run_bevy("instance one ".to_string(), t1_running);
        });
        let t2_running = running.clone();
        let thread2 = std::thread::spawn(move || {
            run_bevy("instance two ".to_string(), t2_running);
        });

        std::thread::sleep(Duration::from_secs(1));
        *running.lock().expect("aquire lock for shutting down") = false;
        thread.join().unwrap();
        thread2.join().unwrap();
        println!("end");
    }
}
