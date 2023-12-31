use chrono::{Local, NaiveTime, Timelike};
use embedded_svc::utils::asyncify::timer::AsyncTimerService;
use embedded_svc::utils::asyncify::Asyncify;
use esp_idf_hal::task::asynch::Notification;
use esp_idf_svc::sntp::{self};
use esp_idf_svc::timer::EspTimerService;
use esp_idf_sys::EspError;
use log::info;
use std::time::Duration;

#[derive(Debug, thiserror::Error)]
pub enum TimerError {
    #[error("chrono to std conversion error")]
    ConversionError,
    #[error("EspError error")]
    TimerError(#[from] EspError),
}

pub async fn shedule_event(mut callback: impl FnMut()) -> Result<(), TimerError> {
    update_current_time_async().await;
    info!("SNTP updated");
    let remaining = get_duration_until_next(8).ok_or(TimerError::ConversionError)?;

    let timer_service = get_timer()?;
    let mut timer = timer_service.timer()?;
    callback();

    showtime();
    timer.after(remaining).await?;
    callback();

    let stream = timer.every(chrono::Duration::days(1).to_std().expect("Can't fail"))?;

    loop {
        info!("Waiting for timer");
        stream.tick().await;
        callback();
    }
}

pub fn get_timer() -> Result<AsyncTimerService<EspTimerService<esp_idf_svc::timer::Task>>, EspError>
{
    let timer_service = EspTimerService::new()?;
    let timer = timer_service.into_async();
    Ok(timer)
}

pub async fn safe_sleep(duration: Duration) {
    if let Ok(ts) = get_timer() {
        if let Ok(mut sleep) = ts.timer() {
            sleep.after(duration).await.ok();
        }
    }
}

pub fn showtime() {
    let now = Local::now();

    let (is_pm, hour) = now.hour12();
    info!(
        "The current time is {:02}:{:02}:{:02} {}",
        hour,
        now.minute(),
        now.second(),
        if is_pm { "PM" } else { "AM" }
    );
}

pub fn get_duration_until_next(hour: u32) -> Option<Duration> {
    use chrono::Duration;
    let current_time = Local::now().time();
    let target_time = NaiveTime::from_hms_opt(hour, 0, 0)?;

    let elapsed = if current_time <= target_time {
        // If current time is before 8 AM, calculate duration until 8 AM of the same day
        target_time - current_time
    } else {
        // If current time is after 8 AM, calculate duration until 8 AM of the next day
        Duration::days(1) - (current_time - target_time)
    };
    info!(
        "Time until next 8 AM:{}:{}:{} ",
        elapsed.num_hours(),
        elapsed.num_minutes() % 60,
        elapsed.num_seconds() % 60,
    );
    elapsed.to_std().ok()
}

async fn update_current_time_async() {
    let notification = Notification::new();

    let _sntp = sntp::EspSntp::new_with_callback(&Default::default(), |_now| {
        notification.notify_lsb();
    });

    notification.wait().await;
}
