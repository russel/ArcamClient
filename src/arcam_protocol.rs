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

//! This module provides various enums and structs to do with implementing the Arcam protocol
//! for communicating with an Arcam amplifier over a TCP connection.
//!
//! The Arcam manual states that each Arcam amplifier that has an Ethernet connection can be
//! connected to using port 50000 (AVR850, 50001 for AVR600). The amplifier responds to messages
//! "AMX" by sending an AMXB response. Otherwise communication is via packets with well defined
//! structures.  Each packet starts with 0x21 (!) and ends with 0x0d (\r).
//!
//! Request packets sent to the amplifier are structured:
//!
//! - St (Start transmission): 0x21 (‘!’)
//! - Zn (Zone number): 0x1, 0x2 for the zone number
//! - Cc (Command code): the code for the command
//! - Dl (Data Length): the number of data items following this item, excluding the Et
//! - Data: the parameters for the response of length n. n is limited to 255
//! - Et (End transmission): 0x0d (\r)
//!
//! Response packet sent by the amplifier are structured:
//!
//! - St (Start transmission): 0x21 (‘!’)
//! - Zn (Zone number): 0x1, 0x2 for the zone number
//! - Cc (Command code): the code for the command
//! - Ac (Answer code): the answer code for the request
//! - Dl (Data Length): the number of data items following this item, excluding the Et
//! - Data: the parameters for the response of length n. n is limited to 255
//! - Et (End transmission): 0x0d (\r)
//!
//! Communication is not synchronous: the amplifier receives request packets and within 3
//! seconds will send a response packet. There is no guarantee that the response order will be
//! the request order, but it normal circumstances it probably will be.

use std::collections::HashMap;
use std::fmt;

use lazy_static::lazy_static;

use num_derive::FromPrimitive;  // Apparently unused, but it is necessary.
use num_traits::FromPrimitive;

/// Zone numbers 1 and 2 for AVR850 but 1, 2, and 3 for AVR600.
#[derive(Clone, Copy, Debug, Eq, FromPrimitive, Hash, PartialEq)]
#[repr(u8)]
pub enum ZoneNumber {
    One = 1,
    Two = 2,
}

/// The commands (Cc entries) that can be sent to the amplifier using the message protocol.
#[derive(Clone, Copy, Debug, Eq, FromPrimitive, PartialEq)]
#[repr(u8)]
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
    // SetRequestVideoInputType = 0x0C,  // Not in AVR850, was in AVR600.
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
    RequestDABStation = 0x18, // Was called RequestDABSiriusStation for AVR600
    ProgrammeTypeCategory = 0x19, // Was called RadioProgrammeTypeCategory for AVR600
    DLSPDTInformation = 0x1A, // Was called RequestRDSDLSInformation dor AVR600
    RequestPresetDetails = 0x1B,
    NetworkPlaybackStatus = 0x1C,
    IMAXEnhanced = 0x0C, // In AVR850, not in AVR600
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
    // SetRequestBrightness = 0x46,  // Not in AVR850, was in AVR600
    // SetRequestContrast = 0x47,  // Not in AVR850, was in AVR600
    // SetRequestColour = 0x48,  // Not in AVR850, was in AVR600
    // SetRequestPictureMode = 0x49,  // Not in AVR850, was in AVR600
    // SetRequestEdgeEnhancement = 0x4A,  // Not in AVR850, was in AVR600
    // SetRequestMosquitoNR = 0x4B,  // Not in AVR850, was in AVR600
    // SetRequestNoiseReduction = 0x4C,  // Not in AVR850, was in AVR600
    // SetRequestBlockNoiseReduction = 0x4D,  // Not in AVR850, was in AVR600
    SetRequestZone1OSDOnOff = 0x4E,
    SetRequestVideoOutputSwitching = 0x4F,
    // SetRequestOutputFrameRate = 0x50,  // Not in AVR850, was in AVR600
    SetRequestInputName = 0x20, // In AVR850, not in AVR600
    FMScanUpDown = 0x23, // In AVR850, not in AVR600
    DABScan = 0x24, // In AVR850, not in AVR600
    Heartbeat = 0x25, // In AVR850, not in AVR600
    Reboot = 0x26,  // In AVR850, not in AVR600
}

/// The RC5 commands used via the `SimulateRC5IRCommand` [Command](enum.Command.html).
///
/// The values of these variants are pairs of `u8` values. Python and D can handle enum variants
/// being tuples, but it seems that Rust cannot. Thus, define the variants and the values
/// separately. :-(
///
///The order of the variants is as it is written in the table of the documentation.  It is
/// neither numeric order nor function order.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RC5Command {
    Standby,
    One,
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    AccessLipsyncDelayControl,
    Zero,
    CycleBetweenVFDInformationPanels,
    Rewind,
    FastForward,
    SkipBack,
    SkipForward,
    Stop,
    Play,
    Pause,
    Disc_Record_EnterTrimMenu,
    MENU_EnterSystemMenu,
    NavigateUp,
    PopUp_DolbyVolumeOnOff,
    NavigateLeft,
    OK,
    NavigateRight,
    Audio_RoomEQOnOff,
    NavigateDown,
    RTN_AccessSubwooferTrimControl,
    HOME,
    Mute,
    IncreaseVolume,
    MODE_CycleBetweenDecodingModes,
    DISP_ChangeVFDBrightness,
    ActivateDIRECTMode,
    DecreaseVolume,
    Red,
    Green,
    Yellow,
    Blue,
    Radio,
    Aux,
    Net,
    USB,
    AV,
    Sat,
    PVR,
    Game,
    BD,
    CD,
    STB,
    VCR,
    Display,
    PowerOn,
    PowerOff,
    ChangeControlToNextZone,
    CycleBetweenOutputResolutions,
    AccessBassControl,
    AccessSpeakerTrimControls,
    AccessTrebleControl,
    Random,
    Repeat,
    DirectModeOn,
    DirectModeOff,
    MultiChannel,
    Stereo,
    DolbySurround,
    DTSNeo6Cinema,
    DTSNeo6Music,
    DTSNeuralX,
    Reserved,
    DTSVirtualX,
    FiveSevenChannelStereo,
    DolbyDEX,
    MuteOn,
    MuteOff,
    FM,
    DAB,
    LipSyncPlus5ms,
    LipSyncMinus5ms,
    SubTrimPlusHalfDb,
    SubTrimMinusHalfDb,
    DisplayOff,
    DisplayL1,
    DisplayL2,
    BalanceLeft,
    BalanceRight,
    BassPlus1,
    BassMinus1,
    TreblePlus1,
    TrebleMinus1,
    SetZone2ToFollowZone1,
    Zone2PowerOn,
    Zone2PowerOff,
    Zone2VolumePlus,
    Zone2VolumeMinus,
    Zone2Mute,
    Zone2MuteOn,
    Zone2MuteOff,
    Zone2CD,
    Zone2BD,
    Zone2STB,
    Zone2AV,
    Zone2Game,
    Zone2Aux,
    Zone2PVR,
    Zone2FM,
    Zone2DAB,
    Zone2USB,
    Zone2NET,
    Zone2Sat,
    Zone2VCR,
    SelectHDMIOut1,
    SelectHDMIOut2,
    SelectHDMIOut1And2,
}

