// Copyright 2025 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use core::sync::atomic::Ordering::Relaxed;

use portable_atomic_util::Arc;
use wasefire::sync::{AtomicBool, Mutex};
use wasefire::timer::{self, Mode, Timer};
use core::time::Duration;

pub(crate) struct Touch {
    touched: Arc<AtomicBool>,
}

impl Touch {
    pub(crate) fn new() -> Self {
        Touch {
            touched: State::start(&mut STATE.lock()),
        }
    }

    pub(crate) fn is_present(&self) -> bool {
        self.touched.load(Relaxed)
    }
}

impl Drop for Touch {
    fn drop(&mut self) {
        if Arc::strong_count(&self.touched) == 2 {
            // We're the last object, so we can stop listening.
            *STATE.lock() = None;
        }
    }
}

static STATE: Mutex<Option<State>> = Mutex::new(None);

struct State {
    _timer: Timer<Handler>,
    _blink: crate::blink::Blink,
    touched: Arc<AtomicBool>,
}

impl State {
    fn start(this: &mut Option<State>) -> Arc<AtomicBool> {
        match this {
            None => {
                let blink = crate::blink::Blink::new_ms(500);
                let touched = Arc::new(AtomicBool::new(false));

                // 1. Lấy giá trị ngẫu nhiên từ RNG
                let mut rand_bytes = [0u8; 4];
                wasefire::rng::fill_bytes(&mut rand_bytes).unwrap();
                let random_val = u32::from_le_bytes(rand_bytes);

                // 2. Tính delay ngẫu nhiên từ 0 đến 3000ms
                let delay_ms = (random_val % 3001) as u64;

                // 3. Khởi tạo Timer theo API mới của Wasefire
                let timer = Timer::new(Handler);
                timer.start(Mode::Oneshot, Duration::from_millis(delay_ms));

                *this = Some(State {
                    _timer: timer,
                    _blink: blink,
                    touched: touched.clone(),
                });
                touched
            }
            Some(state) => state.touched.clone(),
        }
    }

    fn touch(this: &mut Option<State>) {
        let Some(state) = this.take() else {
            unreachable!()
        };
        state.touched.store(true, Relaxed);
    }
}

struct Handler;

impl timer::Handler for Handler {
    fn event(&self) {
        State::touch(&mut STATE.lock());
    }
}
