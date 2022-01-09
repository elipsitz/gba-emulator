use bit::BitIndex;

use crate::{interrupt::InterruptKind, scheduler::Event, Gba};

const NUM_TIMERS: usize = 4;
const OVERFLOW: usize = 0x1_0000;
const INTERRUPTS: [InterruptKind; 4] = [
    InterruptKind::Timer0,
    InterruptKind::Timer1,
    InterruptKind::Timer2,
    InterruptKind::Timer3,
];

/// State for the timers in the system.
///
/// The timers aren't incremented every "clock cycle" like they would be in hardware.
/// Instead, we update them in bulk by comparing the current scheduler timestamp
/// to the last time they were updated. We do these updates only when we need to:
/// when a timer register is read or written. We also keep track of when the next
/// overflow IRQ is going to happen, and set up a scheduler event so we don't miss it.
pub struct TimerManager {
    timers: [Timer; NUM_TIMERS],
    /// Last timestamp the timers were updated.
    last_update: usize,
}

/// A single timer.
#[derive(Default)]
struct Timer {
    /// Current count of the timer.
    count: u16,
    /// Initial/reload count of the timer.
    initial_count: u16,
    /// Control register.
    control: TimerControl,
}

/// Timer control register.
#[derive(Copy, Clone, Default)]
struct TimerControl(u16);

impl TimerControl {
    /// Returns the prescaler period in cycles.
    fn period(self) -> usize {
        [1, 64, 256, 1024][self.0.bit_range(0..2) as usize]
    }

    /// Returns if this timer is in cascade mode.
    fn cascade(self) -> bool {
        self.0.bit(2)
    }

    /// Returns if this timer will raise an interrupt on overflow.
    fn irq(self) -> bool {
        self.0.bit(6)
    }

    /// Returns if this timer is enabled.
    fn enabled(self) -> bool {
        self.0.bit(7)
    }
}

impl TimerManager {
    pub fn new() -> TimerManager {
        TimerManager {
            timers: <[Timer; NUM_TIMERS]>::default(),
            last_update: 0,
        }
    }
}

/// Get the number of multiples of the period in the range.
/// For example, the 1024 prescaler ticks at 0, 1024, 2048, etc.
/// In the range (500, 600], there are no ticks. In the range (1020, 1030],
/// there is one. (1023, 1024] -> 1. (1024, 1025] -> 0.
/// The start is exclusive and the end (start + duration) is inclusive.
#[inline(always)]
fn multiples_in_range(period: usize, start: usize, duration: usize) -> usize {
    ((start + duration) / period) - (start / period)
}

impl Gba {
    /// Update timers so that their state is such that they have been ticking
    /// this whole time since the last update.
    ///
    /// The first possible prescaler clock tick happens at (last_update + 1).
    ///
    /// This may result in interrupts. For accurate timing, this should be called
    /// as soon as possible after an interrupt would happen (using the
    /// [`calculate_next_irq`] function).
    fn update_timers(&mut self) {
        let last_update = self.timer.last_update;
        let steps = self.scheduler.timestamp() - last_update;
        self.timer.last_update = self.scheduler.timestamp();

        // The number of times the previous timer has overflowed.
        let mut last_overflows = 0;
        for i in 0..NUM_TIMERS {
            let timer = &mut self.timer.timers[i];
            if timer.control.enabled() {
                let increment = if timer.control.cascade() {
                    last_overflows
                } else {
                    multiples_in_range(timer.control.period(), last_update, steps)
                };

                // The counter range [initial, 0x10000) is rescaled to [0, period).
                let initial = timer.initial_count as usize;
                let period = OVERFLOW - initial;

                // The current count could be lower than the initial if the initial was
                // changed while the timer was running.
                let time_to_overflow = OVERFLOW - (timer.count as usize);
                if increment >= time_to_overflow {
                    timer.count = (((increment - time_to_overflow) % period) + initial) as u16;
                    last_overflows = 1 + ((increment - time_to_overflow) / period);
                } else {
                    timer.count += increment as u16;
                }

                if last_overflows > 0 {
                    self.interrupt_raise(INTERRUPTS[i]);
                }
            }
        }
    }

    /// Calculate how many cycles until the next time we may have to fire an IRQ.
    fn calculate_next_irq(&mut self) -> Option<usize> {
        // Early return: if no enabled timers have IRQ set, no IRQ needed.
        let irq_possible = (0..NUM_TIMERS)
            .any(|i| self.timer.timers[i].control.enabled() && self.timer.timers[i].control.irq());
        if !irq_possible {
            return None;
        }
        let mut first_irq: Option<usize> = None;

        // If the previous timer will overflow,
        // the time until the next overflow and the period of subsequent overflows.
        let timestamp = self.scheduler.timestamp();
        let mut last_overflow: Option<(usize, usize)> = None;
        for i in 0..NUM_TIMERS {
            let timer = &mut self.timer.timers[i];
            if timer.control.enabled() {
                let next_ticks = if timer.control.cascade() {
                    // This timer's ticks depend on the previous timer's overflows.
                    last_overflow
                } else {
                    // This timer's ticks depend on premultiplier.
                    let period = timer.control.period();
                    let next_tick = period - (timestamp % period);
                    Some((next_tick, period))
                };
                if let Some((first_tick, tick_period)) = next_ticks {
                    // Ticks needed until next overflow.
                    let needed = OVERFLOW - (timer.count as usize);
                    let first_overflow = first_tick + (tick_period * (needed - 1));
                    // Ticks needed for next overflow.
                    let needed_next = OVERFLOW - (timer.initial_count as usize);
                    let next_overflow = tick_period * needed_next;
                    last_overflow = Some((first_overflow, next_overflow));

                    if next_overflow < first_irq.unwrap_or(usize::MAX) {
                        first_irq = Some(next_overflow);
                    }
                } else {
                    last_overflow = None;
                }
            } else {
                // This timer is not enabled and thus will never overflow.
                last_overflow = None;
            }
        }

        first_irq
    }

    /// Schedule an event to make us update timers when there's an IRQ.
    /// Assumes the timers are up-to-date.
    /// Optionally cancels the previous IRQ events.
    fn schedule_irq_event(&mut self, cancel_others: bool) {
        if cancel_others {
            self.scheduler.cancel_event(Event::TimerUpdate);
        }
        if let Some(next_irq) = self.calculate_next_irq() {
            self.scheduler.push_event(Event::TimerUpdate, next_irq);
        }
    }

    /// Handle a scheduler timer update event.
    pub(crate) fn timer_handle_event(&mut self) {
        self.update_timers();
        self.schedule_irq_event(false);
    }

    pub(crate) fn timer_write_counter(&mut self, index: usize, value: u16) {
        self.update_timers();
        self.timer.timers[index].initial_count = value;
        self.schedule_irq_event(true);
    }

    pub(crate) fn timer_read_counter(&mut self, index: usize) -> u16 {
        self.update_timers();
        self.timer.timers[index].count
    }

    pub(crate) fn timer_write_control(&mut self, index: usize, value: u16) {
        self.update_timers();
        let new = TimerControl(value);
        let old = self.timer.timers[index].control;
        if new.enabled() && !old.enabled() {
            self.timer.timers[index].count = self.timer.timers[index].initial_count;
        }
        self.timer.timers[index].control = new;
        self.schedule_irq_event(true);
    }

    pub(crate) fn timer_read_control(&mut self, index: usize) -> u16 {
        self.timer.timers[index].control.0
    }
}
