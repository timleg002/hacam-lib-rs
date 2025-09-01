use chrono::{Datelike as _, Timelike as _};

/// Trait implemented by all setting enums representing a 2D resolution.
pub trait Resolution {
    /// Gets the width.
    fn w(&self) -> u32;

    /// Gets the height.
    fn h(&self) -> u32;
}

#[repr(i8)]
#[derive(Debug, Clone, Copy, Default, int_enum::IntEnum)]
/// Specifies the resolution for the live view.
pub enum LiveViewResolution {
    #[default]
    /// 1280 x 640
    Low = 10, // 0
    /// 1920 x 960
    High = 9, // 1
}

impl Resolution for LiveViewResolution {
    fn w(&self) -> u32 {
        match self {
            Self::High => 1920,
            Self::Low => 1280,
        }
    }

    fn h(&self) -> u32 {
        match self {
            Self::High => 960,
            Self::Low => 640,
        }
    }
}

#[repr(i8)]
#[derive(Debug, Clone, Copy, Default, int_enum::IntEnum)]
/// Specifies the spherical picture orientation.
pub enum PictureOrientation {
    #[default]
    /// 0°
    Deg0 = 2,
    /// 90°
    Deg90 = 3,
    /// 180°
    Deg180 = 0,
    /// 270°
    Deg270 = 1,
}

#[repr(i8)]
#[derive(Debug, Clone, Copy, Default, int_enum::IntEnum)]
/// Specifies the resolution for the picture.
pub enum PhotoResolution {
    /// 5376 x 2688
    High = 3, // 0
    #[default]
    /// 3840 x 1920
    Low = 4, // 1
}

impl Resolution for PhotoResolution {
    fn w(&self) -> u32 {
        match self {
            Self::High => 5376,
            Self::Low => 3840,
        }
    }

    fn h(&self) -> u32 {
        match self {
            Self::High => 2688,
            Self::Low => 1920,
        }
    }
}

#[repr(i8)]
#[derive(Debug, Clone, Copy, Default, int_enum::IntEnum)]
/// Specifies the resolution for recording video.
pub enum VideoResolution {
    /// 1920 x 960
    High = 9, // 0
    /// 1280 x 640
    #[default]
    Low = 10, // 1
    /// Undocumented.
    Unknown = 11,
}

impl Resolution for VideoResolution {
    fn w(&self) -> u32 {
        match self {
            Self::High => 1920,
            Self::Low => 1280,
            Self::Unknown => 1280,
        }
    }

    fn h(&self) -> u32 {
        match self {
            Self::High => 960,
            Self::Low => 640,
            Self::Unknown => 640,
        }
    }
}

#[repr(i8)]
#[derive(Debug, Clone, Copy, Default, int_enum::IntEnum)]
/// Specifies the exposure value compensation.
pub enum EvValue {
    #[default]
    None = 0,
    Neg2 = 1,
    Neg1_67,
    Neg1_33,
    Neg1,
    Neg0_67,
    Neg0_33,
    Pos0_33,
    Pos0_67,
    Pos1,
    Pos1_33,
    Pos1_67,
    Pos2,
}

#[repr(i8)]
#[derive(Debug, Clone, Copy, Default, int_enum::IntEnum)]
/// Specifies the white balance as a preset.
pub enum WhiteBalance {
    #[default]
    Auto,
    Sunny,
    Cloudy,
    Tungsten,
    Fluorescent = 4,
}

#[repr(i8)]
#[derive(Debug, Clone, Copy, Default, int_enum::IntEnum)]
/// Specifies the camera color filter.
pub enum FilterValue {
    #[default]
    None,
    Faded,
    Nimbus,
    Tea,
    Twilight,
    Sapphire,
    Vintage,
    Greyscale,
    Newspaper,
}

#[repr(i8)]
#[derive(Debug, Clone, Copy, Default, int_enum::IntEnum)]
/// Specifies the logo type superimposed on the camera. (either the Huawei logo or no logo)
pub enum LogoType {
    HuaweiLogo = 1,
    #[default]
    None = 0,
}

#[repr(i8)]
#[derive(Debug, Clone, Copy, Default, int_enum::IntEnum)]
/// Specifies the bitrate. Usually a higher bitrate is set for higher quality video.
pub enum Bitrate {
    #[default]
    Unset = 0,
    Bitrate0 = 4,
    Bitrate1 = 8,
    Bitrate2 = 16,
}

#[repr(i8)]
#[derive(Debug, Clone, Copy, int_enum::IntEnum)]
/// Represents a specific setting type, such as a `PhotoResolution` setting.
pub enum SettingType {
    PhotoResolution = 3,
    VideoResolution = 4,
    EvBalance = 7,
    WhiteBalance = 8,
    Filter = 9,
    ShutterTime = 16,
    LogoType = 17,
    Bitrate = 12,
}

#[derive(Debug, Clone, Default)]
pub struct CamSettings {
    pub photo_resolution: PhotoResolution, // @2
    pub video_resolution: VideoResolution, // @3
    pub evb: EvValue,                      // @6
    pub wb: WhiteBalance,                  // @7
    pub date_time: chrono::NaiveDateTime,  // @10-19
    pub filter: FilterValue,               // @32
    pub bitrate: Bitrate,                  // @35
    pub logo_type: LogoType,               // @39
}

impl CamSettings {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bfr = vec![0; 48];

        bfr[2] = self.photo_resolution as u8;
        bfr[3] = self.video_resolution as u8;
        bfr[6] = self.evb as u8;
        bfr[7] = self.wb as u8;

        bfr[10] = self.date_time.year() as u8;
        bfr[11] = (self.date_time.year() >> 8) as u8;
        bfr[12] = self.date_time.month() as u8;
        bfr[13] = self.date_time.day() as u8;
        bfr[14] = self.date_time.hour() as u8;
        bfr[15] = self.date_time.minute() as u8;
        bfr[16] = self.date_time.second() as u8;
        bfr[18] = (self.date_time.nanosecond() / 1_000_000) as u8;
        bfr[19] = ((self.date_time.nanosecond() / 1_000_000) >> 8) as u8;

        bfr[32] = self.filter as u8;
        bfr[35] = self.bitrate as u8;
        bfr[39] = self.logo_type as u8;

        bfr
    }

    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < 40 {
            return None;
        }

        let photo_resolution = PhotoResolution::try_from(data[2] as i8).ok()?;
        let video_resolution = VideoResolution::try_from(data[3] as i8).ok()?;
        let evb = EvValue::try_from(data[6] as i8).ok()?;
        let wb = WhiteBalance::try_from(data[7] as i8).ok()?;

        let year = u16::from_le_bytes([data[10], data[11]]);
        let month = data[12];
        let day = data[13];
        let hour = data[14];
        let minute = data[15];
        let second = data[16];
        let ms = u16::from_le_bytes([data[18], data[19]]);

        let date = chrono::NaiveDate::from_ymd_opt(year as i32, month as u32, day as u32)?;
        let time = chrono::NaiveTime::from_hms_milli_opt(
            hour as u32,
            minute as u32,
            second as u32,
            ms as u32,
        )?;
        let date_time = chrono::NaiveDateTime::new(date, time);

        let filter = FilterValue::try_from(data[32] as i8).ok()?;
        let bitrate = Bitrate::try_from(data[35] as i8).ok()?;
        let logo_type = LogoType::try_from(data[39] as i8).ok()?;

        Some(Self {
            photo_resolution,
            video_resolution,
            evb,
            wb,
            date_time,
            filter,
            bitrate,
            logo_type,
        })
    }
}