lazy_static! {
    // The data for the RC5 commands. Lazy statics cannot be exported it seems
    // so use an accessor function to get the values.
    static ref RC5DATA: HashMap<RC5Command, (u8, u8)> = {
        let mut d = HashMap::new();
        d.insert(RC5Command::Standby, (0x10, 0x0c));
        d.insert(RC5Command::One, (0x10, 0x01));
        d.insert(RC5Command::Two, (0x10, 0x02));
        d.insert(RC5Command::Three, (0x10, 0x03));
        d.insert(RC5Command::Four, (0x10, 0x04));
        d.insert(RC5Command::Five, (0x10, 0x05));
        d.insert(RC5Command::Six, (0x10, 0x06));
        d.insert(RC5Command::Seven, (0x10, 0x07));
        d.insert(RC5Command::Eight, (0x10, 0x08));
        d.insert(RC5Command::Nine, (0x10, 0x09));
        d.insert(RC5Command::AccessLipsyncDelayControl, (0x10, 0x32));
        d.insert(RC5Command::Zero, (0x10, 0x00));
        d.insert(RC5Command::CycleBetweenVFDInformationPanels, (0x10, 0x37));
        d.insert(RC5Command::Rewind, (0x10, 0x79));
        d.insert(RC5Command::FastForward, (0x10, 0x34));
        d.insert(RC5Command::SkipBack, (0x10, 0x21));
        d.insert(RC5Command::SkipForward, (0x10, 0x0b));
        d.insert(RC5Command::Stop, (0x10, 0x36));
        d.insert(RC5Command::Play, (0x10, 0x35));
        d.insert(RC5Command::Pause, (0x10, 0x30));
        d.insert(RC5Command::Disc_Record_EnterTrimMenu, (0x10, 0x5a));
        d.insert(RC5Command::MENU_EnterSystemMenu, (0x10, 0x52));
        d.insert(RC5Command::NavigateUp, (0x10, 0x56));
        d.insert(RC5Command::PopUp_DolbyVolumeOnOff, (0x10, 0x46));
        d.insert(RC5Command::NavigateLeft, (0x10, 0x51));
        d.insert(RC5Command::OK, (0x10, 0x57));
        d.insert(RC5Command::NavigateRight, (0x10, 0x50));
        d.insert(RC5Command::Audio_RoomEQOnOff, (0x10, 0x1e));
        d.insert(RC5Command::NavigateDown, (0x10, 0x55));
        d.insert(RC5Command::RTN_AccessSubwooferTrimControl, (0x10, 0x33));
        d.insert(RC5Command::HOME, (0x10, 0x2b));
        d.insert(RC5Command::Mute, (0x10, 0x0d));
        d.insert(RC5Command::IncreaseVolume, (0x10, 0x10));
        d.insert(RC5Command::MODE_CycleBetweenDecodingModes, (0x10, 0x20));
        d.insert(RC5Command::DISP_ChangeVFDBrightness, (0x10, 0x3b));
        d.insert(RC5Command::ActivateDIRECTMode, (0x10, 0x0a));
        d.insert(RC5Command::DecreaseVolume, (0x10, 0x11));
        d.insert(RC5Command::Red, (0x10, 0x29));
        d.insert(RC5Command::Green, (0x10, 0x2a));
        d.insert(RC5Command::Yellow, (0x10, 0x2b)); // Repeat use of value according to the document.
        d.insert(RC5Command::Blue, (0x10, 0x37)); // Repeat use of value according to the document.
        d.insert(RC5Command::Radio, (0x10, 0x5b));
        d.insert(RC5Command::Aux, (0x10, 0x63));
        d.insert(RC5Command::Net, (0x10, 0x5c));
        d.insert(RC5Command::USB, (0x10, 0x5d));
        d.insert(RC5Command::AV, (0x10, 0x5e));
        d.insert(RC5Command::Sat, (0x10, 0x1b));
        d.insert(RC5Command::PVR, (0x10, 0x60));
        d.insert(RC5Command::Game, (0x10, 0x61));
        d.insert(RC5Command::BD, (0x10, 0x62));
        d.insert(RC5Command::CD, (0x10, 0x76));
        d.insert(RC5Command::STB, (0x10, 0x64));
        d.insert(RC5Command::VCR, (0x10, 0x77));
        d.insert(RC5Command::Display, (0x10, 0x3a));
        d.insert(RC5Command::PowerOn, (0x10, 0x7b));
        d.insert(RC5Command::PowerOff, (0x10, 0x7c));
        d.insert(RC5Command::ChangeControlToNextZone, (0x10, 0x5f));
        d.insert(RC5Command::CycleBetweenOutputResolutions, (0x10, 0x2f));
        d.insert(RC5Command::AccessBassControl, (0x10, 0x27));
        d.insert(RC5Command::AccessSpeakerTrimControls, (0x10, 0x25));
        d.insert(RC5Command::AccessTrebleControl, (0x10, 0x0e));
        d.insert(RC5Command::Random, (0x10, 0x4c));
        d.insert(RC5Command::Repeat, (0x10, 0x31));
        d.insert(RC5Command::DirectModeOn, (0x10, 0x4e));
        d.insert(RC5Command::DirectModeOff, (0x10, 0x4f));
        d.insert(RC5Command::MultiChannel, (0x10, 0x6a));
        d.insert(RC5Command::Stereo, (0x10, 0x6b));
        d.insert(RC5Command::DolbySurround, (0x10, 0x6e));
        d.insert(RC5Command::DTSNeo6Cinema, (0x10, 0x6f));
        d.insert(RC5Command::DTSNeo6Music, (0x10, 0x70));
        d.insert(RC5Command::DTSNeuralX, (0x10, 0x71));
        d.insert(RC5Command::Reserved, (0x10, 0x72));
        d.insert(RC5Command::DTSVirtualX, (0x10, 0x73));
        d.insert(RC5Command::FiveSevenChannelStereo, (0x10, 0x45));
        d.insert(RC5Command::DolbyDEX, (0x10, 0x17));
        d.insert(RC5Command::MuteOn, (0x10, 0x1a));
        d.insert(RC5Command::MuteOff, (0x10, 0x78));
        d.insert(RC5Command::FM, (0x10, 0x1c));
        d.insert(RC5Command::DAB, (0x10, 0x48));
        d.insert(RC5Command::LipSyncPlus5ms, (0x10, 0x0f));
        d.insert(RC5Command::LipSyncMinus5ms, (0x10, 0x65));
        d.insert(RC5Command::SubTrimPlusHalfDb, (0x10, 0x69));
        d.insert(RC5Command::SubTrimMinusHalfDb, (0x10, 0x6c));
        d.insert(RC5Command::DisplayOff, (0x10, 0x1f));
        d.insert(RC5Command::DisplayL1, (0x10, 0x22));
        d.insert(RC5Command::DisplayL2, (0x10, 0x23));
        d.insert(RC5Command::BalanceLeft, (0x10, 0x26));
        d.insert(RC5Command::BalanceRight, (0x10, 0x28));
        d.insert(RC5Command::BassPlus1, (0x10, 0x2c));
        d.insert(RC5Command::BassMinus1, (0x10, 0x2d));
        d.insert(RC5Command::TreblePlus1, (0x10, 0x2e));
        d.insert(RC5Command::TrebleMinus1, (0x10, 0x66));
        d.insert(RC5Command::SetZone2ToFollowZone1, (0x10, 0x14));
        d.insert(RC5Command::Zone2PowerOn, (0x17, 0x7b));
        d.insert(RC5Command::Zone2PowerOff, (0x17, 0x7c));
        d.insert(RC5Command::Zone2VolumePlus, (0x17, 0x01));
        d.insert(RC5Command::Zone2VolumeMinus, (0x17, 0x02));
        d.insert(RC5Command::Zone2Mute, (0x17, 0x03));
        d.insert(RC5Command::Zone2MuteOn, (0x17, 0x04));
        d.insert(RC5Command::Zone2MuteOff, (0x17, 0x05));
        d.insert(RC5Command::Zone2CD, (0x17, 0x06));
        d.insert(RC5Command::Zone2BD, (0x17, 0x07));
        d.insert(RC5Command::Zone2STB, (0x17, 0x08));
        d.insert(RC5Command::Zone2AV, (0x17, 0x09));
        d.insert(RC5Command::Zone2Game, (0x17, 0x0b));
        d.insert(RC5Command::Zone2Aux, (0x17, 0x0d));
        d.insert(RC5Command::Zone2PVR, (0x17, 0x0f));
        d.insert(RC5Command::Zone2FM, (0x17, 0x0e));
        d.insert(RC5Command::Zone2DAB, (0x17, 0x10));
        d.insert(RC5Command::Zone2USB, (0x17, 0x12));
        d.insert(RC5Command::Zone2NET, (0x17, 0x13));
        d.insert(RC5Command::Zone2Sat, (0x17, 0x14));
        d.insert(RC5Command::Zone2VCR, (0x17, 0x15));
        d.insert(RC5Command::SelectHDMIOut1, (0x10, 0x49));
        d.insert(RC5Command::SelectHDMIOut2, (0x10, 0x4a));
        d.insert(RC5Command::SelectHDMIOut1And2, (0x10, 0x4b));
        d
    };
}

