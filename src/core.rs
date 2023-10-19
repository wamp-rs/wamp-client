use std::sync::{Mutex, Arc};
use wamp_core::tungstenite::WebSocket;
use wamp_core::tungstenite::stream::MaybeTlsStream;
use std::net::TcpStream;

pub(crate) type Socket = Arc<Mutex<WebSocket<MaybeTlsStream<TcpStream>>>>;