// Copyright 2024 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::libslirp_config;
use crate::libslirp_config::SlirpConfigs;
///
/// This crate is a wrapper for libslirp C library.
///
/// All calls into libslirp are routed to and handled by a dedicated
/// thread.
///
/// Rust struct LibslirpConfig for conversion between Rust and C types
/// (IpV4Addr, SocketAddrV4, etc.).
///
/// Callbacks for libslirp send_packet are delivered on Channel.
///
use crate::libslirp_sys;
use bytes::Bytes;
use core::sync::atomic::{AtomicUsize, Ordering};
use lazy_static::lazy_static;
use log::{debug, info, warn};
use std::collections::HashMap;
use std::ffi::{c_char, c_int, c_void, CStr};
use std::sync::{mpsc, Mutex};
use std::thread;
use std::time::Duration;
use std::time::Instant;

// Uses a static to hold callback state instead of the libslirp's
// opaque parameter to limit the number of unsafe regions.
static CONTEXT: Mutex<CallbackContext> =
    Mutex::new(CallbackContext { tx_bytes: None, tx_cmds: None, poll_fds: Vec::new() });

// Timers are managed across the ffi boundary using a unique usize ID
// (TimerOpaque) and a hashmap rather than memory pointers to reduce
// unsafe code.

lazy_static! {
    static ref TIMERS: Mutex<TimerManager> = Mutex::new(TimerManager {
        clock: Instant::now(),
        map: HashMap::new(),
        timers: AtomicUsize::new(1),
    });
}

type TimerOpaque = usize;

struct TimerManager {
    clock: Instant,
    map: HashMap<TimerOpaque, Timer>,
    timers: AtomicUsize,
}

#[derive(Clone)]
struct Timer {
    id: libslirp_sys::SlirpTimerId,
    cb_opaque: usize,
    expire_time: u64,
}

// The operations performed on the slirp thread

enum SlirpCmd {
    Input(Bytes),
    PollResult(Vec<PollFd>, c_int),
    TimerModified,
    Shutdown,
}

#[derive(Default)]
struct CallbackContext {
    tx_bytes: Option<mpsc::Sender<Bytes>>,
    tx_cmds: Option<mpsc::Sender<SlirpCmd>>,
    poll_fds: Vec<PollFd>,
}

// A poll thread request has a poll_fds and a timeout
type PollRequest = (Vec<PollFd>, u32);

// API to LibSlirp

pub struct LibSlirp {
    tx_cmds: mpsc::Sender<SlirpCmd>,
}

impl TimerManager {
    fn next_timer(&self) -> TimerOpaque {
        self.timers.fetch_add(1, Ordering::SeqCst) as TimerOpaque
    }

    // Finds expired Timers, clears then clones them
    fn collect_expired(&mut self) -> Vec<Timer> {
        let now_ms = self.clock.elapsed().as_millis() as u64;
        self.map
            .iter_mut()
            .filter(|(_, timer)| timer.expire_time < now_ms)
            .map(|(_, &mut ref mut timer)| {
                timer.expire_time = u64::MAX;
                timer.clone()
            })
            .collect()
    }

    // Return the minimum duration until the next timer
    fn min_duration(&self) -> Duration {
        match self.map.iter().min_by_key(|(_, timer)| timer.expire_time) {
            Some((_, &ref timer)) => {
                let now_ms = self.clock.elapsed().as_millis() as u64;
                // Duration is >= 0
                Duration::from_millis(timer.expire_time.saturating_sub(now_ms))
            }
            None => Duration::from_millis(u64::MAX),
        }
    }
}

impl LibSlirp {
    pub fn new(config: libslirp_config::SlirpConfig, tx_bytes: mpsc::Sender<Bytes>) -> LibSlirp {
        // Initialize the callback context
        let mut guard = CONTEXT.lock().unwrap();
        if guard.tx_bytes.is_some() {
            panic!("LibSlirp::new called twice");
        }
        guard.tx_bytes = Some(tx_bytes);

        let (tx_cmds, rx_cmds) = mpsc::channel::<SlirpCmd>();
        let (tx_poll, rx_poll) = mpsc::channel::<PollRequest>();

        guard.tx_cmds = Some(tx_cmds.clone());

        // Create channels for polling thread and launch
        let tx_cmds_poll = tx_cmds.clone();
        if let Err(e) = thread::Builder::new()
            .name("slirp_poll".to_string())
            .spawn(move || slirp_poll_thread(rx_poll, tx_cmds_poll))
        {
            warn!("Failed to start slirp poll thread: {}", e);
        }

        // Create channels for command processor thread and launch
        if let Err(e) = thread::Builder::new()
            .name("slirp".to_string())
            .spawn(move || slirp_thread(config, rx_cmds, tx_poll))
        {
            warn!("Failed to start slirp thread: {}", e);
        }

        LibSlirp { tx_cmds }
    }