impl From<(u8, u8)> for RC5Command {
    fn from(value: (u8, u8)) -> Self {
        match value {
            (0x10, 0x0c) => RC5Command::Standby,
            (0x10, 0x01) => RC5Command::One,
            (0x10, 0x02) => RC5Command::Two,
            (0x10, 0x03) => RC5Command::Three,
            (0x10, 0x04) => RC5Command::Four,
            (0x10, 0x05) => RC5Command::Five,
            (0x10, 0x06) => RC5Command::Six,
            (0x10, 0x07) => RC5Command::Seven,
            (0x10, 0x08) => RC5Command::Eight,
            (0x10, 0x09) => RC5Command::Nine,
            (0x10, 0x32) => RC5Command::AccessLipsyncDelayControl,
            (0x10, 0x00) => RC5Command::Zero,
            (0x10, 0x37) => RC5Command::CycleBetweenVFDInformationPanels,
            (0x10, 0x79) => RC5Command::Rewind,
            (0x10, 0x34) => RC5Command::FastForward,
            (0x10, 0x21) => RC5Command::SkipBack,
            (0x10, 0x0b) => RC5Command::SkipForward,
            (0x10, 0x36) => RC5Command::Stop,
            (0x10, 0x35) => RC5Command::Play,
            (0x10, 0x30) => RC5Command::Pause,
            (0x10, 0x5a) => RC5Command::Disc_Record_EnterTrimMenu,
            (0x10, 0x52) => RC5Command::MENU_EnterSystemMenu,
            (0x10, 0x56) => RC5Command::NavigateUp,
            (0x10, 0x46) => RC5Command::PopUp_DolbyVolumeOnOff,
            (0x10, 0x51) => RC5Command::NavigateLeft,
            (0x10, 0x57) => RC5Command::OK,
            (0x10, 0x50) => RC5Command::NavigateRight,
            (0x10, 0x1e) => RC5Command::Audio_RoomEQOnOff,
            (0x10, 0x55) => RC5Command::NavigateDown,
            (0x10, 0x33) => RC5Command::RTN_AccessSubwooferTrimControl,
            (0x10, 0x2b) => RC5Command::HOME,
            (0x10, 0x0d) => RC5Command::Mute,
            (0x10, 0x10) => RC5Command::IncreaseVolume,
            (0x10, 0x20) => RC5Command::MODE_CycleBetweenDecodingModes,
            (0x10, 0x3b) => RC5Command::DISP_ChangeVFDBrightness,
            (0x10, 0x0a) => RC5Command::ActivateDIRECTMode,
            (0x10, 0x11) => RC5Command::DecreaseVolume,
            (0x10, 0x29) => RC5Command::Red,
            (0x10, 0x2a) => RC5Command::Green,
            (0x10, 0x2b) => RC5Command::Yellow, // Repeat use of value according to the document.
            (0x10, 0x37) => RC5Command::Blue, // Repeat use of value according to the document.
            (0x10, 0x5b) => RC5Command::Radio,
            (0x10, 0x63) => RC5Command::Aux,
            (0x10, 0x5c) => RC5Command::Net,
            (0x10, 0x5d) => RC5Command::USB,
            (0x10, 0x5e) => RC5Command::AV,
            (0x10, 0x1b) => RC5Command::Sat,
            (0x10, 0x60) => RC5Command::PVR,
            (0x10, 0x61) => RC5Command::Game,
            (0x10, 0x62) => RC5Command::BD,
            (0x10, 0x76) => RC5Command::CD,
            (0x10, 0x64) => RC5Command::STB,
            (0x10, 0x77) => RC5Command::VCR,
            (0x10, 0x3a) => RC5Command::Display,
            (0x10, 0x7b) => RC5Command::PowerOn,
            (0x10, 0x7c) => RC5Command::PowerOff,
            (0x10, 0x5f) => RC5Command::ChangeControlToNextZone,
            (0x10, 0x2f) => RC5Command::CycleBetweenOutputResolutions,
            (0x10, 0x27) => RC5Command::AccessBassControl,
            (0x10, 0x25) => RC5Command::AccessSpeakerTrimControls,
            (0x10, 0x0e) => RC5Command::AccessTrebleControl,
            (0x10, 0x4c) => RC5Command::Random,
            (0x10, 0x31) => RC5Command::Repeat,
            (0x10, 0x4e) => RC5Command::DirectModeOn,
            (0x10, 0x4f) => RC5Command::DirectModeOff,
            (0x10, 0x6a) => RC5Command::MultiChannel,
            (0x10, 0x6b) => RC5Command::Stereo,
            (0x10, 0x6e) => RC5Command::DolbySurround,
            (0x10, 0x6f) => RC5Command::DTSNeo6Cinema,
            (0x10, 0x70) => RC5Command::DTSNeo6Music,
            (0x10, 0x71) => RC5Command::DTSNeuralX,
            (0x10, 0x72) => RC5Command::Reserved,
            (0x10, 0x73) => RC5Command::DTSVirtualX,
            (0x10, 0x45) => RC5Command::FiveSevenChannelStereo,
            (0x10, 0x17) => RC5Command::DolbyDEX,
            (0x10, 0x1a) => RC5Command::MuteOn,
            (0x10, 0x78) => RC5Command::MuteOff,
            (0x10, 0x1c) => RC5Command::FM,
            (0x10, 0x48) => RC5Command::DAB,
            (0x10, 0x0f) => RC5Command::LipSyncPlus5ms,
            (0x10, 0x65) => RC5Command::LipSyncMinus5ms,
            (0x10, 0x69) => RC5Command::SubTrimPlusHalfDb,
            (0x10, 0x6c) => RC5Command::SubTrimMinusHalfDb,
            (0x10, 0x1f) => RC5Command::DisplayOff,
            (0x10, 0x22) => RC5Command::DisplayL1,
            (0x10, 0x23) => RC5Command::DisplayL2,
            (0x10, 0x26) => RC5Command::BalanceLeft,
            (0x10, 0x28) => RC5Command::BalanceRight,
            (0x10, 0x2c) => RC5Command::BassPlus1,
            (0x10, 0x2d) => RC5Command::BassMinus1,
            (0x10, 0x2e) => RC5Command::TreblePlus1,
            (0x10, 0x66) => RC5Command::TrebleMinus1,
            (0x10, 0x14) => RC5Command::SetZone2ToFollowZone1,
            (0x17, 0x7b) => RC5Command::Zone2PowerOn,
            (0x17, 0x7c) => RC5Command::Zone2PowerOff,
            (0x17, 0x01) => RC5Command::Zone2VolumePlus,
            (0x17, 0x02) => RC5Command::Zone2VolumeMinus,
            (0x17, 0x03) => RC5Command::Zone2Mute,
            (0x17, 0x04) => RC5Command::Zone2MuteOn,
            (0x17, 0x05) => RC5Command::Zone2MuteOff,
            (0x17, 0x06) => RC5Command::Zone2CD,
            (0x17, 0x07) => RC5Command::Zone2BD,
            (0x17, 0x08) => RC5Command::Zone2STB,
            (0x17, 0x09) => RC5Command::Zone2AV,
            (0x17, 0x0b) => RC5Command::Zone2Game,
            (0x17, 0x0d) => RC5Command::Zone2Aux,
            (0x17, 0x0f) => RC5Command::Zone2PVR,
            (0x17, 0x0e) => RC5Command::Zone2FM,
            (0x17, 0x10) => RC5Command::Zone2DAB,
            (0x17, 0x12) => RC5Command::Zone2USB,
            (0x17, 0x13) => RC5Command::Zone2NET,
            (0x17, 0x14) => RC5Command::Zone2Sat,
            (0x17, 0x15) => RC5Command::Zone2VCR,
            (0x10, 0x49) => RC5Command::SelectHDMIOut1,
            (0x10, 0x4a) => RC5Command::SelectHDMIOut2,
            (0x10, 0x4b) => RC5Command::SelectHDMIOut1And2,
            (_, _) => panic!("Invalid RC5Command value."),
        }
    }
}

