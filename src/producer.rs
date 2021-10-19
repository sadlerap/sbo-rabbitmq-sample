use std::{thread, time::Duration};

use amiquip::{Connection, Exchange, Publish};
use color_eyre::{
    eyre::{eyre, Result},
    install,
};

fn main() -> Result<()> {
    // if std::env::var("RUST_BACKTRACE").is_err() {
    //     std::env::set_var("RUST_BACKTRACE", "full");
    // }
    install()?;

    let username = std::env::var("RABBITMQCLUSTER_USERNAME").unwrap_or("guest".into());
    let password = std::env::var("RABBITMQCLUSTER_PASSWORD").unwrap_or("guest".into());
    let hostname = std::env::var("RABBITMQCLUSTER_HOST").map_err(|x| match x {
        std::env::VarError::NotPresent => eyre!("$RABBITMQCLUSTER_HOST not defined"),
        std::env::VarError::NotUnicode(_) => {
            eyre!("$RABBITMQCLUSTER_HOST contains invalid characters")
        }
    })?;
    let port = std::env::var("RABBITMQ_SERVICE_PORT_AMQP")
        .as_ref()
        .map(|x| u16::from_str_radix(x, 10))
        .unwrap_or(Ok(5672u16))?;

    let connection_string = format!("amqp://{}:{}@{}:{}", username, password, hostname, port);
    println!("connecting to: {}", connection_string);
    let mut connection = Connection::insecure_open(&connection_string)?;

    let channel = connection.open_channel(None)?;

    let exchange = Exchange::direct(&channel);

    let message = "hello, world!";
    let queue = "hello";
    loop {
        println!("sending [{}] to queue {}", message, queue);
        exchange.publish(Publish::new(message.as_bytes(), queue))?;
        thread::sleep(Duration::from_secs(10));
    }
}