    pub fn shutdown(self) {
        if let Err(e) = self.tx_cmds.send(SlirpCmd::Shutdown) {
            warn!("Failed to send Shutdown cmd: {}", e);
        }
    }

    pub fn input(&self, bytes: Bytes) {
        if let Err(e) = self.tx_cmds.send(SlirpCmd::Input(bytes)) {
            warn!("Failed to send Input cmd: {}", e);
        }
    }
}

fn slirp_thread(
    config: libslirp_config::SlirpConfig,
    rx: mpsc::Receiver<SlirpCmd>,
    tx_poll: mpsc::Sender<PollRequest>,
) {
    let callbacks = libslirp_sys::SlirpCb {
        send_packet: Some(send_packet_cb),
        guest_error: Some(guest_error_cb),
        clock_get_ns: Some(clock_get_ns_cb),
        timer_new: None,
        timer_free: Some(timer_free_cb),
        timer_mod: Some(timer_mod_cb),
        register_poll_fd: Some(register_poll_fd_cb),
        unregister_poll_fd: Some(unregister_poll_fd_cb),
        notify: Some(notify_cb),
        init_completed: Some(init_completed),
        remove: None,
        timer_new_opaque: Some(timer_new_opaque_cb),
        try_connect: None,
    };
    let configs = SlirpConfigs::new(&config);
    // Call libslrip "C" library to create a new instance of a slirp
    // protocol stack.
    //
    // SAFETY: We ensure that:
    //
    // `config` is a valid pointer to the "C" config struct. It is
    // held by the "C" slirp library for lifetime of the slirp
    // instance.
    //
    // `callbacks` is a valid pointer to an array of callback
    // functions. It is held by the "C" slirp library for the lifetime
    // of the slirp instance.
    let slirp = unsafe {
        libslirp_sys::slirp_new(&configs.c_slirp_config, &callbacks, std::ptr::null_mut())
    };

    unsafe { slirp_pollfds_fill(slirp, &tx_poll) };

    let min_duration = TIMERS.lock().unwrap().min_duration();
    loop {
        match rx.recv_timeout(min_duration) {
            Ok(SlirpCmd::PollResult(poll_fds, select_error)) => {
                // SAFETY: we ensure that slirp is a valid state returned by `slirp_new()`
                unsafe { slirp_pollfds_poll(slirp, select_error, poll_fds) };
                unsafe { slirp_pollfds_fill(slirp, &tx_poll) };
            }
            // SAFETY: we ensure that slirp is a valid state returned by `slirp_new()`
            Ok(SlirpCmd::Input(bytes)) => unsafe { slirp_input(slirp, &bytes) },

            // A timer has been modified, new expired_time value
            Ok(SlirpCmd::TimerModified) => continue,

            // Exit the while loop and shutdown
            Ok(SlirpCmd::Shutdown) => break,

            // Timeout... process any timers
            Err(mpsc::RecvTimeoutError::Timeout) => continue,

            // Error
            _ => break,
        }
        // Callback any expired timers in the slirp thread...
        for timer in TIMERS.lock().unwrap().collect_expired() {
            unsafe {
                libslirp_sys::slirp_handle_timer(slirp, timer.id, timer.cb_opaque as *mut c_void)
            };
        }
    }
    // Shuts down the instance of a slirp stack and release slirp storage. No callbacks
    // occur after `slirp_cleanup` is called.

    // SAFETY: we ensure that slirp is a valid state returned by `slirp_new()`
    unsafe { libslirp_sys::slirp_cleanup(slirp) };
    // Shutdown slirp_poll_thread -- worst case it sends a PollResult that is ignored
    // since this thread is no longer processing Slirp commands.
    drop(tx_poll);

    // SAFETY: Slirp is shutdown. `slirp` `config` and `libslirp` can
    // be released.
}