impl From<&Vec<u8>> for RC5Command {
    fn from(data: &Vec<u8>) -> Self {
        assert_eq!(data.len(), 2);
        RC5Command::from((data[0], data[1]))
    }
}

/// Accessor for the [RC5Command](enum.RC5Command.html) variant values.
// This is needed because lazy static values seemingly cannot be exported out of the
// module to another module in the crate, or another crate.
pub fn get_rc5command_data(rc5command: RC5Command) -> (u8, u8) {
    RC5DATA[&rc5command]
}

/// The answer codes (Ac entries) that can be received from the amplifier.
#[derive(Clone, Copy, Debug, Eq, FromPrimitive, PartialEq)]
#[repr(u8)]
pub enum AnswerCode {
    StatusUpdate = 0x00,
    ZoneInvalid = 0x82,
    CommandNotRecognized = 0x83,
    ParameterNotRecognized = 0x84,
    CommandInvalidAtThisTime = 0x85,
    InvalidDataLength = 0x86,
}

/// The three levels of brightness of the amplifier display.
#[derive(Clone, Copy, Debug, Eq, FromPrimitive, PartialEq)]
#[repr(u8)]
pub enum Brightness {
    Off = 0,
    Level1 = 1,
    Level2 = 2,
}

impl ToString for Brightness {
    fn to_string(&self) -> String {
        match self {
            Brightness::Off => "Off".to_string(),
            Brightness::Level1 => "Level1".to_string(),
            Brightness::Level2 => "Level2".to_string(),
        }
    }
}

