//! # Real Time Clock (RTC)

use {
    core::{fmt, ops::Add},
    x86_64::instructions::port::Port,
};

const CMOS_COMMAND_PORT: u16 = 0x70;
const CMOS_DATA_PORT: u16 = 0x71;
// https://wiki.osdev.org/CMOS#Non-Maskable_Interrupts
const CMOS_DISABLE_NMI: u8 = 1 << 7;

const SECOND_REGISTER: u8 = 0x00;
const MINUTE_REGISTER: u8 = 0x02;
const HOUR_REGISTER: u8 = 0x04;
const DAY_REGISTER: u8 = 0x07;
const MONTH_REGISTER: u8 = 0x08;
const YEAR_REGISTER: u8 = 0x09;

const STATUS_REGISTER_A: u8 = 0x0A;
const STATUS_REGISTER_B: u8 = 0x0B;

const FORMAT_24_HOUR_FLAG: u8 = 1 << 1;
const FORMAT_BINARY_FLAG: u8 = 1 << 2;
const UPDATE_IN_PROGRESS_FLAG: u8 = 1 << 7;
const HOUR_PM_FLAG: u8 = 1 << 7;


#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Time {
    pub year: u8,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

impl fmt::Display for Time {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!(
            "{}/{}/20{} {:02}:{:02}:{:02}",
            self.month, self.day, self.year, self.hour, self.minute, self.second,
        ))
    }
}

impl Time {
    pub const ZERO: Self = Self {
        year: 0,
        month: 0,
        day: 0,
        hour: 0,
        minute: 0,
        second: 0,
    };

    pub fn now() -> Self {
        loop {
            // Wait for the current update to finish.
            while read_cmos(STATUS_REGISTER_A) & UPDATE_IN_PROGRESS_FLAG > 0 {
                core::hint::spin_loop();
            }

            let time_1 = unsafe { Self::now_unsynced() };

            // If the clock is already updating the time again, retry.
            if read_cmos(STATUS_REGISTER_A) & UPDATE_IN_PROGRESS_FLAG > 0 {
                continue;
            }

            let time_2 = unsafe { Self::now_unsynced() };
            if time_1 == time_2 {
                return time_1;
            }
        }
    }

    // https://wiki.osdev.org/CMOS#Reading_All_RTC_Time_and_Date_Registers
    pub unsafe fn now_unsynced() -> Self {
        let mut second = read_cmos(SECOND_REGISTER);
        let mut minute = read_cmos(MINUTE_REGISTER);
        let mut hour = read_cmos(HOUR_REGISTER);
        let mut day = read_cmos(DAY_REGISTER);
        let mut month = read_cmos(MONTH_REGISTER);
        let mut year = read_cmos(YEAR_REGISTER);

        let format = read_cmos(STATUS_REGISTER_B);
        if format & FORMAT_BINARY_FLAG != FORMAT_BINARY_FLAG {
            second = convert_bcd(second);
            minute = convert_bcd(minute);
            hour = convert_bcd(hour & 0x7F) | (hour & 0x80);
            day = convert_bcd(day);
            month = convert_bcd(month);
            year = convert_bcd(year);
        }

        let after_noon = hour & HOUR_PM_FLAG == HOUR_PM_FLAG;

        if format & FORMAT_24_HOUR_FLAG > 0 || after_noon {
            hour = ((hour & 0x7F) + 12) % 24;
        }

        Self {
            year,
            month,
            day,
            hour,
            minute,
            second,
        }
    }

    pub const fn from_seconds(seconds: u64) -> Self {
        if seconds >= 3600 * 24 {
            todo!()
        } else {
            if seconds >= 3600 {
                Self {
                    second: (seconds % 60) as u8,
                    minute: (seconds / 60) as u8,
                    hour: (seconds / 3600) as u8,
                    ..Self::ZERO
                }
            } else {
                if seconds >= 60 {
                    Self {
                        second: (seconds % 60) as u8,
                        minute: (seconds / 60) as u8,
                        ..Self::ZERO
                    }
                } else {
                    Self {
                        second: seconds as u8,
                        ..Self::ZERO
                    }
                }
            }
        }
    }
}

impl Add for Time {
    type Output = Self;

    fn add(mut self, rhs: Self) -> Self {
        let seconds = self.second + rhs.second;
        if seconds >= 60 {
            self.minute += 1;
            self.second = seconds - 60;
        } else {
            self.second = seconds;
        }
        let minutes = self.minute + rhs.minute;
        if minutes >= 60 {
            self.hour += 1;
            self.minute = minutes - 60;
        } else {
            self.minute = minutes;
        }
        let hours = self.hour + rhs.hour;
        if hours >= 24 {
            self.day += 1;
            self.hour = hours - 24;
        } else {
            self.hour = hours;
        }

        // TODO: Days, months, years.

        self
    }
}

const fn convert_bcd(value: u8) -> u8 {
    (value & 0x0F) + ((value / 16) * 10)
}

fn read_cmos(register: u8) -> u8 {
    unsafe {
        Port::new(CMOS_COMMAND_PORT).write(CMOS_DISABLE_NMI | register);
        Port::new(CMOS_DATA_PORT).read()
    }
}

// fn write_cmos(register: u8, value: u8) {
//     unsafe {
//         Port::new(CMOS_COMMAND_PORT).write(CMOS_DISABLE_NMI | register);
//         Port::new(CMOS_DATA_PORT).write(value)
//     }
// }
