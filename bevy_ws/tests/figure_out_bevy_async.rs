#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use bevy::app::AppExit;
    use bevy::prelude::*;
    use bevy::tasks::IoTaskPool;
    use log::LevelFilter;

    struct Running(Arc<Mutex<bool>>);

    fn shutdown(running: Res<Running>, mut exit: EventWriter<AppExit>) {
        if !*running.0.lock().unwrap() {
            exit.send(AppExit);
        }
    }

    fn setup_async_countdown(
        running: Res<Running>,
        task_pool: Res<IoTaskPool>,
        _exit: EventWriter<AppExit>,
    ) {
        let running_clone = running.0.clone();
        let task = task_pool.spawn(async move {
            info!("Inside task");
            async_io::Timer::after(core::time::Duration::from_secs(1)).await;
            *running_clone.lock().unwrap() = false;
        });
        task.detach();
    }

    #[test]
    fn figure_out_async() {
        let _res = env_logger::builder()
            .filter_level(LevelFilter::Info)
            .try_init();

        App::new()
            .add_plugins(MinimalPlugins)
            .insert_resource(Running(Arc::new(Mutex::new(true))))
            .add_system(shutdown.system())
            .add_startup_system(setup_async_countdown.system())
            .run();
    }
}
