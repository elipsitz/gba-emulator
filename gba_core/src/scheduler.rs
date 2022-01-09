use std::collections::BinaryHeap;

/// Scheduler, which manages events that happen at certain timestamps.
pub struct Scheduler {
    /// The current time (in cycles).
    time: usize,

    /// Priority queue of events.
    queue: BinaryHeap<ScheduledEvent>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// Stop running the emulator.
    StopRunning,

    /// A PPU Event.
    Ppu(PpuEvent),

    /// Activate a DMA channel.
    DmaActivate(u8),

    /// Timers need to be updated.
    TimerUpdate,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PpuEvent {
    /// End of h-draw (during v-draw).
    EndHDraw,

    /// End of h-blank (during v-draw).
    EndHBlank,

    /// End of h-draw (during v-blank).
    EndVBlankHDraw,

    /// End of h-blank (during v-blank).
    EndVBlankHBlank,
}

impl Scheduler {
    pub fn new() -> Scheduler {
        Scheduler {
            queue: BinaryHeap::new(),
            time: 0,
        }
    }

    /// Returns the current timestamp.
    pub fn timestamp(&self) -> usize {
        self.time
    }

    /// Increment the internal timestamp by `delta` cycles.
    pub fn update(&mut self, delta: usize) {
        self.time += delta;
    }

    /// Advance the internal timestamp to the timestamp of the next event.
    pub fn skip_to_next_event(&mut self) {
        let next_deadline = self.peek_deadline().unwrap();
        self.time = self.time.max(next_deadline);
    }

    /// Get the timestamp of the next event's deadline (or None if there are no events).
    pub fn peek_deadline(&self) -> Option<usize> {
        self.queue.peek().map(|x| x.deadline)
    }

    /// Pop the next fired event (or None). Returns the number of cycles we were late by.
    pub fn pop_event(&mut self) -> Option<(Event, usize)> {
        if let Some(next_event) = self.queue.peek() {
            if next_event.deadline <= self.time {
                let event = unsafe { self.queue.pop().unwrap_unchecked() };
                let lateness = self.time - event.deadline;
                return Some((event.event, lateness));
            }
        }
        None
    }

    /// Cancels a scheduled event.
    ///
    /// Takes O(N) time.
    pub fn cancel_event(&mut self, event: Event) {
        // TODO: use [`BinaryHeap::retain`] if/when it's stabilized (it's more efficient).
        let mut queue = std::mem::take(&mut self.queue).into_vec();
        queue.retain(|f| f.event != event);
        self.queue = queue.into();
    }

    /// Schedule an event at a moment in time (now + given cycles).
    pub fn push_event(&mut self, event: Event, when: usize) {
        let scheduled = ScheduledEvent {
            event,
            deadline: self.time + when,
        };
        self.queue.push(scheduled);
    }
}

#[derive(Debug)]
pub struct ScheduledEvent {
    /// Time at which the event should fire.
    deadline: usize,

    /// The event.
    event: Event,
}

impl PartialEq for ScheduledEvent {
    fn eq(&self, other: &Self) -> bool {
        self.deadline == other.deadline
    }
}

impl Eq for ScheduledEvent {}

impl PartialOrd for ScheduledEvent {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.deadline
            .partial_cmp(&other.deadline)
            .map(|x| x.reverse())
    }
}

impl Ord for ScheduledEvent {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.deadline.cmp(&other.deadline).reverse()
    }
}
