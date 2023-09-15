use esp_idf_svc::eventloop::{Background, EspEventLoop, EspSubscription, User};
use esp_idf_sys::EspError;

pub mod discord;
pub mod mqtt;

pub trait LoopRelay {
    fn post(&self, msg: &str) -> Result<(), ()>;
    // fn listen_queue<T: Display>(&self, rx: Receiver<T>);
    fn listen_on_event_loop(
        &self,
        event_loop: EspEventLoop<User<Background>>,
    ) -> Result<EspSubscription<User<Background>>, EspError>;
}

pub trait Source {
    fn publish_to_loop(&self, event_loop: &EspEventLoop<User<Background>>) -> Result<(), EspError>;
}
