use crate::core::{error::Error, messages::*};
use tungstenite::Message;

use super::client::{self};
use super::super::core::Socket;

pub(crate) type CallBack<T> = Box<dyn FnMut(Context, T) -> Context>;
pub(crate) type CallBackResult<T> = CallBack<Result<T, WampError>>;
pub(crate) type CallBackVec<K, V> = Vec<(K, CallBack<V>)>;
pub(crate) type CallBackVecResult<K, V> = CallBackVec<K, Result<V, WampError>>; 

macro_rules! create_push_methods {
    (
        $(#[$attr:meta])*
        {$method_name: ident, $vec_name: ident, $var_type: ident, $callback: ty}
    ) => {
        $(#[$attr])*
        pub fn $method_name(
            &mut self,
            $method_name: $var_type,
            callback: $callback,
        ) -> Result<(), Error> {
            self.send($method_name.clone())?;
            Ok(self.$vec_name.push(($method_name, callback)))
        }
    };

    (
        $(#[$attr:meta])*
        {$method_name: ident, $vec_name: ident, $var_type: ident, $callback: ty, no_send}
    ) => {
        $(#[$attr])*
        pub fn $method_name(
            &mut self,
            $method_name: $var_type,
            callback: $callback,
        ) -> Result<(), Error> {
            Ok(self.$vec_name.push(($method_name, callback)))
        }
    };
}

macro_rules! create_find_methods {
    ($method_name: ident, $var_name: ident, $vec_name: ident, $return_type: ident, $var_type: ident) => {
        pub(crate) fn $method_name(
            &mut self,
            $var_name: &$var_type,
        ) -> Option<&mut ($return_type, CallBackResult<$var_type>)> {
            self.$vec_name
                .iter_mut()
                .find(|i| i.0.request_id == $var_name.request_id)
        }
    };
    (event: $method_name: ident, $var_name: ident, $vec_name: ident, $return_type: ident, $var_type: ident) => {
        pub(crate) fn $method_name(
            &mut self,
            $var_name: &$var_type,
        ) -> Option<&mut ($return_type, CallBack<$var_type>)> {
            self.$vec_name
                .iter_mut()
                .find(|i| i.0.subscription == $var_name.subscription)
        }
    };
}

macro_rules! create_find_by_error_method {
    ($method_name: ident, $method_type: ident, $vec_name: ident, $return_type: ident) => {
        pub fn $method_name(
            &mut self,
            error: &WampError,
        ) -> Option<&mut ($method_type, CallBackResult<$return_type>)> {
            if let Some(info) = self
                .$vec_name
                .iter_mut()
                .find(|($method_name, _)| $method_name.request_id == error.request_id)
            {
                Some(info)
            } else {
                None
            }
        }
    };
}

pub struct Context {
    pub socket: Option<Socket>,
    pub(crate) registrations: CallBackVecResult<Register, Registered>,
    pub(crate) unregistrations: CallBackVecResult<Unregister, Unregistered>,
    pub(crate) subscriptions: CallBackVecResult<Subscribe, Subscribed>,
    pub(crate) unsubscriptions: CallBackVecResult<Unsubscribe, Unsubscribed>,
    pub(crate) publications: CallBackVecResult<Publish, Published>,
    pub(crate) calls: CallBackVecResult<Call, WampResult>,
    pub(crate) events: CallBackVec<Subscribed, Event>,
    pub(crate) invocations: CallBackVecResult<Registered, Invocation>,
    //pub(crate) errors: CallBackVecResult<Messages, WampError>,
    pub(crate) messages: Vec<Message>,
    pub(crate) cancelations: CallBackVecResult<Cancel, Interrupt>,
}

impl Context {
    /// # Create a new context object
    /// Create a new context object, there is a main one held by the client,
    /// and new ones are created to be shared among the callbacks that get
    /// passed back along to the main client.
    ///
    /// Primarily internal currently, im working on support to better manipulate this process.
    /// ## Examples
    /// ```
    /// use wamp::client::context::Context;
    ///
    /// // Create a new context with no socket.
    /// let context = Context::new(None);
    /// ```
    pub fn new(socket: Option<Socket>) -> Self {
        Self {
            socket: socket,
            registrations: vec![],
            unregistrations: vec![],
            subscriptions: vec![],
            unsubscriptions: vec![],
            publications: vec![],
            calls: vec![],
            events: vec![],
            invocations: vec![],
            messages: vec![],
            //errors: vec![],
            cancelations: vec![],
        }
    }

    /// # [TODO]: Context::new_with_capacity
    /// This method allows for creating restricted contexts with limited capacitys on each vec.
    /// It only allows for setting one capacity for all vecs currently, and im explorting other ways
    /// to implement this functionality, i may create an option struct for people to pass to it, and
    /// then a callback method for creating context objects so people can fully customize it before
    /// the context object is then passed along to the rest of the context routing.
    ///
    /// Also after adding a doc test this does not even appear to panic on capacity exceeded which is rather concerning.
    ///
    /// I may need to add functionality for erroring on capacity exceeded.
    /// ## Examples
    /// ```
    /// use wamp::client::context::Context;
    /// use wamp::core::messages::Call;
    /// use wamp::call;
    ///
    /// let mut context = Context::new_with_capacity(None, 10);
    ///
    /// for i in 1..50 {
    ///     if i > 10 {
    ///         context.send(call!("topic")).unwrap();
    ///     } else {
    ///         context.send(call!("topic")).unwrap();
    ///     }
    /// }
    /// ```
    pub fn new_with_capacity(socket: Option<Socket>, capacity: usize) -> Self {
        Self {
            socket: socket,
            registrations: Vec::with_capacity(capacity),
            unregistrations: Vec::with_capacity(capacity),
            subscriptions: Vec::with_capacity(capacity),
            unsubscriptions: Vec::with_capacity(capacity),
            publications: Vec::with_capacity(capacity),
            calls: Vec::with_capacity(capacity),
            events: Vec::with_capacity(capacity),
            invocations: Vec::with_capacity(capacity),
            messages: Vec::with_capacity(capacity),
            //errors: Vec::with_capacity(capacity),
            cancelations: Vec::with_capacity(capacity),
        }
    }

    /// # Context Send
    /// Takes any value with the trait `TryFrom<tungstenite::Message>` trait implemented.
    ///
    /// All message types have this, as well as the `Messages` enum, so you can send any of those
    /// using this method.
    ///
    /// ## Example
    /// ```
    /// use wamp::client::context::Context;
    /// use wamp::hello;
    /// use wamp::core::messages::Hello;
    ///
    /// // Create a context with no socket
    /// let mut ctx = Context::new(None);
    ///
    /// // If there is no socket, it pushes the message to an array to be pulled back to the main scope, which is why this does not error.
    /// ctx.send(hello!("some.realm.uri")).unwrap();
    /// ```
    pub fn send<T: TryInto<Message>>(&mut self, message: T) -> Result<(), Error>
    where
        Error: From<<T as TryInto<Message>>::Error>,
    {
        if let Some(socket) = &self.socket {
            let socket = &mut *socket.lock().unwrap();
            Ok(socket.send(message.try_into()?)?)
        } else {
            Ok(self.messages.push(message.try_into()?))
        }
    }

    create_find_by_error_method!(
        find_by_error_unsubscribe,
        Unsubscribe,
        unsubscriptions,
        Unsubscribed
    );
    create_find_by_error_method!(
        find_by_error_subscribe,
        Subscribe,
        subscriptions,
        Subscribed
    );
    create_find_by_error_method!(find_by_error_publish, Publish, publications, Published);
    create_find_by_error_method!(find_by_error_register, Register, registrations, Registered);
    create_find_by_error_method!(
        find_by_error_unregister,
        Unregister,
        unregistrations,
        Unregistered
    );
    create_find_by_error_method!(find_by_error_cancel, Cancel, cancelations, Interrupt);
    create_find_by_error_method!(find_by_error_call, Call, calls, WampResult);

    create_push_methods!(
        /// # Context Registration
        /// Method that allows for registering easily with a callback to the wamp client.
        ///
        /// ## Examples
        /// ```
        /// use wamp::core::messages::Register;
        /// use wamp::client::context::Context;
        /// use wamp::register;
        ///
        /// // Construct a context with no socket
        /// let mut context = Context::new(None);
        ///
        /// // Dont forget to send your register message with the callback registration!
        /// context.register(register!("procedure"), Box::new(|ctx, registered| {
        ///     // This never happens in this test, but if it did it would allow you to access the values returned.
        ///     // You must always return the created context object
        ///     ctx
        /// })).unwrap();
        /// ```
        {
            register,
            registrations,
            Register,
            CallBackResult<Registered>
        }
    );
    create_push_methods!(
        /// # Context Unregistration
        /// Method that allows for unregistering easily with a callback to the wamp client.
        /// 
        /// ## Examples
        /// ```
        /// use wamp::core::messages::Unregister;
        /// use wamp::client::context::Context;
        /// use wamp::unregister;
        /// 
        /// // Construct a context with no socket
        /// let mut context = Context::new(None);
        /// 
        /// // Dont forget to send your unregister message with the callback registration!
        /// context.unregister(unregister!(2), Box::new(|ctx, unregistered| {
        ///     // This never happens in this test, but if it did it would allow you to access the values returned.
        ///     // You must always return the created context object
        ///     ctx
        /// })).unwrap();
        /// 
        /// ```
        {
            unregister,
            unregistrations,
            Unregister,
            CallBackResult<Unregistered>
        }
    );
    create_push_methods!(
        /// # Context Event Listener
        /// Method that allows for listening to Events after subscribing to a topic.
        /// 
        /// ## Examples
        /// ```
        /// use wamp::core::messages::Subscribe;
        /// use wamp::client::context::Context;
        /// use wamp::subscribe;
        /// 
        /// // Construct a context with no socket
        /// let mut context = Context::new(None);
        /// 
        /// // Dont forget to send your unsubscribe message with the callback registration!
        /// context.subscribe(subscribe!("topic"), Box::new(|mut ctx, subscribed| {
        ///     // This never happens in this test, but if it did it would allow you to access the values returned.
        ///     // You must always return the created context object
        ///     
        ///     // Using this mutable conext object you can now register your event listener 
        ///     // We unwrap the subscribed result, since it can be an error
        ///     ctx.event(subscribed.unwrap(), Box::new(|ctx, event| {
        ///         // This callback also never happens but would be passed back to the client context to be used
        ///         // for all events that come the way of the client.
        ///         ctx // Always return context :)
        ///     })).unwrap();
        ///     ctx
        /// })).unwrap();
        /// ```
        {
            event, 
            events, 
            Subscribed, 
            CallBack<Event>, 
            no_send
        }
    );

    create_push_methods!(
        /// # Context Unsubscribe
        /// Method that allows for unsubscribing easily with a callback to the wamp client.
        /// 
        /// ## Examples
        /// ```
        /// use wamp::core::messages::Unsubscribe;
        /// use wamp::client::context::Context;
        /// use wamp::unsubscribe;
        /// 
        /// // Construct a context with no socket
        /// let mut context = Context::new(None);
        /// 
        /// // Dont forget to send your unsubscribe message with the callback registration!
        /// context.unsubscribe(unsubscribe!(1), Box::new(|ctx, unsubscribed| {
        ///     // This never happens in this test, but if it did it would allow you to access the values returned.
        ///     // You must always return the created context object
        ///     ctx
        /// })).unwrap();
        /// ```
        {
            unsubscribe,
            unsubscriptions,
            Unsubscribe,
            CallBackResult<Unsubscribed>
        }
    );

    create_push_methods!(
        /// # Context Subscribe
        /// Method that allows for subscribing to a topic with a callback.
        /// 
        /// ## Examples
        /// ```
        /// use wamp::core::messages::Subscribe;
        /// use wamp::client::context::Context;
        /// use wamp::subscribe;
        /// 
        /// // Construct a context with no socket
        /// let mut context = Context::new(None);
        /// 
        /// // Dont forget to send your unsubscribe message with the callback registration!
        /// context.subscribe(subscribe!("topic"), Box::new(|mut ctx, subscribed| {
        ///     // This never happens in this test, but if it did it would allow you to access the values returned.
        ///     // You must always return the created context object
        /// 
        ///     ctx.event(subscribed.unwrap(), Box::new(|ctx, event| {
        ///         ctx // Always return context :)
        ///     })).unwrap();
        /// 
        ///     ctx
        /// })).unwrap();
        /// ```
        {
            subscribe,
            subscriptions,
            Subscribe,
            CallBackResult<Subscribed>
        }
    );
    create_push_methods!(
        /// # Context Publish
        /// Method that allows for publishing easily with a callback to the wamp client.
        ///
        /// ## Examples
        /// ```
        /// use wamp::core::messages::Publish;
        /// use wamp::client::context::Context;
        /// use wamp::publish;
        ///
        /// // Construct a context with no socket
        /// let mut context = Context::new(None);
        ///
        /// // Dont forget to send your publish message with the callback registration!
        /// context.publish(publish!("topic"), Box::new(|ctx, published| {
        ///     // This never happens in this test, but if it did it would allow you to access the values returned.
        ///     // You must always return the created context object
        ///     ctx
        /// })).unwrap();
        /// ```
        {
            publish, 
            publications, 
            Publish, 
            CallBackResult<Published>
        }
    );
    create_push_methods!(
        /// # Context Call
        /// Method that allows for calling easily with a callback to the wamp client.
        ///
        /// ## Examples
        /// ```
        /// use wamp::core::messages::Call;
        /// use wamp::client::context::Context;
        /// use wamp::call;
        ///
        /// // Construct a context with no socket
        /// let mut context = Context::new(None);
        ///
        /// // Dont forget to send your call message with the callback registration!
        /// context.call(call!("procedure"), Box::new(|ctx, result| {
        ///     // This never happens in this test, but if it did it would allow you to access the values returned.
        ///     // You must always return the created context object
        ///     ctx
        /// })).unwrap();
        /// ```
        {
            call, 
            calls, 
            Call, 
            CallBackResult<WampResult>
        }
    );
    create_push_methods!(
        /// # Context Invocation Listener
        /// Method that allows for invocation listening after 
        ///
        /// ## Examples
        /// ```
        /// use wamp::core::messages::{Register, Registered, Invocation};
        /// use wamp::client::context::Context;
        /// use wamp::register;
        ///
        /// // Construct a context with no socket
        /// let mut context = Context::new(None);
        ///
        /// // Dont forget to send your register message with the callback registration!
        /// context.register(register!("procedure"), Box::new(|mut ctx, registered| {
        ///     // This never happens in this test, but if it did it would allow you to access the values returned.
        ///     // You must always return the created context object
        ///     ctx.invocation(registered.unwrap(), Box::new(|ctx, invocation| {
        ///         // Listen for invocations here
        ///         ctx
        ///     })).unwrap();
        ///     ctx
        /// })).unwrap();
        /// ```
        {
            invocation,
            invocations,
            Registered,
            CallBackResult<Invocation>
        }
    );
    create_push_methods!(
        /// # Context Cancel
        /// Method that allows for canceling a call easily with a callback to the wamp client.
        ///
        /// ## Examples
        /// ```
        /// use wamp::core::messages::Cancel;
        /// use wamp::client::context::Context;
        /// use wamp::cancel;
        ///
        /// // Construct a context with no socket
        /// let mut context = Context::new(None);
        ///
        /// // Dont forget to send your cancel message with the callback registration!
        /// context.cancel(cancel!(1), Box::new(|ctx, interrupt| {
        ///     // This never happens in this test, but if it did it would allow you to access the values returned.
        ///     // You must always return the created context object
        ///     ctx
        /// })).unwrap();
        /// ```
        {
            cancel, 
            cancelations, 
            Cancel, 
            CallBackResult<Interrupt>
        }
    );

    create_find_methods!(
        find_register,
        registered,
        registrations,
        Register,
        Registered
    );
    create_find_methods!(
        find_unregister,
        unregister,
        unregistrations,
        Unregister,
        Unregistered
    );
    create_find_methods!(event: find_event, event, events, Subscribed, Event);
    create_find_methods!(
        find_unsubscribe,
        unsubscribe,
        unsubscriptions,
        Unsubscribe,
        Unsubscribed
    );
    create_find_methods!(
        find_subscribe,
        subscribe,
        subscriptions,
        Subscribe,
        Subscribed
    );
    create_find_methods!(find_publish, publish, publications, Publish, Published);
    create_find_methods!(find_call, call, calls, Call, WampResult);
    create_find_methods!(
        find_invocation,
        invocation,
        invocations,
        Registered,
        Invocation
    );
    create_find_methods!(find_cancel, interrupt, cancelations, Cancel, Interrupt);

    pub fn extend(&mut self, ctx: Context) {
        self.registrations.extend(ctx.registrations);
        self.unregistrations.extend(ctx.unregistrations);
        self.events.extend(ctx.events);
        self.unsubscriptions.extend(ctx.unsubscriptions);
        self.subscriptions.extend(ctx.subscriptions);
        self.publications.extend(ctx.publications);
        self.calls.extend(ctx.calls);
        self.invocations.extend(ctx.invocations);
        self.messages.extend(ctx.messages);
    }
}
