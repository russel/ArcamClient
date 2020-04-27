/*
 *  arcamclient —  A gtk-rs based Rust application for controlling Arcam amplifiers.
 *
 *  Copyright © 2020  Russel Winder
 *
 *  This program is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  This program is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with this program. If not, see <http://www.gnu.org/licenses/>.
 */

/*!
The Arcam manual states that each Arcam amplifier that has an Ethernet connection
can be connected to using port 50000 (AVR850, 50001 for AVR600) as a Telnet connection.
The command PACKET_START (Remote Flow Control) is used to exchange request/response packets
(asynchronous question answer, not synchronous, response within 3 seconds of request.
*/

use num_derive::FromPrimitive;  // Apparently unused, but it is necessary.
use num_traits::FromPrimitive;

/// Zone numbers 1 and 2 for AVR850 but 1, 2, and 3 for AVR600.
#[derive(Clone, Copy, Debug, Eq, FromPrimitive, Hash, PartialEq)]
pub enum ZoneNumber {
    One = 1,
    Two = 2,
}

/// The commands (Cc entries) that can be sent to the AVR using the message protocol.
#[derive(Clone, Copy, Debug, Eq, FromPrimitive, PartialEq)]
pub enum Command {
    // =================== System Commands
    Power = 0x00,
    DisplayBrightness = 0x01,
    Headphones = 0x02,
    FMGenre = 0x03,
    SoftwareVersion = 0x04,
    RestoreFactoryDefaultSettings = 0x05,
    SaveRestoreSecureCopyOfSettings = 0x06,
    SimulateRC5IRCommand = 0x08,
    DisplayInformationType = 0x09,
    RequestCurrentSource = 0x1D,
    HeadphoneOverride = 0x1F,
    // =================== Input Commands
    VideoSelection = 0x0A,
    SelectAnalogueDigital = 0x0B,
    SetRequestVideoInputType = 0x0C,
    // =================== Output Commands
    SetRequestVolume = 0x0D,
    RequestMuteStatus = 0x0E,
    RequestDirectModeStatus = 0x0F,
    RequestDecodeModeStatus2ch = 0x10,
    RequestDecodeModeStatusMCH = 0x11,
    RequestRDSInformation = 0x12,
    SetRequestVideoOutputResolution = 0x13,
    // =================== Menu Commands
    RequestMenuStatus = 0x14,
    RequestTunerPreset = 0x15,
    Tune = 0x16,
    RequestDABSiriusStation = 0x18,
    RadioProgrammeTypeCategory = 0x19,
    RequestRDSDLSInformation = 0x1A,
    RequestPresetDetails = 0x1B,
    // =================== Network Commands
    NetworkPlaybackStatus = 0x1C,
    // =================== Setup Adjustment Commands
    TrebleEqualisation = 0x35,
    BassEqualisation = 0x36,
    RoomEqualisation = 0x37,
    DolbyVolume = 0x38,
    DolbyLeveller = 0x39,
    DolbyVolumeCalibrationOffset = 0x3A,
    Balance = 0x3B,
    DolbyProLogicIIDimension = 0x3C,
    DolbyProLogicIICentreWidth = 0x3D,
    DolbyProLogicIIPanorama = 0x3E,
    SubwooferTrim = 0x3F,
    LipsyncDelay = 0x40,
    Compression = 0x41,
    RequestIncomingVideoParameters = 0x42,
    RequestIncomingAudioFormat = 0x43,
    RequestIncomingAudioSampleRate = 0x44,
    SetRequestSubStereoTrim = 0x45,
    SetRequestBrightness = 0x46,
    SetRequestContrast = 0x47,
    SetRequestColour = 0x48,
    SetRequestPictureMode = 0x49,
    SetRequestEdgeEnhancement = 0x4A,
    SetRequestMosquitoNR = 0x4B,
    SetRequestNoiseReduction = 0x4C,
    SetRequestBlockNoiseReduction = 0x4D,
    SetRequestZone1OSDOnOff = 0x4E,
    SetRequestVideoOutputSwitching = 0x4F,
    SetRequestOutputFrameRate = 0x50,
}