impl From<&str> for Brightness {
    fn from(s: &str) -> Self {
        match s {
            "Off" => Brightness::Off,
            "Level1" => Brightness::Level1,
            "Level2" => Brightness::Level2,
            x => panic!("Illegal brightness value from display – {}", x),
        }
    }
}

/// The various sources the amplifier can use.
///
/// Numeric representation as per the `RequestCurrentSource` [Command](enum.Command.html) return value.
#[derive(Clone, Copy, Debug, Eq, FromPrimitive, PartialEq)]
#[repr(u8)]
pub enum Source {
    FollowZone1 = 0x00,
    CD = 0x01,
    BD = 0x02,
    AV = 0x03,
    SAT = 0x04,
    PVR = 0x05,
    VCR = 0x06,
    AUX = 0x08,
    DISPLAY = 0x09,
    TUNER = 0x0B,  // TUNER (FM) according to the documentation.
    TUNERDAB = 0x0C,  // (AVR450/750 only) according to the documentation.
    NET = 0x0E,
    USB = 0x0F,
    STB = 0x10,
    GAME = 0x11,
}

impl From<&str> for Source {
    fn from(s: &str) -> Self {
        match s {
            "FollowZone1" => Source::FollowZone1,
            "CD" => Source::CD,
            "BD" => Source::BD,
            "AV" => Source::AV,
            "SAT" => Source::SAT,
            "PVR" => Source::PVR,
            "VCR" => Source::VCR,
            "AUX" => Source::AUX,
            "DISPLAY" => Source::DISPLAY,
            "TUNER" => Source::TUNER,  // TUNER (FM) according to the documentation.
            "TUNERDAB" => Source::TUNERDAB,  // (AVR450/750 only) according to the documentation.
            "NET" => Source::NET,
            "USB" => Source::USB,
            "STB" => Source::STB,
            "GAME" => Source::GAME,
            x => panic!("Illegal source value {}.", x),
        }
    }
}

/// The video sources.
///
/// Numeric representation as per the `VideoSelection` [Command](enum.Command.html) return value.
#[derive(Copy, Clone, Debug, Eq, FromPrimitive, PartialEq)]
#[repr(u8)]
pub enum VideoSource {
    BD = 0x00,
    SAT = 0x01,
    AV = 0x02,
    PVR = 0x03,
    VCR = 0x04,
    Game = 0x05,
    STB = 0x06,
}

/// An analogue of bool to represent the power state of a zone.
///
/// Numeric representation as per `Power` [Command](enum.Command.html) return value.
/// The UI needs a string representation and this avoids spelling errors.
#[derive(Clone, Copy, Debug, Eq, FromPrimitive, PartialEq)]
#[repr(u8)]
pub enum PowerState {
    Standby = 0x00,
    On = 0x01,
}

impl ToString for PowerState {
    fn to_string(&self) -> String {
        match self {
            Self::Standby => "Standby".to_string(),
            Self::On => "On".to_string(),
        }
    }
}

impl From<&str> for PowerState {
    fn from(s: &str) -> Self {
        match s {
            "Standby" => Self::Standby,
            "On" => Self::On,
            x => panic!("Illegal PowerState value, {}", x),
        }
    }
}

impl From<bool> for PowerState {
    fn from(b: bool) -> Self {
        match b {
            false => Self::Standby,
            true => Self::On,
        }
    }
}

impl From<PowerState> for bool {
    fn from(p: PowerState) -> Self {
        match p {
            PowerState::Standby => false,
            PowerState::On => true,
        }
    }
}

/// An analogue of bool to represent the mute state of a zone.
///
/// Numeric representation as per the `RequestMuteState` [Command](enum.Command.html) return value.
/// The UI needs a string representation and this avoids spelling errors.
#[derive(Clone, Copy, Debug, Eq, FromPrimitive, PartialEq)]
#[repr(u8)]
pub enum MuteState {
    Muted = 0x00,
    NotMuted = 0x01,
}

impl ToString for MuteState {
    fn to_string(&self) -> String {
        match self {
            Self::NotMuted => "Not Muted".to_string(),
            Self::Muted => "Muted".to_string(),
        }
    }
}

impl From<&str> for MuteState {
    fn from(s: &str) -> Self {
        match s {
            "Muted" => Self::Muted,
            "Not Muted" => Self::NotMuted,
            x => panic!("Illegal MuteState value, {}", x),
        }
    }
}

impl From<bool> for MuteState {
    fn from(b: bool) -> Self {
        match b {
            true => Self::Muted,
            false => Self::NotMuted,
        }
    }
}

impl From<MuteState> for bool {
    fn from(m: MuteState) -> Self {
        match m {
            MuteState::Muted => true,
            MuteState::NotMuted => false,
        }
    }
}

/// The value used as the start of packet value.
pub static PACKET_START: u8 = 0x21;

/// The value used as the end of packet value.
pub static PACKET_END: u8 = 0x0d;