struct PollFd {
    fd: c_int,
    events: libslirp_sys::SlirpPollType,
    revents: libslirp_sys::SlirpPollType,
}

// Fill the pollfds from libslirp and pass the request to the polling thread.
//
// This is called by the application when it is about to sleep through
// poll().  *timeout is set to the amount of virtual time (in ms) that
// the application intends to wait (UINT32_MAX if
// infinite). slirp_pollfds_fill updates it according to e.g. TCP
// timers, so the application knows it should sleep a smaller amount
// of time. slirp_pollfds_fill calls add_poll for each file descriptor
// that should be monitored along the sleep. The opaque pointer is
// passed as such to add_poll, and add_poll returns an index.
//
// # Safety
//
// `slirp` must be a valid Slirp state returned by `slirp_new()`
unsafe fn slirp_pollfds_fill(slirp: *mut libslirp_sys::Slirp, tx: &mpsc::Sender<PollRequest>) {
    let mut timeout: u32 = 0;
    CONTEXT.lock().unwrap().poll_fds.clear();

    // Call libslrip "C" library to fill poll information using
    // slirp_add_poll_cb callback function.
    //
    // SAFETY: we ensure that:
    //
    // `slirp` is a valid Slirp state.
    //
    // `timeout` is a valid ptr to a mutable u32.  The "C" slirp
    // library stores into timeout.
    //
    // `slirp_add_poll_cb` is a valid `SlirpAddPollCb` function.
    unsafe {
        libslirp_sys::slirp_pollfds_fill(
            slirp,
            &mut timeout,
            Some(slirp_add_poll_cb),
            std::ptr::null_mut(),
        );
    }
    let poll_fds: Vec<PollFd> = CONTEXT.lock().unwrap().poll_fds.drain(..).collect();
    debug!("got {} items", poll_fds.len());
    if let Err(e) = tx.send((poll_fds, timeout)) {
        warn!("Failed to send poll fds: {}", e);
    }
}

// "C" library callback that is called for each file descriptor that
// should be monitored.

extern "C" fn slirp_add_poll_cb(fd: c_int, events: c_int, _opaque: *mut c_void) -> c_int {
    let mut guard = CONTEXT.lock().unwrap();
    let idx = guard.poll_fds.len();
    guard.poll_fds.push(PollFd { fd, events: events as libslirp_sys::SlirpPollType, revents: 0 });
    idx as i32
}

// Pass the result from the polling thread back to libslirp

// This is called by the application when it is about to sleep through
// poll().  *timeout is set to the amount of virtual time (in ms) that
// the application intends to wait (UINT32_MAX if
// infinite). slirp_pollfds_fill updates it according to e.g. TCP
// timers, so the application knows it should sleep a smaller amount
// of time. slirp_pollfds_fill calls add_poll for each file descriptor
// that should be monitored along the sleep. The opaque pointer is
// passed as such to add_poll, and add_poll returns an index.
//
// # Safety
//
// `slirp` must be a valid Slirp state returned by `slirp_new()`
//
// 'select_error' should be 1 if poll() returned an error, else 0.
unsafe fn slirp_pollfds_poll(
    slirp: *mut libslirp_sys::Slirp,
    select_error: c_int,
    poll_fds: Vec<PollFd>,
) {
    CONTEXT.lock().unwrap().poll_fds = poll_fds;

    // Call libslrip "C" library to fill poll return event information
    // using slirp_get_revents_cb callback function.
    //
    // SAFETY: we ensure that:
    //
    // `slirp` is a valid Slirp state.
    //
    // `slirp_get_revents_cb` is a valid `SlirpGetREventsCb` callback
    // function.
    //
    // 'select_error' should be 1 if poll() returned an error, else 0.
    unsafe {
        libslirp_sys::slirp_pollfds_poll(
            slirp,
            select_error,
            Some(slirp_get_revents_cb),
            std::ptr::null_mut(),
        );
    }
}

// "C" library callback that is called on each file descriptor, giving
// it the index that add_poll returned.

extern "C" fn slirp_get_revents_cb(idx: c_int, _opaue: *mut c_void) -> c_int {
    if let Some(poll_fd) = CONTEXT.lock().unwrap().poll_fds.get(idx as usize) {
        return poll_fd.revents as c_int;
    }
    return 0;
}