/// The answer codes (Ac entries) that can be received from the AVR.
#[derive(Clone, Copy, Debug, Eq, FromPrimitive, PartialEq)]
pub enum AnswerCode {
    StatusUpdate = 0x00,
    ZoneInvalid = 0x82,
    CommandNotRecognized = 0x83,
    ParameterNotRecognized = 0x84,
    CommandInvalidAtThisTime = 0x85,
    InvalidDataLength = 0x86,
}

pub static PACKET_START: u8 = 0x21;
pub static PACKET_END: u8 = 0x0d;

/**
Construct a byte sequence representing a valid request to the AVR.

All requests are structured:

- St (Start transmission): PACKET_START ‘!’
- Zn (Zone number): 0x1, 0x2 for the zone number
- Cc (Command code): the code for the command
- Dl (Data Length): the number of data items following this item, excluding the ETR
- Data: the parameters for the response of length n. n is limited to 255
- Et (End transmission): PACKET_END
*/
pub fn create_request(zone: ZoneNumber, cc: Command, args: &[u8]) -> Result<Vec<u8>, &'static str> {
    let dl = args.len();
    if dl >= 256 { return Err("args array length not right."); }
    let mut result = vec![PACKET_START, zone as u8, cc as u8, dl as u8];
    result.extend(args);
    result.push(PACKET_END);
    Ok(result)
}

/**
Construct a byte sequence representing a valid response from the AVR.

All responses are structured:

- St (Start transmission): PACKET_START ‘!’
- Zn (Zone number): 0x1, 0x2 for the zone number
- Cc (Command code): the code for the command
- Ac (Answer code): the answer code for the request
- Dl (Data Length): the number of data items following this item, excluding the ETR
- Data: the parameters for the response of length n. n is limited to 255
- Et (End transmission): PACKET_END
*/
pub fn create_response(zone: ZoneNumber, cc: Command, ac: AnswerCode, args: &[u8]) -> Result<Vec<u8>, &'static str> {
    let dl = args.len();
    if dl >= 256 { return Err("args array length not right."); }
    let mut result = vec![PACKET_START, zone as u8, cc as u8, ac as u8, dl as u8];
    result.extend(args);
    result.push(PACKET_END);
    Ok(result)
}

/**
 Parse the bytes to create a tuple representing an Arcam request.

All requests are structured:

- St (Start transmission): PACKET_START ‘!’
- Zn (Zone number): 0x1, 0x2 for the zone number
- Cc (Command code): the code for the command
- Dl (Data Length): the number of data items following this item, excluding the ETR
- Data: the parameters for the response of length n. n is limited to 255
- Et (End transmission): PACKET_END
*/
pub fn parse_request(packet: &[u8]) -> Result<(ZoneNumber, Command, Vec<u8>), &'static str> {
    if packet[0] != PACKET_START { return Err("First byte is not the start of packet marker."); }
    if packet[packet.len() - 1] != PACKET_END { return Err("Final byte is not the end of packet marker."); }
    let length = packet.len();
    let packet = &packet[1 .. (length - 1)];
    assert_eq!(packet.len(), length - 2);
    if packet.len() < 4 { return Err("Insufficient bytes to form a valid packet."); }
    let zone= FromPrimitive::from_u8(packet[0]).unwrap();
    let cc= FromPrimitive::from_u8(packet[1]).unwrap();
    let dl = packet[2] as usize;
    if packet.len() != dl + 3 { return Err("Data length byte and actual length of the data do not agree."); }
    let data = packet[3..].to_vec();
    assert_eq!(data.len(), dl as usize);
    Ok((zone, cc, data))
}

