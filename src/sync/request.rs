use std::str::FromStr;
use wamp_core::tungstenite;
use wamp_core::http::{Uri, Version, self};
use wamp_core::tungstenite::{client::IntoClientRequest, handshake::client::generate_key};

pub struct WampRequest<U: ToString, P: ToString> {
    pub uri: U,
    pub protocol: P
}

impl<U: ToString, P: ToString> IntoClientRequest for WampRequest<U, P> {
    fn into_client_request(self) -> tungstenite::Result<tungstenite::handshake::client::Request> {
        let uri = Uri::from_str(&self.uri.to_string())?;
        let req = http::Request::builder()
            .uri(self.uri.to_string())
            .version(Version::HTTP_11)
            .header("Sec-WebSocket-Protocol", self.protocol.to_string())
            .header("Sec-WebSocket-Key", generate_key())
            .header("Connection", "Upgrade")
            .header("Upgrade", "websocket")
            .header("Sec-WebSocket-Version", 13)
            .header("Host", uri.host().unwrap());
        Ok(tungstenite::handshake::client::Request::from(req.body(())?))
    }
}