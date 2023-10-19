use std::sync::{Arc, Mutex};

use wamp_core::serde_json::Value;

use wamp_core::{messages::*, error::Error};

use super::client::Client;


pub type Callback<T> = Box<dyn FnMut(Client, T) + Send>;
pub type RoutingID = u64;

/// Events
/// This defines the valid frames able to be received by a WAMP client.
/// 
/// This also defines a callback spec to work with.
pub enum Events {
    Abort(Callback<Abort>),
    Goodbye(Callback<Goodbye>),
    Error(Callback<WampError>),
    Event(Callback<Event>),
    Interrupt(Callback<Interrupt>),
    Published(Callback<Published>),
    Registered(Callback<Registered>),
    Result(Callback<WampResult>),
    Subscribed(Callback<Subscribed>),
    Invocation(Callback<Invocation>),
    Unsubscribed(Callback<Unsubscribed>),
    Welcome(Callback<Welcome>),
    Challenge(Callback<Challenge>),
    Extension(Callback<Vec<Value>>),
    Unregistered(Callback<Unregistered>),
    InvalidFrame(Callback<Messages>)
}

macro_rules! event_method {
    () => {
       
    };
}

impl Events {
    pub fn abort() {

    }
}