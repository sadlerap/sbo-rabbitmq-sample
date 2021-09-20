use std::{thread, time::Duration};

use color_eyre::eyre::Result;
use amiquip::{Connection, Exchange, Publish};

fn main() -> Result<()> {
    if std::env::var("RUST_BACKTRACE").is_err() {
        std::env::set_var("RUST_BACKTRACE", "full");
    }
    color_eyre::install()?;

    for (var, value) in std::env::vars() {
        println!("{}: {}", var, value);
    }

    let username = std::env::var("RABBITMQCLUSTER_USERNAME").unwrap_or("guest".into());
    let password = std::env::var("RABBITMQCLUSTER_PASSWORD").unwrap_or("guest".into());
    let hostname = std::env::var("RABBITMQCLUSTER_HOST")?;
    let port = std::env::var("RABBITMQ_SERVICE_PORT_AMQP")
        .as_ref()
        .map(|x| u16::from_str_radix(x, 10))
        .unwrap_or(Ok(5672u16))?;

    let connection_string = format!("amqp://{}:{}@{}:{}", username, password, hostname, port);
    println!("connecting to: {}", connection_string);
    let mut connection = Connection::insecure_open(&connection_string)?;

    let channel = connection.open_channel(None)?;

    let exchange = Exchange::direct(&channel);

    loop {
        exchange.publish(Publish::new("hello there".as_bytes(), "hello"))?;
        thread::sleep(Duration::from_secs(10));
    }
}
