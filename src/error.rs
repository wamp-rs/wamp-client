use std::time::SystemTimeError;

use wamp_core::WampError;




pub enum Error {
    WampCoreError(wamp_core::Error),
    TimeOutError(&'static str),
    SystemTimeError(SystemTimeError),
    NoSubscription,
    WampMessageError(WampError)
}

impl From<wamp_core::Error> for Error {
    fn from(value: wamp_core::Error) -> Self {
        Error::WampCoreError(value)
    }
}

impl From<WampError> for Error {
    fn from(value: WampError) -> Self {
        Error::WampMessageError(value)
    }
}

impl From<SystemTimeError> for Error {
    fn from(value: SystemTimeError) -> Self {
        Error::SystemTimeError(value)
    }
}