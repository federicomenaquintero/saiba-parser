#![feature(str_checked_slicing)]

// `error_chain!` can recurse deeply
#![recursion_limit = "1024"]
#[macro_use]
extern crate error_chain;

use std::ffi::CStr;
use std::str::FromStr;

mod errors;

use errors::*;

/// Temperature scales supported by the EZO RTD sensor
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum TemperatureScale {
    Celsius,
    Kelvin,
    Fahrenheit
}

/// Response from the "S,?" command to query temperature scale
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct TemperatureScaleResponse (pub TemperatureScale);

impl TemperatureScaleResponse {
    /// Parses the result of the "S,?" command to query temperature scale.
    /// Assumes that the passed response is the device's response without
    /// the initial status byte.
    pub fn parse (response: &[u8]) -> Result<TemperatureScaleResponse> {
        let r = str_from_response (response)?;

        match r {
            "?S,c" => Ok (TemperatureScaleResponse (TemperatureScale::Celsius)),
            "?S,k" => Ok (TemperatureScaleResponse (TemperatureScale::Kelvin)),
            "?S,f" => Ok (TemperatureScaleResponse (TemperatureScale::Fahrenheit)),
            _ => Err (ErrorKind::ResponseParse.into ())
        }
    }
}

// Takes in a slice of bytes, and validates that they are nul-terminated and valid UTF-8/ASCII
fn str_from_response (response: &[u8]) -> Result <&str> {
    let terminated = CStr::from_bytes_with_nul (response).chain_err (|| ErrorKind::MalformedResponse)?;
    let r = terminated.to_str ().chain_err (|| ErrorKind::MalformedResponse)?;

    Ok (r)
}

/// Seconds between automatic logging of readings
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct DataLoggerStorageIntervalSeconds(pub u32);

/// Response from the "D,?" command to query the data logger's storage interval
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct DataLoggerStorageIntervalResponse (DataLoggerStorageIntervalSeconds);

impl DataLoggerStorageIntervalResponse {
    /// Parses the result of the "D,?" command to query the data logger's
    /// storage interval.  Returns the number of seconds between readings.
    pub fn parse (response: &[u8]) -> Result <DataLoggerStorageIntervalResponse> {
        let r = str_from_response (response)?;

        if r.starts_with ("?D,") {
            let num_str = r.get (3..).unwrap ();
            let num = u32::from_str (num_str).chain_err (|| ErrorKind::ResponseParse)?;
            Ok (DataLoggerStorageIntervalResponse(DataLoggerStorageIntervalSeconds (num)))
        } else {
            Err (ErrorKind::ResponseParse.into ())
        }
    }
}

/// A temperature value from a temperature reading
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Temperature {
    Celsius    (f64),
    Kelvin     (f64),
    Fahrenheit (f64)
}

impl Temperature {
    pub fn new (scale: TemperatureScale, value: f64) -> Temperature {
        match scale {
            TemperatureScale::Celsius    => Temperature::Celsius (value),
            TemperatureScale::Kelvin     => Temperature::Kelvin (value),
            TemperatureScale::Fahrenheit => Temperature::Fahrenheit (value)
        }
    }
}

/// Response from the "R" command to take a temperature reading
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct TemperatureResponse (pub Temperature);

impl TemperatureResponse {
    /// Parses the result of the "D" command to get a temperature reading.
    /// Note that this depends on knowing the temperature scale
    /// which the device is configured to use.
    pub fn parse (response: &[u8], scale: TemperatureScale) -> Result <TemperatureResponse> {
        let r = str_from_response (response)?;
        let val = f64::from_str (r).chain_err (|| ErrorKind::ResponseParse)?;
        Ok (TemperatureResponse (Temperature::new (scale, val)))
    }
}

/// Reason for which the device restarted, data sheet pp. 58
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum RestartReason {
    PoweredOff,
    SoftwareReset,
    BrownOut,
    Watchdog,
    Unknown
}

/// Response from the "Status" command to get the device status
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct DeviceStatusResponse {
    pub restart_reason: RestartReason,
    pub vcc_voltage: f64
}