macro_rules! ternary {
    ($cond:expr, $true_expr:expr) => {
        if $cond != 0 {
            $true_expr
        } else {
            0
        }
    };
}

// Loop issuing blocking poll requests, sending the results into the slirp thread

#[cfg(target_os = "macos")]
fn slirp_poll_thread(rx: mpsc::Receiver<PollRequest>, tx: mpsc::Sender<SlirpCmd>) {
    todo!();
}

#[cfg(target_os = "windows")]
fn slirp_poll_thread(rx: mpsc::Receiver<PollRequest>, tx: mpsc::Sender<SlirpCmd>) {
    todo!();
}

#[cfg(target_os = "linux")]
fn slirp_poll_thread(rx: mpsc::Receiver<PollRequest>, tx: mpsc::Sender<SlirpCmd>) {
    use libc::{poll, pollfd, POLLERR, POLLHUP, POLLIN, POLLOUT, POLLPRI};

    fn to_os_events(events: libslirp_sys::SlirpPollType) -> i16 {
        ternary!(events & libslirp_sys::SLIRP_POLL_IN, POLLIN)
            | ternary!(events & libslirp_sys::SLIRP_POLL_OUT, POLLOUT)
            | ternary!(events & libslirp_sys::SLIRP_POLL_PRI, POLLPRI)
            | ternary!(events & libslirp_sys::SLIRP_POLL_ERR, POLLERR)
            | ternary!(events & libslirp_sys::SLIRP_POLL_HUP, POLLHUP)
    }

    fn to_slirp_events(events: i16) -> libslirp_sys::SlirpPollType {
        ternary!(events & POLLIN, libslirp_sys::SLIRP_POLL_IN)
            | ternary!(events & POLLOUT, libslirp_sys::SLIRP_POLL_OUT)
            | ternary!(events & POLLPRI, libslirp_sys::SLIRP_POLL_PRI)
            | ternary!(events & POLLOUT, libslirp_sys::SLIRP_POLL_ERR)
            | ternary!(events & POLLHUP, libslirp_sys::SLIRP_POLL_HUP)
    }

    while let Ok((poll_fds, timeout)) = rx.recv() {
        // Create a c format array with the same size as poll
        let mut os_poll_fds: Vec<pollfd> = Vec::with_capacity(poll_fds.len());
        for fd in &poll_fds {
            os_poll_fds.push(pollfd { fd: fd.fd, events: to_os_events(fd.events), revents: 0 });
        }

        // SAFETY: we ensure that:
        //
        // `os_poll_fds` is a valid ptr to a vector of pollfd which
        // the `poll` system call can write into. Note `os_poll_fds`
        // is created and allocated above.
        let poll_result =
            unsafe { poll(os_poll_fds.as_mut_ptr(), os_poll_fds.len() as u64, timeout as i32) };

        let mut slirp_poll_fds: Vec<PollFd> = Vec::with_capacity(poll_fds.len());
        for &fd in &os_poll_fds {
            slirp_poll_fds.push(PollFd {
                fd: fd.fd,
                events: to_slirp_events(fd.events),
                revents: to_slirp_events(fd.revents) & to_slirp_events(fd.events),
            });
        }
        // 'select_error' should be 1 if poll() returned an error, else 0.
        if let Err(e) = tx.send(SlirpCmd::PollResult(slirp_poll_fds, (poll_result < 0) as i32)) {
            warn!("Failed to send slirp PollResult cmd: {}", e);
        }
    }
}

// Call libslrip "C" library to send input.
//
// This is called by the application when the guest emits a packet on
// the guest network, to be interpreted by slirp.
//
// # Safety
//
// `slirp` must be a valid Slirp state returned by `slirp_new()`
unsafe fn slirp_input(slirp: *mut libslirp_sys::Slirp, bytes: &[u8]) {
    // SAFETY: The "C" library ensure that the memory is not
    // referenced after the call and `bytes` does not need to remain
    // valid after the function returns.
    unsafe { libslirp_sys::slirp_input(slirp, bytes.as_ptr(), bytes.len() as i32) };
}

