// `error_chain!` can recurse deeply
#![recursion_limit = "1024"]

#[macro_use]
extern crate error_chain;

use std::ffi::CStr;

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
pub struct TemperatureScaleResponse (TemperatureScale);

// Takes in a slice of bytes, and validates that they are nul-terminated and valid UTF-8/ASCII
fn str_from_response (response: &[u8]) -> Result <&str> {
    let terminated = CStr::from_bytes_with_nul (response).chain_err (|| ErrorKind::MalformedResponse)?;
    let r = terminated.to_str ().chain_err (|| ErrorKind::MalformedResponse)?;

    Ok (r)
}

/// Parses the result of the "S,?" command to query temperature scale.
/// Assumes that the passed response is the device's response without
/// the initial status byte.
pub fn parse_temperature_scale_response (response: &[u8]) -> Result<TemperatureScaleResponse> {
    let r = str_from_response (response)?;
    
    match r {
        "?S,c" => Ok (TemperatureScaleResponse (TemperatureScale::Celsius)),
        "?S,k" => Ok (TemperatureScaleResponse (TemperatureScale::Kelvin)),
        "?S,f" => Ok (TemperatureScaleResponse (TemperatureScale::Fahrenheit)),
        _ => Err (ErrorKind::ResponseParse.into ())
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_temperature_scale_response () {
        let response = "?S,c\0".as_bytes ();
        assert_eq! (parse_temperature_scale_response (&response).unwrap (),
                    TemperatureScaleResponse (TemperatureScale::Celsius));

        let response = "?S,k\0".as_bytes ();
        assert_eq! (parse_temperature_scale_response (&response).unwrap (),
                    TemperatureScaleResponse (TemperatureScale::Kelvin));

        let response = "?S,f\0".as_bytes ();
        assert_eq! (parse_temperature_scale_response (&response).unwrap (),
                    TemperatureScaleResponse (TemperatureScale::Fahrenheit));
    }

    #[test]
    fn parsing_invalid_temperature_scale_response_yields_error () {
        let response = "".as_bytes ();
        assert! (parse_temperature_scale_response (&response).is_err ());

        let response = "\0".as_bytes ();
        assert! (parse_temperature_scale_response (&response).is_err ());

        let response = "\x01".as_bytes ();
        assert! (parse_temperature_scale_response (&response).is_err ());

        let response = "?S,\0".as_bytes ();
        assert! (parse_temperature_scale_response (&response).is_err ());
    }
}
