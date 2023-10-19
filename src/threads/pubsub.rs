use std::{sync::{Arc, Mutex}, time::{SystemTime, Duration}};

use wamp_core::{Subscribe, WampError, Subscribed, Unsubscribed, Unsubscribe, Event, call};

use crate::error::Error;

use super::{client::Client, events::Events};

pub struct Subscription {
    pub client: Client,
    pub subscribe: Option<Subscribe>,
    pub subscribed: Option<Subscribed>,
    pub routing_ids: Vec<u64>
}

macro_rules! create_callback_handler {
    ($sig:ident, $arg_type:ty, $return_value:ty, $variant:ident, $lock_error:expr, $timeout_error:expr) => {
        pub fn $sig(&mut self, $sig: $arg_type ) -> Result<Result<$return_value, wamp_core::WampError>, $crate::error::Error> {
            let routing_id1 = self.client.new_routing_id();
            let error_routing_id = self.client.new_routing_id();
    
            self.routing_ids.push(routing_id1);
            self.routing_ids.push(error_routing_id);
    
            let routed_reference: Arc<Mutex<Option<$return_value>>> = Arc::new(Mutex::new(None));
            let wamperror: Arc<Mutex<Option<WampError>>> = Arc::new(Mutex::new(None));
    
            let routed_reference2 = routed_reference.clone();

            let request_id = $sig.request_id;

            self.client.on(routing_id1, Events::$variant(Box::new(move |_, result| {
                if request_id == result.request_id {
                    let mut routed_reference = routed_reference2.lock().expect($lock_error);
                    *routed_reference = Some(result);
                };
            })));
    
            let wamperror2 = wamperror.clone();
            self.client.on(error_routing_id, Events::Error(Box::new(move |_, error| {
                if request_id == error.request_id {
                    let mut wamperror = wamperror2.lock().expect($lock_error);
                    *wamperror = Some(error);
                }
            })));
    
            let time_start = SystemTime::now();
            loop {
                if SystemTime::now().duration_since(time_start)? > Duration::from_secs(10) {
                    break Err(Error::TimeOutError($timeout_error))
                };
    
                let result = routed_reference.lock().expect($lock_error).clone();
                let wamperror = wamperror.lock().expect($lock_error).clone();
    
                if let Some(result) = result {
                    break Ok(Ok(result));
                }
    
                if let Some(error) = wamperror {
                    break Ok(Err(error))
                }
            }
        }
    };
}

impl Subscription {
    pub fn new(client: Client) -> Self {
        Subscription {
            client,
            subscribe: None,
            subscribed: None,
            routing_ids: vec![]
        }
    }
    create_callback_handler!(subscribe, Subscribe, Subscribed, Subscribed, "One of the values involved in the subscription callback was poisoned, oh no.", "The client did not receive a `Subscribed` message from the WAMP implementation in less than 10 seconds...");
    create_callback_handler!(unsubscribe, Unsubscribe, Unsubscribed, Unsubscribed, "One of the values involved in the unsubscription callback was poisoned, oh no.", "The client did not receive a `Unsubscribed` message from the WAMP implementation in less than 10 seconds...");
    pub fn events(&mut self, callback: Box<dyn FnMut(Client, Event) + Send> ) -> Result<(), Error> {
        if let Some(subscribed) = &self.subscribed {
            let routing_id = self.client.new_routing_id();
            self.routing_ids.push(routing_id);
            let callback = Arc::new(Mutex::new(callback)); 
            
            let subscribed = subscribed.request_id;

            self.client.on(routing_id, Events::Event(Box::new(move |client, event| {
                if event.subscription == subscribed {
                    let callback = &mut *callback.lock().unwrap();
                    callback(client, event)
                }
            })));
            Ok(())
        } else {
            Err(Error::NoSubscription)
        }
    }
    
}