/// The values used to represent the question of which value is currently set.
pub static REQUEST_QUERY: u8 = 0xf0;

/// A request to the amplifier.
#[derive(Clone, Eq, PartialEq)]
pub struct Request {
    pub zone: ZoneNumber,
    pub cc: Command,
    pub data: Vec<u8>,
}

impl Request {
    /// Create a new request.
    ///
    /// The data value is restricted to being at most 255 bytes long.
    pub fn new(zone: ZoneNumber, cc: Command, data: Vec<u8>) -> Result<Self, &'static str> {
        if data.len() > 255 { Err("Cannot have more than 255 bytes as data.") }
        else { Ok(Self {zone, cc, data}) }
    }

    /// Return the byte sequence representing this request.
    ///
    /// All requests are structured:
    ///
    /// - St (Start transmission): PACKET_START
    /// - Zn (Zone number): 0x1, 0x2 for the zone number
    /// - Cc (Command code): the code for the command
    /// - Dl (Data Length): the number of data items following this item, excluding the Et
    /// - Data: the parameters for the response of length n. n is limited to 255
    /// - Et (End transmission): PACKET_END
    pub fn to_bytes(self: &Self) -> Vec<u8> {
        let dl = self.data.len();
        if dl >= 256 { panic!("args array length not right."); }
        let mut result = vec![PACKET_START, self.zone as u8, self.cc as u8, dl as u8];
        result.extend(self.data.iter());
        result.push(PACKET_END);
        result
    }

    /// Parse the bytes in the buffer to create a tuple representing a request and the
    /// number of bytes used for the request packet.
    ///
    /// All requests are structured:
    ///
    /// - St (Start transmission): PACKET_START
    /// - Zn (Zone number): 0x1, 0x2 for the zone number
    /// - Cc (Command code): the code for the command
    /// - Dl (Data Length): the number of data items following this item, excluding the Et
    /// - Data: the parameters for the response of length n. n is limited to 255
    /// - Et (End transmission): PACKET_END
    pub fn parse_bytes(buffer: &[u8]) -> Result<(Self, usize), &str> {
        let packet_length = buffer.len();
        if packet_length < 5 { return Err("Insufficient bytes to form a packet."); }
        let mut index = 0;
        if buffer[index] != PACKET_START { return Err("First byte is not the start of packet marker."); }
        index += 1;
        if index >= packet_length { return Err("Insufficient bytes to form a packet."); }
        let zone = FromPrimitive::from_u8(buffer[index]).unwrap();
        index += 1;
        if index >= packet_length { return Err("Insufficient bytes to form a packet."); }
        let cc = FromPrimitive::from_u8(buffer[index]).unwrap();
        index += 1;
        if index >= packet_length { return Err("Insufficient bytes to form a packet."); }
        let dl = buffer[index] as usize;
        index += 1;
        if index >= packet_length { return Err("Insufficient bytes to form a packet."); }
        let end_index = index + dl;
        if end_index >= packet_length { return Err("Insufficient bytes to form a packet."); }
        let data = buffer[index..end_index].to_vec();
        assert_eq!(data.len(), dl);
        index = end_index;
        if index >= packet_length { return Err("Insufficient bytes to form a packet."); }
        if buffer[index] != PACKET_END { return Err("Final byte is not the end of packet marker."); }
        index += 1;
        Ok((Self{zone, cc, data}, index))
    }
}

impl fmt::Debug for Request {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut ds = f.debug_struct("Request");
        ds.field("zone", &self.zone);
        ds.field("cc", &self.cc);
        ds.field("data", &self.data);
        if self.cc == Command::SimulateRC5IRCommand {
            assert_eq!(self.data.len(), 2);
            ds.field("rc5command", &RC5Command::from(&self.data));
        } else if self.data.len() == 1 && self.data[0] == REQUEST_QUERY {
            ds.field("value", &"RequestQuery");
        } else if self.cc == Command::RequestCurrentSource {
            assert_eq!(self.data.len(), 1);
            let value: Source = FromPrimitive::from_u8(self.data[0]).unwrap();
            ds.field("value", &value);
        }
        ds.finish()
    }
}

/// A response from the amplifier.
#[derive(Clone, Eq, PartialEq)]
pub struct Response {
    pub zone: ZoneNumber,
    pub cc: Command,
    pub ac: AnswerCode,
    pub data: Vec<u8>,
}

impl Response {
    /// Create a new response.
    ///
    /// The data value is restricted to being at most 255 bytes long.
    pub fn new(zone: ZoneNumber, cc: Command, ac: AnswerCode, data: Vec<u8>) -> Result<Self, &'static str> {
        if data.len() > 255 { Err("data is too long for a response.") }
        else { Ok(Self{zone, cc, ac, data}) }
    }

    /// Return the byte sequence representing this response.
    ///
    /// All responses are structured:
    ///
    /// - St (Start transmission): PACKET_START
    /// - Zn (Zone number): 0x1, 0x2 for the zone number
    /// - Cc (Command code): the code for the command
    /// - Ac (Answer code): the answer code for the request
    /// - Dl (Data Length): the number of data items following this item, excluding the Et
    /// - Data: the parameters for the response of length n. n is limited to 255
    /// - Et (End transmission): PACKET_END
    pub fn to_bytes(self: &Self) -> Vec<u8> {
        let dl = self.data.len();
        if dl >= 256 { panic!("data length not right."); }
        let mut result = vec![PACKET_START, self.zone as u8, self.cc as u8, self.ac as u8, dl as u8];
        result.extend(self.data.iter());
        result.push(PACKET_END);
        result
    }

    /// Parse the bytes in the buffer to create a tuple representing a response and the
    /// number of bytes used for the request packet.
    ///
    /// All responses are structured;
    ///
    /// - St (Start transmission): PACKET_START ‘!’
    /// - Zn (Zone number): 0x1, 0x2 for the zone number
    /// - Cc (Command code): the code for the command
    /// - Ac (Answer code): the answer code for the request
    /// - Dl (Data Length): the number of data items following this item, excluding the ETR
    /// - Data: the parameters for the response of length n. n is limited to 255
    /// - Et (End transmission): PACKET_END
    pub fn parse_bytes(buffer: &[u8]) -> Result<(Self, usize), &'static str> {
        let packet_length = buffer.len();
        if packet_length < 6 { return Err("Insufficient bytes to form a packet."); }
        let mut index = 0;
        if buffer[index] != PACKET_START { return Err("First byte is not the start of packet marker."); }
        index += 1;
        if index >= packet_length { return Err("Insufficient bytes to form a packet."); }
        let zone = FromPrimitive::from_u8(buffer[index]).unwrap();
        index += 1;
        if index >= packet_length { return Err("Insufficient bytes to form a packet."); }
        let cc = FromPrimitive::from_u8(buffer[index]).unwrap();
        index += 1;
        if index >= packet_length { return Err("Insufficient bytes to form a packet."); }
        let ac = FromPrimitive::from_u8(buffer[index]).unwrap();
        index += 1;
        if index >= packet_length { return Err("Insufficient bytes to form a packet."); }
        let dl = buffer[index] as usize;
        index += 1;
        if index >= packet_length { return Err("Insufficient bytes to form a packet."); }
        let end_index = index + dl;
        if end_index >= packet_length { return Err("Insufficient bytes to form a packet."); }
        let data = buffer[index..end_index].to_vec();
        assert_eq!(data.len(), dl);
        index = end_index;
        if index >= packet_length { return Err("Insufficient bytes to form a packet."); }
        if buffer[index] != PACKET_END { return Err("Final byte is not the end of packet marker."); }
        index += 1;
        Ok((Self{zone, cc, ac, data}, index))
    }
}

