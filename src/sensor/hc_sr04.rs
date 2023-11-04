//! Source [Code](https://github.com/marcoradocchia/hc-sr04) for the **HC-SR04** ultrasonic sensor driver.\
//! Modified to work with the ESP32.

use esp_idf_hal::gpio::{Input, Output, Pin, PinDriver};
use log::info;
use std::{
    thread,
    time::{Duration, Instant},
};

/// Measuring unit (defaults to [`Unit::Meters`]).
pub enum Unit {
    Millimeters,
    Centimeters,
    Decimeters,
    Meters,
}

/// **HC-SR04** ultrasonic sensor on *ESP32*.
///
/// # Fileds
///
/// - `trig`: **TRIGGER** output GPIO pin
/// - `echo`: **ECHO** input GPIO pin
/// - `temp`: ambient **Temperature** measure calibration
/// - `sound_speed`: speed of sound given the ambient **Temperature**
/// - `timeout`: **ECHO** pin polling timeout, considering the maximum measuring range of 4m for
///     the sensor and the speed of sound given the ambient **Temperature**
///
/// # Example
/// ```
/// let triger = PinDriver::output(peripherals.pins.gpio4)?;
/// let mut echo = PinDriver::input(peripherals.pins.gpio2)?;
/// echo.set_pull(esp_idf_hal::gpio::Pull::Down)?;
/// let mut ultra = hc_sr04::HcSr04::new(triger, echo, None).expect("cant create sensor");
/// let ultra = async {
///     let delay_service = trigger::timer::get_timer().unwrap();
///     let mut timer = delay_service.timer().unwrap();
///
///     loop {
///         if let Some(distance) = ultra.measure_distance(hc_sr04::Unit::Centimeters)? {
///             info!("Distance: {}cm", distance);
///         } else {
///             warn!("Distance error");
///         }
///         timer.after(Duration::from_secs(1)).await.ok();
///     }
///
///     Ok::<(), hc_sr04::MeasurementError>(())
/// };
///
/// block_on(ultra).unwrap();
/// ```
pub struct HcSr04<'a, OPin: Pin, IPin: Pin> {
    trig: PinDriver<'a, OPin, Output>,
    echo: PinDriver<'a, IPin, Input>,
    sound_speed: f32,
    timeout: Duration,
}

#[derive(Debug)]
pub enum MeasurementError {
    EchoError,
    TrigError,
    NoEcho,
    MissedEcho,
}

impl<'a, OPin: Pin, IPin: Pin> HcSr04<'a, OPin, IPin> {
    /// Perform `sound_speed` and `timeout` calculations required to calibrate the sensor,
    /// based on **ambient temperature**.
    fn calibration_calc(temp: f32) -> (f32, Duration) {
        /// Speed of sound at 0C in m/s.
        const SOUND_SPEED_0C: f32 = 331.3;
        /// Increase speed of sound over temperature factor m/[sC].
        const SOUND_SPEED_INC_OVER_TEMP: f32 = 0.606;
        /// Maximum measuring range for HC-SR04 sensor in m.
        const MAX_RANGE: f32 = 4.0;

        // Speed of sound, depending on ambient temperature (if `temp` is `None`, default to 20C).
        let sound_speed = SOUND_SPEED_0C + (SOUND_SPEED_INC_OVER_TEMP * temp);

        // Polling timeout for **ECHO** pin: since max range for HC-SR04 is 4m, it doesn't make
        // sense to wait longer than the time required to the ultrasonic sound wave to cover the
        // max range distance. In other words, if the timeout is reached, the measurement was not
        // successfull or the object is located too far away from the sensor in order to be
        // detected.
        let timeout = Duration::from_secs_f32(MAX_RANGE / sound_speed * 2.);

        (sound_speed, timeout)
    }

    /// Initialize HC-SR04 sensor and register GPIO interrupt on `echo` pin for RisingEdge events
    /// in order to poll it for bouncing UltraSonic waves detection.
    ///
    /// # Parameters
    ///
    /// - `trig`: **TRIGGER** output GPIO pin
    /// - `echo`: **ECHO** input GPIO pin
    /// - `temp`: ambient **TEMPERATURE** used for calibration (if `None` defaults to `20.0`)
    pub fn new(
        mut trig: PinDriver<'a, OPin, Output>,
        echo: PinDriver<'a, IPin, Input>,
        temp: Option<f32>,
    ) -> Result<Self, MeasurementError> {
        trig.set_low().or(Err(MeasurementError::TrigError))?;
        let (sound_speed, timeout) = Self::calibration_calc(temp.unwrap_or(20f32));
        Ok(Self {
            trig,
            echo,
            sound_speed,
            timeout,
        })
    }

    /// Calibrate the sensor with the given **ambient temperature** (`temp`) expressed as *Celsius
    /// degrees*.
    pub fn calibrate(&mut self, temp: f32) {
        (self.sound_speed, self.timeout) = Self::calibration_calc(temp);
    }

    /// Perform **distance measurement**.
    ///
    /// Returns `Ok` variant if measurement succedes. Inner `Option` value is `None` if no object
    /// is present within maximum measuring range (*4m*); otherwhise, on `Some` variant instead,
    /// contained value represents distance expressed as the specified `unit`
    /// (**unit of measure**).
    pub fn measure_distance(&mut self, unit: Unit) -> Result<Option<f32>, MeasurementError> {
        info!("Measuring distance ...");
        self.echo.enable_interrupt().ok();
        let timeout = Instant::now();

        self.trig.set_high().ok();
        thread::sleep(Duration::from_micros(30));
        self.trig.set_low().ok();

        while self.echo.is_low() {
            if timeout.elapsed().as_millis() > 10 {
                return Err(MeasurementError::NoEcho);
            }
        }
        let instant = Instant::now();

        while self.echo.is_high() {
            if instant.elapsed().as_millis() > 1 {
                return Err(MeasurementError::MissedEcho);
            }
        }
        info!("calc distance ...");
        // Distance in cm.
        let distance = (self.sound_speed * instant.elapsed().as_secs_f32()) / 2.;

        Ok(Some(match unit {
            Unit::Millimeters => distance * 1000.,
            Unit::Centimeters => distance * 100.,
            Unit::Decimeters => distance * 10.,
            Unit::Meters => distance,
        }))
    }
}
