pub mod timer;

trait Trigger {
    fn handle_event(
        &self,
        // event_handler: impl Fn(Result<MqttCommand, CommandError>) + Send + 'static,
    );
}
