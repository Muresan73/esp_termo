use std::sync::{Arc, Mutex};
use std::time::Duration;

use embedded_svc::utils::asyncify::Asyncify;
use esp_idf_svc::timer::EspTimerService;
use esp_idf_svc::{
    notify::EspNotify,
    sntp::{self},
    systime::EspSystemTime,
    timer::EspTimer,
};
use esp_idf_sys::EspError;
use log::info;

const ONE_DAY: u64 = 86400;

pub async fn get_time(mut callback: impl FnMut() + Send + 'static) -> Result<(), EspError> {
    update_current_time_async().await;
    info!("SNTP updated");

    let clock = EspSystemTime {};
    let now = clock.now();
    let remaining = get_time_until_8(now.as_secs());
    let timer_service = EspTimerService::new()?;

    callback();

    let mut timer = timer_service.into_async().timer()?;
    timer.after(Duration::from_secs(remaining))?.await;
    callback();

    let stream = timer.every(Duration::from_secs(ONE_DAY))?;

    loop {
        info!("Waiting for timer");
        stream.tick().await;
        callback();
    }
}

async fn update_current_time_async() {
    let now = EspSystemTime {}.now();
    let notification = EspNotify::new(&Default::default()).unwrap();
    let notification_a = notification.clone().into_async();

    let sntp = sntp::EspSntp::new_with_callback(&Default::default(), move |now| {
        notification.post(&(now.as_secs() as u32)).unwrap();
    });

    let mut sub = notification_a.subscribe().unwrap();
    let val = sub.recv().await;
}

fn get_time_until_8(current: u64) -> u64 {
    let today = current % ONE_DAY;
    let remaining = ONE_DAY as i64 + (28800 - today as i64);
    remaining as u64
}

#[cfg(test)]
mod tests {
    use crate::trigger::timer::get_time_until_8;

    #[test]
    fn time() {
        let result = get_time_until_8(1694894400);
        assert_eq!(result, 43200);
    }
}
