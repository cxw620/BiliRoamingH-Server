use serde::{Deserialize, Serialize};

use std::{net::SocketAddr, sync::OnceLock};

static CONFIG_VERISON: &'static str = "0.1.0";

/// The version of the server.
pub static SERVER_VERSION: &'static str = concat!(
    "BiliRoamingH-Server/",
    include_str!(concat!(env!("OUT_DIR"), "/VERSION"))
);

pub static CONFIG_SERVER: OnceLock<ServerConfigServer> = OnceLock::new();

#[tracing::instrument]
pub fn init_config() {
    let _ = CONFIG_SERVER.set(ServerConfigServer::default());
}

pub struct ServerConfig {
    pub config_ver: &'static str,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct ServerConfigServer {
    /// HTTP/gRPC server addr
    pub listen: SocketAddr,
}

impl Default for ServerConfigServer {
    fn default() -> Self {
        Self {
            listen: ([127, 0, 0, 1], 2663).into(),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::server::config::SERVER_VERSION;

    #[test]
    fn test() {
        println!("{}", SERVER_VERSION)
    }
}