impl fmt::Debug for Response {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut ds = f.debug_struct("Response");
        ds.field("zone", &self.zone);
        ds.field("cc", &self.cc);
        ds.field("ac", &self.ac);
        ds.field("data", &self.data);
        if self.cc == Command::SimulateRC5IRCommand {
            assert_eq!(self.data.len(), 2);
            ds.field("rc5command", &RC5Command::from(&self.data));
        }else if self.data.len() == 1 && self.data[0] == REQUEST_QUERY {
            ds.field("value", &"RequestQuery");
        } else if self.cc == Command::RequestCurrentSource {
            assert_eq!(self.data.len(), 1);
            let value: Source = FromPrimitive::from_u8(self.data[0]).unwrap();
            ds.field("value", &value);
        }
        ds.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use num_traits::FromPrimitive;

    #[test]
    fn create_display_brightness_request() {
        let request = Request::new(ZoneNumber::One, Command::DisplayBrightness, vec![REQUEST_QUERY]).unwrap();
        assert_eq!(
            request.to_bytes(),
            [PACKET_START, ZoneNumber::One as u8, Command::DisplayBrightness as u8, 0x01, REQUEST_QUERY, PACKET_END]
        );
    }

    #[test]
    fn create_get_volume_request() {
        let request = Request::new(ZoneNumber::One, Command::SetRequestVolume, vec![REQUEST_QUERY]).unwrap();
        assert_eq!(
            request.to_bytes(),
            [PACKET_START, ZoneNumber::One as u8, Command::SetRequestVolume as u8, 0x01, REQUEST_QUERY, PACKET_END]
        );
    }

    #[test]
    fn create_set_volume_request() {
        let request = Request::new(ZoneNumber::One, Command::SetRequestVolume, vec![20]).unwrap();
        assert_eq!(
            request.to_bytes(),
            [PACKET_START, ZoneNumber::One as u8, Command::SetRequestVolume as u8, 0x01, 0x14, PACKET_END]
        );
    }

    #[test]
    fn parse_empty_request_buffer() {
        if let Err(e) = Request::parse_bytes(&[]) {
            assert_eq!(e, "Insufficient bytes to form a packet.");
        };
    }

    #[test]
    fn parse_request_buffer_with_incorrect_start_marker() {
        if let Err(e) = Request::parse_bytes(&[21, 0, 0, 0, 0, 0]) {
            assert_eq!(e, "First byte is not the start of packet marker.");
        };
    }

    #[test]
    fn parse_valid_set_volume_request() {
        let mut request = Request::new(ZoneNumber::One, Command::SetRequestVolume, vec![20]).unwrap();
        assert_eq!(Request::parse_bytes(&request.to_bytes()).unwrap(), (request, 6));
    }

    #[test]
    fn parse_buffer_with_multiple_request_packets() {
        let r1 = Request::new(ZoneNumber::One, Command::RequestCurrentSource, vec![REQUEST_QUERY]).unwrap();
        let r2 = Request::new(ZoneNumber::One, Command::DisplayBrightness, vec![REQUEST_QUERY]).unwrap();
        let r3 = Request::new(ZoneNumber::One, Command::SetRequestVolume, vec![30u8]).unwrap();
        let mut input = r1.to_bytes();
        input.append(&mut r2.to_bytes());
        input.append(&mut r3.to_bytes());
        assert_eq!(input, vec![
            PACKET_START, ZoneNumber::One as u8, Command::RequestCurrentSource as u8, 1, REQUEST_QUERY, PACKET_END,
            PACKET_START, ZoneNumber::One as u8, Command::DisplayBrightness as u8, 1, REQUEST_QUERY, PACKET_END,
            PACKET_START, ZoneNumber::One as u8, Command::SetRequestVolume as u8, 1, 30, PACKET_END,
        ]);
        assert_eq!(
            Request::parse_bytes(&input).unwrap(),
            (r1, 6)
        );
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
        // We know that 0x61 is not known Command.
        let cc: Option<Command> = FromPrimitive::from_i8(0x61);
        assert!(cc.is_none());
    }

    #[test]
    fn data_length_must_be_less_than_256() {
        assert!(Request::new(ZoneNumber::One, Command::Power, vec![0u8; 300]).is_err());
    }

    #[test]
    fn create_display_brightness_response() {
        let response = Response::new(ZoneNumber::One, Command::DisplayBrightness, AnswerCode::StatusUpdate, vec![Brightness::Level1 as u8]).unwrap();
        assert_eq!(
            response.to_bytes(),
            [PACKET_START, ZoneNumber::One as u8, Command::DisplayBrightness as u8, AnswerCode::StatusUpdate as u8, 0x01, Brightness::Level1 as u8, PACKET_END]
        );
    }

