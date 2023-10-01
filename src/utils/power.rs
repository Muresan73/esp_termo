use std::time::Duration;

use esp_idf_hal::gpio::{AnyInputPin, PinDriver};
use log::warn;

pub fn enter_deep_sleep_with_buttonwake(wakeup_pin: AnyInputPin, sleep_time: Duration) {
    let wakeup_pin = PinDriver::input(wakeup_pin).expect("wakeup pin sleep");
    unsafe {
        esp_idf_sys::esp_sleep_enable_ext0_wakeup(wakeup_pin.pin(), 0);
    }
    warn!("entering deep sleep");
    unsafe {
        // TODO: measure current draw vs gpio_deep_sleep_hold_en
        //esp_idf_sys::rtc_gpio_hold_en(led.pin());
        //esp_idf_sys::gpio_deep_sleep_hold_en()
        // TODO see if these need to be configured or if it makes a difference at all
        // esp_sleep_pd_config(ESP_PD_DOMAIN_RTC_PERIPH, ESP_PD_OPTION_OFF);
        // esp_sleep_pd_config(ESP_PD_DOMAIN_RTC_SLOW_MEM, ESP_PD_OPTION_OFF);
        // esp_sleep_pd_config(ESP_PD_DOMAIN_RTC_FAST_MEM, ESP_PD_OPTION_OFF);
        // esp_sleep_pd_config(ESP_PD_DOMAIN_XTAL, ESP_PD_OPTION_OFF);
        esp_idf_sys::esp_deep_sleep(sleep_time.as_micros() as u64);
    }
    // unreachable!("we will be asleep by now");
}

pub fn enter_light_sleep(sleep_time: Duration) {
    // make sure that services with timers are freed like wifi
    warn!("enabling timer wakeup");
    unsafe {
        esp_idf_sys::esp_sleep_enable_timer_wakeup(sleep_time.as_micros() as u64);
    }
    warn!("entering sleep");
    unsafe {
        esp_idf_sys::esp_light_sleep_start();
    }
    std::thread::sleep(Duration::from_secs(2));
    warn!("woke up from sleep");
}