impl DeviceStatusResponse {
    /// Parses the result of the "Status" command to get the device's status.
    pub fn parse (response: &[u8]) -> Result<DeviceStatusResponse> {
        let r = str_from_response (response)?;

        if r.starts_with ("?Status,") {
            let rest = r.get (8..).unwrap ();
            let mut split = rest.split (',');

            let restart_reason = match split.next () {
                Some ("P") => RestartReason::PoweredOff,
                Some ("S") => RestartReason::SoftwareReset,
                Some ("B") => RestartReason::BrownOut,
                Some ("W") => RestartReason::Watchdog,
                Some ("U") => RestartReason::Unknown,
                _          => return Err (ErrorKind::ResponseParse.into ())
            };

            let voltage = if let Some (voltage_str) = split.next () {
                f64::from_str (voltage_str).chain_err (|| ErrorKind::ResponseParse)?
            } else {
                return Err (ErrorKind::ResponseParse.into ());
            };

            if let Some (_) = split.next() {
                return Err (ErrorKind::ResponseParse.into ());
            }

            Ok (DeviceStatusResponse {
                restart_reason: restart_reason,
                vcc_voltage: voltage
            })
        } else {
            Err (ErrorKind::ResponseParse.into ())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_temperature_scale_response () {
        let response = "?S,c\0".as_bytes ();
        assert_eq! (TemperatureScaleResponse::parse (&response).unwrap (),
                    TemperatureScaleResponse (TemperatureScale::Celsius));

        let response = "?S,k\0".as_bytes ();
        assert_eq! (TemperatureScaleResponse::parse (&response).unwrap (),
                    TemperatureScaleResponse (TemperatureScale::Kelvin));

        let response = "?S,f\0".as_bytes ();
        assert_eq! (TemperatureScaleResponse::parse (&response).unwrap (),
                    TemperatureScaleResponse (TemperatureScale::Fahrenheit));
    }

    #[test]
    fn parsing_invalid_temperature_scale_response_yields_error () {
        let response = "".as_bytes ();
        assert! (TemperatureScaleResponse::parse (&response).is_err ());

        let response = "\0".as_bytes ();
        assert! (TemperatureScaleResponse::parse (&response).is_err ());

        let response = "\x01".as_bytes ();
        assert! (TemperatureScaleResponse::parse (&response).is_err ());

        let response = "?S,\0".as_bytes ();
        assert! (TemperatureScaleResponse::parse (&response).is_err ());
    }

    #[test]
    fn parses_data_logger_storage_interval_response () {
        let response = "?D,1\0".as_bytes ();
        assert_eq! (DataLoggerStorageIntervalResponse::parse (response).unwrap (),
                    DataLoggerStorageIntervalResponse (DataLoggerStorageIntervalSeconds (1)));

        let response = "?D,42\0".as_bytes ();
        assert_eq! (DataLoggerStorageIntervalResponse::parse (response).unwrap (),
                    DataLoggerStorageIntervalResponse (DataLoggerStorageIntervalSeconds (42)));
    }

    #[test]
    fn parsing_invalid_data_logger_storage_interval_response_yields_error () {
        let response = "?D,\0".as_bytes ();
        assert! (DataLoggerStorageIntervalResponse::parse (response).is_err ());

        let response = "?D,-1\0".as_bytes ();
        assert! (DataLoggerStorageIntervalResponse::parse (response).is_err ());

        let response = "?D,foo\0".as_bytes ();
        assert! (DataLoggerStorageIntervalResponse::parse (response).is_err ());
    }

    #[test]
    fn parses_temperature_response () {
        let response = "0\0".as_bytes ();
        assert_eq! (TemperatureResponse::parse (response, TemperatureScale::Celsius).unwrap (),
                    TemperatureResponse (Temperature::Celsius (0.0)));

        let response = "1234.5\0".as_bytes ();
        assert_eq! (TemperatureResponse::parse (response, TemperatureScale::Kelvin).unwrap (),
                    TemperatureResponse (Temperature::Kelvin (1234.5)));

        let response = "-10.5\0".as_bytes ();
        assert_eq! (TemperatureResponse::parse (response, TemperatureScale::Fahrenheit).unwrap (),
                    TemperatureResponse (Temperature::Fahrenheit (-10.5)));
    }

    #[test]
    fn parsing_invalid_temperature_response_yields_error () {
        let response = "\0".as_bytes ();
        assert! (TemperatureResponse::parse (response, TemperatureScale::Celsius).is_err ());

        let response = "-x\0".as_bytes ();
        assert! (TemperatureResponse::parse (response, TemperatureScale::Celsius).is_err ());
    }

    #[test]
    fn parses_device_status_response () {
        let response = "?Status,P,1.5\0".as_bytes ();
        assert_eq! (DeviceStatusResponse::parse (response).unwrap (),
                    DeviceStatusResponse {
                        restart_reason: RestartReason::PoweredOff,
                        vcc_voltage: 1.5
                    });

        let response = "?Status,S,1.5\0".as_bytes ();
        assert_eq! (DeviceStatusResponse::parse (response).unwrap (),
                    DeviceStatusResponse {
                        restart_reason: RestartReason::SoftwareReset,
                        vcc_voltage: 1.5
                    });

        let response = "?Status,B,1.5\0".as_bytes ();
        assert_eq! (DeviceStatusResponse::parse (response).unwrap (),
                    DeviceStatusResponse {
                        restart_reason: RestartReason::BrownOut,
                        vcc_voltage: 1.5
                    });

        let response = "?Status,W,1.5\0".as_bytes ();
        assert_eq! (DeviceStatusResponse::parse (response).unwrap (),
                    DeviceStatusResponse {
                        restart_reason: RestartReason::Watchdog,
                        vcc_voltage: 1.5
                    });

        let response = "?Status,U,1.5\0".as_bytes ();
        assert_eq! (DeviceStatusResponse::parse (response).unwrap (),
                    DeviceStatusResponse {
                        restart_reason: RestartReason::Unknown,
                        vcc_voltage: 1.5
                    });
    }

    #[test]
    fn parsing_invalid_device_status_response_yields_error () {
        let response = "\0".as_bytes ();
        assert! (DeviceStatusResponse::parse (response).is_err ());

        let response = "?Status,X,\0".as_bytes ();
        assert! (DeviceStatusResponse::parse (response).is_err ());

        let response = "?Status,P,1.5,\0".as_bytes ();
        assert! (DeviceStatusResponse::parse (response).is_err ());
    }
}