    #[test]
    fn parse_empty_response_buffer() {
        if let Err(e) = Response::parse_bytes(&[]) {
            assert_eq!(e, "Insufficient bytes to form a packet.");
        };
    }

    #[test]
    fn parse_response_buffer_with_incorrect_start_marker() {
        if let Err(e) = Response::parse_bytes(&[21, 0, 0, 0, 0, 0]) {
            assert_eq!(e, "First byte is not the start of packet marker.");
        };
    }

    #[test]
    fn parse_valid_display_brightness_response() {
        let response = Response::new(ZoneNumber::One, Command::DisplayBrightness, AnswerCode::StatusUpdate, vec![0x01]).unwrap();
        assert_eq!(
            Response::parse_bytes(&response.to_bytes()).unwrap(),
            (response, 7)
        );
    }

    #[test]
    fn parse_buffer_with_multiple_response_packets() {
        let r1 = Response::new(ZoneNumber::One, Command::RequestCurrentSource, AnswerCode::StatusUpdate, vec![REQUEST_QUERY]).unwrap();
        let r2 = Response::new(ZoneNumber::One, Command::DisplayBrightness, AnswerCode::StatusUpdate, vec![REQUEST_QUERY]).unwrap();
        let r3 = Response::new(ZoneNumber::One, Command::SetRequestVolume, AnswerCode::StatusUpdate, vec![30u8]).unwrap();
        let mut input = r1.to_bytes();
        input.append(&mut r2.to_bytes());
        input.append(&mut r3.to_bytes());
        assert_eq!(input, vec![
            PACKET_START, ZoneNumber::One as u8, Command::RequestCurrentSource as u8, AnswerCode::StatusUpdate as u8, 1, REQUEST_QUERY, PACKET_END,
            PACKET_START, ZoneNumber::One as u8, Command::DisplayBrightness as u8, AnswerCode::StatusUpdate as u8, 1, REQUEST_QUERY, PACKET_END,
            PACKET_START, ZoneNumber::One as u8, Command::SetRequestVolume as u8, AnswerCode::StatusUpdate as u8, 1, 30, PACKET_END,
        ]);
        assert_eq!(
            Response::parse_bytes(&input).unwrap(),
            (r1, 7)
        );
    }

    //  Some real response packets from an AVR850.

    #[test]
    fn create_station_name_response() {
        let response = Response::new(ZoneNumber::One, Command::RequestDABStation, AnswerCode::StatusUpdate, "Smooth Country  ".as_bytes().to_vec()).unwrap();
        assert_eq!(
            response.to_bytes(),
            [33, 1, 24, 0, 16, 83, 109, 111, 111, 116, 104, 32, 67, 111, 117, 110, 116, 114, 121, 32, 32, 13]
        )
    }

    #[test]
    fn parse_station_name_response() {
        let input = [33, 1, 24, 0, 16, 83, 109, 111, 111, 116, 104, 32, 67, 111, 117, 110, 116, 114, 121, 32, 32, 13];
        let response = Response::new(ZoneNumber::One, Command::RequestDABStation, AnswerCode::StatusUpdate, "Smooth Country  ".as_bytes().to_vec()).unwrap();
        assert_eq!(Response::parse_bytes(&input).unwrap(), (response, 22));
    }

    #[test]
    fn create_station_category_response() {
        let response = Response::new(ZoneNumber::One, Command::ProgrammeTypeCategory, AnswerCode::StatusUpdate, "Country Music   ".as_bytes().to_vec()).unwrap();
        assert_eq!(
            response.to_bytes(),
            [33, 1, 25, 0, 16, 67, 111, 117, 110, 116, 114, 121, 32, 77, 117, 115, 105, 99, 32, 32, 32, 13]
        );
    }

    #[test]
    fn parse_station_category_response() {
        let input = [33, 1, 25, 0, 16, 67, 111, 117, 110, 116, 114, 121, 32, 77, 117, 115, 105, 99, 32, 32, 32, 13];
        let response = Response::new(ZoneNumber::One, Command::ProgrammeTypeCategory, AnswerCode::StatusUpdate, "Country Music   ".as_bytes().to_vec()).unwrap();
        assert_eq!(Response::parse_bytes(&input).unwrap(), (response, 22));
    }

    #[test]
    fn parse_station_rds_dls() {
        // String here is actually "Now on Smooth: Living In A Box with Room In Your Heart"
        let input = [33, 1, 26, 0, 129, 12, 78, 111, 119, 32, 111, 110, 32, 83, 109, 111, 111, 116, 104, 58,
                32, 76, 105, 118, 105, 110, 103, 32, 73, 110, 32, 65, 32, 66, 111, 120, 32, 119, 105, 116, 104, 32, 82,
                111, 111, 109, 32, 73, 110, 32, 89, 111, 117, 114, 32, 72, 101, 97, 114, 116, 0, 0, 32, 32, 32, 32, 32,
                32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32,
                32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32,
                32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 13,];
        assert_eq!(input.len(), 135);
        let response = Response::new(ZoneNumber::One, Command::DLSPDTInformation, AnswerCode::StatusUpdate,
                                     vec![12, 78, 111, 119, 32, 111, 110, 32, 83, 109, 111, 111, 116, 104, 58,
                  32, 76, 105, 118, 105, 110, 103, 32, 73, 110, 32, 65, 32, 66, 111, 120, 32, 119, 105, 116, 104, 32, 82,
                  111, 111, 109, 32, 73, 110, 32, 89, 111, 117, 114, 32, 72, 101, 97, 114, 116, 0, 0, 32, 32, 32, 32, 32,
                  32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32,
                  32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32,
                  32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32,]).unwrap();
        assert_eq!(Response::parse_bytes(&input).unwrap(), (response, 135));
    }

    #[test]
    fn rc5_data_correctly_accessed() {
        assert_eq!(RC5DATA[&RC5Command::Nine], (0x10, 0x09));
        assert_eq!(get_rc5command_data(RC5Command::Nine), (0x10, 0x09));
    }

    #[test]
    fn rc5command_round_trip() {
        let expected = RC5Command::One;
        let result: RC5Command = get_rc5command_data(expected).into();
        assert_eq!(result, expected);
    }

}
