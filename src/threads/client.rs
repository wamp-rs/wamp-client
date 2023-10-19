

use std::{sync::{Arc, Mutex}, convert::TryInto, thread::JoinHandle};
use wamp_core::{messages::*, serde_json::from_str, tungstenite::client, subscribe, unsubscribe};
use std::thread::spawn;
use wamp_core::{Error, http::Response, tungstenite::{connect, Message}, WampMessage};
use crate::{core::Socket, sync::WampRequest};
use super::events::Events;

#[derive(Clone)]
pub struct Client {
    pub socket: Socket,
    pub request_id: Arc<Mutex<u64>>,
    pub routing_id: Arc<Mutex<u64>>,
    pub events: Arc<Mutex<Vec<Arc<Mutex<(u64, Events)>>>>>,
}

impl Client {
    pub fn connect<U: ToString, P: ToString>(
        request: WampRequest<U, P>,
    ) -> Result<(Client, Response<Option<Vec<u8>>>), Error> {
        let (socket, response) = connect(request)?;
        let socket = Arc::new(Mutex::new(socket));
        let request_id = Arc::new(Mutex::new(0));
        let routing_id = Arc::new(Mutex::new(0));
        let events = Arc::new(Mutex::new(vec![]));
        Ok((
            Client {
                socket,
                request_id,
                routing_id,
                events
            },
            response,
        ))
    }

    pub fn on(&self, routing_id: u64, event: Events) {
        let mut events = self.events.lock().expect("Events mutex guard poisoned");
        events.push(Arc::new(Mutex::new((routing_id, event))));
    }

    pub fn send<T: WampMessage + TryInto<Message>>(&self, message: T) -> Result<(), Error>
    where
        Error: From<<T as TryInto<Message>>::Error>,
    {
        let socket = &mut *self
            .socket
            .lock()
            .expect("WebSocket mutex Poisoned during message sending.");
        Ok(socket.send(message.try_into()?)?)
    }

    pub fn new_routing_id(&self) -> u64 {
        let mut request_id = *self.request_id.lock().unwrap();
        request_id = request_id + 1;
        request_id
    }

    pub fn new_request_id(&self) -> u64 {
        let mut request_id = *self.request_id.lock().unwrap();
        request_id = request_id + 1;
        request_id
    }

    //pub fn create_callback(&self, routing_ids: Vec<u64>, on_callback: Box<dyn FnOnce(Client)>) -> Box<dyn FnOnce()> {
    //  let client = self.clone();
    //  Box::new(move || {
    //  client.remove_callbacks(routing_ids);
    //  on_callback(self.clone());
    //})
    //}

    pub fn remove_callbacks(&self, routing_ids: Vec<u64>) {
        let events = &mut *self.events.lock().unwrap();
        events.retain(|callback| { 
            let (routing, _) = *callback.lock().unwrap();

                if routing_ids.contains(&routing) {
                    return false
                }
                true

        })
    }

    //pub fn subscribe(subscribe: Subscribe) -> Result<Subscribed> {
//
    //}
    /*
    pub fn subscribe(
        &mut self,
        subscribe: Subscribe,
        callback: Box<dyn FnMut(Client, Event, Subscribed, Box<dyn FnOnce()>) + Send>,
    ) -> Result<(), Error> {
        let request_id = subscribe.request_id;
        self.send(subscribe)?;
        let callback = Arc::new(Mutex::new(callback));
        let subscription_routing_id = self.new_routing_id();
        Ok(
            self.on(subscription_routing_id, Events::Subscribed(Box::new(move |client, subscription| {
                if subscription.request_id == request_id {
                    let callback = callback.clone();
                    let request_id = subscription.subscription;
                    let event_routing_id = client.new_routing_id();
                    client.on(event_routing_id, Events::Event(Box::new(move |client, event| {
                        if request_id == event.subscription {
                            let mut callback = callback.lock().unwrap();
                            let unsubscribe = client.create_callback(vec![event_routing_id, subscription_routing_id], Box::new(|client| {
                                let routing_id = client.new_routing_id();

                                client.on(routing_id, Events::Unsubscribed(Box::new(move |client, unsubscribed| {
                                    
                                })))
                            }));
                            callback(client.clone(), event, subscription.clone(), unsubscribe)
                        }
                    }))) 
                }
            }))),
        )
    }
    */

    pub fn event_loop(&mut self) -> Result<(), Error> {
        loop {
            self.read_then_run_event()?;
            //let event = self.read_then_run_event()?;
            //match event {
            //    Some((message, joiner)) => {
            //
            //    },
            //    _ => {}
            //}
        }
    }

    pub fn read_then_run_event(&mut self) -> Result<Option<(Messages, JoinHandle<()>)>, Error> {
        match self.read()? {
            Some(message) => Ok(Some(self.run_events(message)?)),
            None => Ok(None),
        }
    }

    pub fn run_events(&mut self, message: Messages) -> Result<(Messages, JoinHandle<()>), Error> {
        let events = (&self.events).clone();
        let arc_client = Client::from(self);

        macro_rules! run_events {
            ($events:ident, $value:expr) => {{
                let arc_client = arc_client.clone();
                let events = events.clone();
                Ok((
                    message,
                    spawn(move || {
                        let mut events = events.lock().unwrap();
                        for event in events.iter_mut() {
                            let (_, event) = &mut *event.lock().unwrap();
                            if let Events::$events(callback) = event {
                                callback(arc_client.clone(), $value.clone());
                            }
                        }
                    }),
                ))
            }};
        }

        match message.clone() {
            Messages::Abort(abort) => run_events!(Abort, abort),
            Messages::Challenge(challenge) => run_events!(Challenge, challenge),
            Messages::Error(error) => run_events!(Error, error),
            Messages::Event(event) => run_events!(Event, event),
            Messages::Goodbye(goodbye) => run_events!(Goodbye, goodbye),
            Messages::Interrupt(interrupt) => run_events!(Interrupt, interrupt),
            Messages::Invocation(invocation) => run_events!(Invocation, invocation),
            Messages::Published(published) => run_events!(Published, published),
            Messages::Registered(registered) => run_events!(Registered, registered),
            Messages::Result(result) => run_events!(Result, result),
            Messages::Subscribed(subscribed) => run_events!(Subscribed, subscribed),
            Messages::Unregistered(unregistered) => run_events!(Unregistered, unregistered),
            Messages::Unsubscribed(unsubscribed) => run_events!(Unsubscribed, unsubscribed),
            Messages::Welcome(welcome) => run_events!(Welcome, welcome),
            Messages::Extension(extension) => run_events!(Extension, extension),
            _ => Err(Error::InvalidFrameReceived(message)),
        }
    }

    /// # Read
    /// Read a frame from tungstenite and convert to WAMP messages.
    pub fn read(&mut self) -> Result<Option<Messages>, Error> {
        match self.socket.lock().unwrap().read().unwrap() {
            Message::Text(message) => Ok(Some(from_str(&message)?)),
            _ => Ok(None),
        }
    }
}

impl From<&Client> for Client {
    fn from(value: &Client) -> Self {
        Self {
            socket: value.socket.clone(),
            request_id: value.request_id.clone(),
            events: value.events.clone(),
            routing_id: value.routing_id.clone()
        }
    }
}

impl From<&mut Client> for Client {
    fn from(value: &mut Client) -> Self {
        Self {
            socket: value.socket.clone(),
            request_id: value.request_id.clone(),
            events: value.events.clone(),
            routing_id: value.routing_id.clone()
        }
    }
}