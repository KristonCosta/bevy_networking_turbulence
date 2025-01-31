use bevy::{
    app::{App, EventReader, ScheduleRunnerSettings},
    core::Time,
    ecs::prelude::*,
    MinimalPlugins,
};
use bevy_networking_turbulence::{NetworkEvent, NetworkResource, NetworkingPlugin, Packet};

use std::{net::SocketAddr, time::Duration};

mod utils;
use utils::{SimpleArgs as Args, parse_simple_args};

const SERVER_PORT: u16 = 14191;

fn main() {
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            std::panic::set_hook(Box::new(console_error_panic_hook::hook));
            console_log::init_with_level(log::Level::Debug).expect("cannot initialize console_log");
        }
        else {
            simple_logger::SimpleLogger::from_env()
            .init()
            .expect("A logger was already initialized");
        }
    }

    App::new()
        // minimal plugins necessary for timers + headless loop
        .insert_resource(ScheduleRunnerSettings::run_loop(Duration::from_secs_f64(
            1.0 / 60.0,
        )))
        .add_plugins(MinimalPlugins)
        // The NetworkingPlugin
        .add_plugin(NetworkingPlugin::default())
        // Our networking
        .insert_resource(parse_simple_args())
        .add_startup_system(startup.system())
        .add_system(send_packets.system())
        .add_system(handle_packets.system())
        .run();
}

fn startup(mut net: ResMut<NetworkResource>, args: Res<Args>) {
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            // set the following address to your server address (i.e. local machine)
            // and remove compile_error! line
            let mut server_address: SocketAddr = "192.168.1.1:0".parse().unwrap();
            compile_error!("You need to set server_address.");
            server_address.set_port(SERVER_PORT);
        } else {
            let ip_address =
                bevy_networking_turbulence::find_my_ip_address().expect("can't find ip address");
            let server_address = SocketAddr::new(ip_address, SERVER_PORT);
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    if args.is_server {
        log::info!("Starting server");
        net.listen(server_address, None, None);
    }
    if !args.is_server {
        log::info!("Starting client");
        net.connect(server_address);
    }
}

fn send_packets(mut net: ResMut<NetworkResource>, time: Res<Time>, args: Res<Args>) {
    if !args.is_server {
        if (time.seconds_since_startup() * 60.) as i64 % 60 == 0 {
            log::info!("PING");
            net.broadcast(Packet::from("PING"));
        }
    }
}
fn handle_packets(
    mut net: ResMut<NetworkResource>,
    time: Res<Time>,
    mut reader: EventReader<NetworkEvent>,
) {
    for event in reader.iter() {
        match event {
            NetworkEvent::Packet(handle, packet) => {
                let message = String::from_utf8_lossy(packet);
                log::info!("Got packet on [{}]: {}", handle, message);
                if message == "PING" {
                    let message = format!("PONG @ {}", time.seconds_since_startup());
                    match net.send(*handle, Packet::from(message)) {
                        Ok(()) => {
                            log::info!("Sent PONG");
                        }
                        Err(error) => {
                            log::info!("PONG send error: {}", error);
                        }
                    }
                }
            }
            _ => {}
        }
    }
}