// "C" library callback that is called to send an ethernet frame to
// the guest network. If the guest is not ready to receive a frame,
// the function can just drop the data. TCP will then handle
// retransmissions at a lower pace.  A return of < 0 reports an IO
// error.
//
// # Safety:
//
// `buf` must be a valid pointer to `len` bytes of memory. The
// contents of buf must be valid for the duration of this call.
unsafe extern "C" fn send_packet_cb(
    buf: *const c_void,
    len: usize,
    _opaque: *mut c_void,
) -> libslirp_sys::slirp_ssize_t {
    // SAFETY: The caller ensures that `buf` is contains `len` bytes of data.
    let c_slice = unsafe { std::slice::from_raw_parts(buf as *const u8, len) };
    // Bytes::from(slice: &'static [u8]) creates a Bytes object without copying the data.
    // To own its data, copy &'static [u8] to Vec<u8> before converting to Bytes.
    CONTEXT
        .lock()
        .unwrap()
        .tx_bytes
        .as_ref()
        .map(|sender| sender.send(Bytes::from(c_slice.to_vec())));
    len as libslirp_sys::slirp_ssize_t
}

// "C" library callback to print a message for an error due to guest
// misbehavior.
//
// # Safety:
//
// `msg` must be a valid nul-terminated utf8 string.
unsafe extern "C" fn guest_error_cb(msg: *const c_char, _opaque: *mut c_void) {
    // SAFETY: The caller ensures that `msg` is a nul-terminated string.
    let msg = String::from_utf8_lossy(unsafe { CStr::from_ptr(msg) }.to_bytes());
    warn!("libslirp: {msg}");
}

extern "C" fn clock_get_ns_cb(_opaque: *mut c_void) -> i64 {
    TIMERS.lock().unwrap().clock.elapsed().as_nanos() as i64
}

extern "C" fn init_completed(_slirp: *mut libslirp_sys::Slirp, _opaque: *mut c_void) {
    info!("libslirp: initialization completed.");
}

// Create a new timer
extern "C" fn timer_new_opaque_cb(
    id: libslirp_sys::SlirpTimerId,
    cb_opaque: *mut c_void,
    _opaque: *mut c_void,
) -> *mut c_void {
    let mut guard = TIMERS.lock().unwrap();
    let timer = guard.next_timer();
    debug!("timer_new_opaque {timer}");
    guard.map.insert(timer, Timer { expire_time: u64::MAX, id, cb_opaque: cb_opaque as usize });
    timer as *mut c_void
}

extern "C" fn timer_free_cb(
    timer: *mut ::std::os::raw::c_void,
    _opaque: *mut ::std::os::raw::c_void,
) {
    let timer = timer as TimerOpaque;
    debug!("timer_free {timer}");
    if TIMERS.lock().unwrap().map.remove(&timer).is_none() {
        warn!("Unknown timer {timer}");
    }
}

extern "C" fn timer_mod_cb(
    timer: *mut ::std::os::raw::c_void,
    expire_time: i64,
    _opaque: *mut ::std::os::raw::c_void,
) {
    let timer_key = timer as TimerOpaque;
    let now_ms = TIMERS.lock().unwrap().clock.elapsed().as_millis() as u64;
    if let Some(&mut ref mut timer) = TIMERS.lock().unwrap().map.get_mut(&timer_key) {
        // expire_time is > 0
        timer.expire_time = std::cmp::max(expire_time, 0) as u64;
        debug!("timer_mod {timer_key} expire_time: {}ms", timer.expire_time.saturating_sub(now_ms));
    } else {
        warn!("Unknown timer {timer_key}");
    }
    // Wake up slirp command thread to reset sleep duration
    CONTEXT.lock().unwrap().tx_cmds.as_ref().map(|sender| sender.send(SlirpCmd::TimerModified));
}

extern "C" fn register_poll_fd_cb(
    _fd: ::std::os::raw::c_int,
    _opaque: *mut ::std::os::raw::c_void,
) {
    //TODO: Need implementation for Windows
}

extern "C" fn unregister_poll_fd_cb(
    _fd: ::std::os::raw::c_int,
    _opaque: *mut ::std::os::raw::c_void,
) {
    //TODO: Need implementation for Windows
}

extern "C" fn notify_cb(_opaque: *mut ::std::os::raw::c_void) {
    //TODO: Un-implemented
}
