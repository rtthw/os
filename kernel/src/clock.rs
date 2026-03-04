//! # System Clock

use core::{
    pin::Pin,
    task::{Context, Poll},
};

use uefi::runtime::get_time;



#[derive(Clone)]
pub struct SystemClock {
    period_secs: f64,
}

impl SystemClock {
    pub const fn new(bp_speed: ClockSpeed) -> Self {
        SystemClock {
            period_secs: bp_speed.period_seconds(),
        }
    }

    pub fn time(&self) -> f64 {
        let cycle = unsafe { raw_cpu_cycle() };
        self.period_secs * cycle as f64
    }

    pub fn delay(&self, secs: f64) {
        let start_time = self.time();
        while self.time() - start_time < secs {
            core::hint::spin_loop();
        }
    }

    pub fn delay_future(&self, secs: f64) -> Delay {
        Delay {
            clock: self.clone(),
            seconds: secs,
            start_time: self.time(),
        }
    }
}

#[inline]
pub unsafe fn raw_cpu_cycle() -> u64 {
    unsafe { core::arch::x86_64::_rdtsc() }
}



pub struct ClockSpeed {
    pub cycles_per_second: u64,
}

impl ClockSpeed {
    pub fn guess() -> Self {
        let init_sec = get_time().unwrap().second();
        let start_sec = loop {
            let current_sec = get_time().unwrap().second();
            if init_sec != current_sec {
                break current_sec;
            }
        };

        let start_cycle = unsafe { raw_cpu_cycle() };
        loop {
            let current_sec = get_time().unwrap().second();
            if start_sec != current_sec {
                break;
            }
        }
        let end_cycle = unsafe { raw_cpu_cycle() };

        Self {
            cycles_per_second: end_cycle - start_cycle,
        }
    }

    #[inline]
    pub const fn period_seconds(&self) -> f64 {
        1.0 / self.cycles_per_second as f64
    }

    #[inline]
    pub const fn frequency_gigahertz(&self) -> f64 {
        1.0 / self.period_seconds() / 1.0_e9
    }
}



pub struct Delay {
    clock: SystemClock,
    seconds: f64,
    start_time: f64,
}

impl Future for Delay {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.clock.time() - self.start_time < self.seconds {
            cx.waker().wake_by_ref();
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    }
}
