/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//! A timer thread that gives the painting task a little time to catch up when the user scrolls.

use compositor_task::{CompositorProxy, ScrollTimeout};

use native::task::NativeTaskBuilder;
use std::io::timer;
use std::task::TaskBuilder;
use std::time::duration::Duration;
use time;

/// The amount of time in nanoseconds that we give to the painting thread to paint new tiles upon
/// processing a scroll event that caused new tiles to be revealed. When this expires, we give up
/// and composite anyway (showing a "checkerboard") to avoid dropping the frame.
static TIMEOUT: i64 = 12_000_000;

pub struct ScrollingTimerProxy {
    sender: Sender<ToScrollingTimerMsg>,
}

pub struct ScrollingTimer {
    compositor_proxy: Box<CompositorProxy>,
    receiver: Receiver<ToScrollingTimerMsg>,
}

enum ToScrollingTimerMsg {
    ExitMsg,
    ScrollEventProcessedMsg(u64),
}

impl ScrollingTimerProxy {
    pub fn new(compositor_proxy: Box<CompositorProxy+Send>) -> ScrollingTimerProxy {
        let (to_scrolling_timer_sender, to_scrolling_timer_receiver) = channel();
        TaskBuilder::new().native().spawn(proc() {
            let mut scrolling_timer = ScrollingTimer {
                compositor_proxy: compositor_proxy,
                receiver: to_scrolling_timer_receiver,
            };
            scrolling_timer.run();
        });
        ScrollingTimerProxy {
            sender: to_scrolling_timer_sender,
        }
    }

    pub fn scroll_event_processed(&mut self, timestamp: u64) {
        self.sender.send(ScrollEventProcessedMsg(timestamp))
    }

    pub fn shutdown(&mut self) {
        self.sender.send(ExitMsg);
    }
}

impl ScrollingTimer {
    pub fn run(&mut self) {
        loop {
            match self.receiver.recv_opt() {
                Ok(ScrollEventProcessedMsg(timestamp)) => {
                    let target = timestamp as i64 + TIMEOUT;
                    let delta = target - (time::precise_time_ns() as i64);
                    timer::sleep(Duration::nanoseconds(delta));
                    self.compositor_proxy.send(ScrollTimeout(timestamp));
                }
                Ok(ExitMsg) | Err(_) => break,
            }
        }
    }
}