/**
Parse the bytes to create a tuple representing an Arcam response.

All responses are structured;

- St (Start transmission): PACKET_START ‘!’
- Zn (Zone number): 0x1, 0x2 for the zone number
- Cc (Command code): the code for the command
- Ac (Answer code): the answer code for the request
- Dl (Data Length): the number of data items following this item, excluding the ETR
- Data: the parameters for the response of length n. n is limited to 255
- Et (End transmission): PACKET_END
*/
pub fn parse_response(packet: &[u8]) -> Result<(ZoneNumber, Command, AnswerCode, Vec<u8>), &'static str> {
    if packet[0] != PACKET_START { return Err("First byte is not the start of packet marker."); }
    if packet[packet.len() - 1] != PACKET_END { return Err("Final byte is not the end of packet marker."); }
    let length = packet.len();
    let packet = &packet[1 .. (length - 1)];
    assert_eq!(packet.len(), length - 2);
    if packet.len() < 5 { return Err("Insufficient bytes to form a valid packet."); }
    let zone = FromPrimitive::from_u8(packet[0]).unwrap();
    let cc= FromPrimitive::from_u8(packet[1]).unwrap();
    let ac= FromPrimitive::from_u8(packet[2]).unwrap();
    let dl = packet[3] as usize;
    if packet.len() != dl + 4 { return Err("Data length byte and actual length of the data do not agree."); }
    let data = packet[4..].to_vec();
    assert_eq!(data.len(), dl as usize);
    Ok((zone, cc, ac, data))
}

#[cfg(test)]
mod tests {
    use super::*;

    use num_derive::FromPrimitive;
    use num_traits::FromPrimitive;

    #[test]
    fn create_display_brightness_request() {
        match create_request(ZoneNumber::One, Command::DisplayBrightness, &mut [0xf0]) {
            Ok(value) => assert_eq!(value, [PACKET_START, 0x01, 0x01, 0x01, 0xf0, PACKET_END]),
            Err(_) => assert!(false),
        }
    }

    #[test]
    fn create_get_volume_request() {
        match create_request(ZoneNumber::One, Command::SetRequestVolume, &mut [0xf0]) {
            Ok(value) => assert_eq!(value, [PACKET_START, 0x01, PACKET_END, 0x01, 0xf0, PACKET_END]),
            Err(_) => assert!(false),
        }
    }

    #[test]
    fn create_set_volume_request() {
        match create_request(ZoneNumber::One, Command::SetRequestVolume, &mut [20]) {
            Ok(value) => assert_eq!(value, [PACKET_START, 0x01, PACKET_END, 0x01, 0x14, PACKET_END]),
            Err(_) => assert!(false),
        }
    }

    #[test]
    fn parse_valid_set_volume_request() {
        let mut request = create_request(ZoneNumber::One, Command::SetRequestVolume, &mut [20]).unwrap();
        assert_eq!(parse_request(&mut request).unwrap(), (ZoneNumber::One, Command::SetRequestVolume, vec![0x14]));
    }

    #[test]
    fn cannot_create_zone_zero() {
        let zone: Option<ZoneNumber> = FromPrimitive::from_u8(0);
        assert!(zone.is_none())
    }

    #[test]
    fn cannot_create_zone_four() {
        let zone: Option<ZoneNumber> = FromPrimitive::from_u8(4);
        assert!(zone.is_none())
    }

    #[test]
    fn cannot_create_unknown_command() {
        // We know that 0x20 and 0x61 are not known commands.
        let cc: Option<Command> = FromPrimitive::from_i8(0x20);
        assert!(cc.is_none());
        let cc: Option<Command> = FromPrimitive::from_i8(0x61);
        assert!(cc.is_none());
    }

    #[test]
    fn data_length_must_be_less_than_256() {
        assert!(create_request(ZoneNumber::One, Command::Power, &[0u8; 300]) .is_err());
    }

    #[test]
    fn create_display_brightness_response() {
        match create_response(ZoneNumber::One, Command::DisplayBrightness, AnswerCode::StatusUpdate, &[0x01]) {
            Ok(value) => assert_eq!(value, [PACKET_START, 0x01, 0x01, 0x00, 0x01, 0x01, PACKET_END]),
            Err(_) => assert!(false),
        }
    }

    #[test]
    fn parse_valid_display_brightness_response() {
        match parse_response(
            &create_response(ZoneNumber::One, Command::DisplayBrightness, AnswerCode::StatusUpdate, &[0x01]).unwrap()) {
            Ok(value) => assert_eq!(value, (ZoneNumber::One, Command::DisplayBrightness, AnswerCode::StatusUpdate, vec![0x01])),
            Err(_) => assert!(false),
        }
    }
}
