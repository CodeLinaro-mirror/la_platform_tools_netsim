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
use crate::libslirp_sys;
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
use bytes::Bytes;
use core::sync::atomic::{AtomicUsize, Ordering};
use log::{debug, info, warn};
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::{c_char, c_int, c_void, CStr};
use std::mem::ManuallyDrop;
use std::net::SocketAddr;
use std::rc::Rc;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use std::time::Instant;

type TimerOpaque = usize;

struct TimerManager {
    clock: RefCell<Instant>,
    map: RefCell<HashMap<TimerOpaque, Timer>>,
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
    ProxyConnect(libslirp_sys::SlirpProxyConnectFunc, usize, i32, i32),
}

/// Alias for io::fd::RawFd on Unix or RawSocket on Windows (converted to i32)
pub type RawFd = i32;

// HTTP Proxy callback trait
pub trait ProxyManager: Send {
    fn try_connect(
        &self,
        sockaddr: SocketAddr,
        connect_id: usize,
        connect_func: Box<dyn ProxyConnect>,
    ) -> bool;
    fn remove(&self, connect_id: usize);
}

struct CallbackContext {
    tx_bytes: mpsc::Sender<Bytes>,
    tx_cmds: mpsc::Sender<SlirpCmd>,
    poll_fds: Rc<RefCell<Vec<PollFd>>>,
    proxy_manager: Option<Box<dyn ProxyManager>>,
    timer_manager: Rc<TimerManager>,
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
    fn collect_expired(&self) -> Vec<Timer> {
        let now_ms = self.get_elapsed().as_millis() as u64;
        self.map
            .borrow_mut()
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
        match self.map.borrow().iter().min_by_key(|(_, timer)| timer.expire_time) {
            Some((_, timer)) => {
                let now_ms = self.get_elapsed().as_millis() as u64;
                // Duration is >= 0
                Duration::from_millis(timer.expire_time.saturating_sub(now_ms))
            }
            None => Duration::from_millis(u64::MAX),
        }
    }

    fn get_elapsed(&self) -> Duration {
        self.clock.borrow().elapsed()
    }

    fn remove(&self, timer_key: &TimerOpaque) -> Option<Timer> {
        self.map.borrow_mut().remove(timer_key)
    }

    fn insert(&self, timer_key: TimerOpaque, value: Timer) {
        self.map.borrow_mut().insert(timer_key, value);
    }

    fn timer_mod(&self, timer_key: &TimerOpaque, expire_time: u64) {
        if let Some(&mut ref mut timer) = self.map.borrow_mut().get_mut(&timer_key) {
            // expire_time is >= 0
            timer.expire_time = expire_time;
        } else {
            warn!("Unknown timer {timer_key}");
        }
    }
}

impl LibSlirp {
    pub fn new(
        config: libslirp_config::SlirpConfig,
        tx_bytes: mpsc::Sender<Bytes>,
        proxy_manager: Option<Box<dyn ProxyManager>>,
    ) -> LibSlirp {
        let (tx_cmds, rx_cmds) = mpsc::channel::<SlirpCmd>();
        let (tx_poll, rx_poll) = mpsc::channel::<PollRequest>();

        // Create channels for polling thread and launch
        let tx_cmds_poll = tx_cmds.clone();
        if let Err(e) = thread::Builder::new()
            .name("slirp_poll".to_string())
            .spawn(move || slirp_poll_thread(rx_poll, tx_cmds_poll))
        {
            warn!("Failed to start slirp poll thread: {}", e);
        }

        let tx_cmds_slirp = tx_cmds.clone();
        // Create channels for command processor thread and launch
        if let Err(e) = thread::Builder::new().name("slirp".to_string()).spawn(move || {
            slirp_thread(config, tx_bytes, tx_cmds_slirp, rx_cmds, tx_poll, proxy_manager)
        }) {
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

struct ConnectRequest {
    tx_cmds: mpsc::Sender<SlirpCmd>,
    connect_func: libslirp_sys::SlirpProxyConnectFunc,
    connect_id: usize,
    af: i32,
}

pub trait ProxyConnect {
    fn proxy_connect(&self, fd: i32);
}

impl ProxyConnect for ConnectRequest {
    fn proxy_connect(&self, fd: i32) {
        // Send it to Slirp after try_connect() completed
        let _ = self.tx_cmds.send(SlirpCmd::ProxyConnect(
            self.connect_func,
            self.connect_id,
            fd,
            self.af,
        ));
    }
}

// Converts a libslirp callback's `opaque` handle into a
// `CallbackContext.`
//
// Wrapped in a `ManuallyDrop` because we do not want to release the
// storage when the callback returns.
//
// SAFETY:
//
// * opaque is a CallbackContext passed to the slirp API
unsafe fn callback_context_from_raw(opaque: *mut c_void) -> ManuallyDrop<Box<CallbackContext>> {
    ManuallyDrop::new(unsafe { Box::from_raw(opaque as *mut CallbackContext) })
}

// A Rust struct for the fields held by `slirp` C library through it's
// lifetime.
//
// All libslirp C calls are impl on this struct.
struct Slirp {
    slirp: *mut libslirp_sys::Slirp,
    // These fields are held by slirp C library
    #[allow(dead_code)]
    configs: Box<SlirpConfigs>,
    #[allow(dead_code)]
    callbacks: Box<libslirp_sys::SlirpCb>,
    // Passed to API calls and then to callbacks
    callback_context: Box<CallbackContext>,
}

impl Slirp {
    fn new(config: libslirp_config::SlirpConfig, callback_context: Box<CallbackContext>) -> Slirp {
        let callbacks = Box::new(libslirp_sys::SlirpCb {
            send_packet: Some(send_packet_cb),
            guest_error: Some(guest_error_cb),
            clock_get_ns: Some(clock_get_ns_cb),
            timer_new: None,
            timer_free: Some(timer_free_cb),
            timer_mod: Some(timer_mod_cb),
            register_poll_fd: Some(register_poll_fd_cb),
            unregister_poll_fd: Some(unregister_poll_fd_cb),
            notify: Some(notify_cb),
            init_completed: Some(init_completed_cb),
            timer_new_opaque: Some(timer_new_opaque_cb),
            try_connect: Some(try_connect_cb),
            remove: Some(remove_cb),
        });
        let configs = Box::new(SlirpConfigs::new(&config));

        // Call libslrip "C" library to create a new instance of a slirp
        // protocol stack.
        //
        // SAFETY: We ensure that:
        //
        // * config is a valid pointer to the "C" config struct. It is
        // held by the "C" slirp library for lifetime of the slirp
        // instance.
        //
        // * callbacks is a valid pointer to an array of callback
        // functions. It is held by the "C" slirp library for the lifetime
        // of the slirp instance.
        //
        // * callback_context is an arbitrary opaque type passed back
        //  to callback functions by libslirp.
        let slirp = unsafe {
            libslirp_sys::slirp_new(
                &configs.c_slirp_config,
                &*callbacks,
                &*callback_context as *const CallbackContext as *mut c_void,
            )
        };

        Slirp { slirp, configs, callbacks, callback_context }
    }

    fn handle_timer(&self, timer: Timer) {
        unsafe {
            //
            // SAFETY: We ensure that:
            //
            // *self.slirp is a valid state returned by `slirp_new()`
            //
            // * timer.id is a valid c_uint from "C" slirp library calling `timer_new_opaque_cb()`
            //
            // * timer.cb_opaque is an usize representing a pointer to callback function from
            // "C" slirp library calling `timer_new_opaque_cb()`
            libslirp_sys::slirp_handle_timer(self.slirp, timer.id, timer.cb_opaque as *mut c_void);
        };
    }
}

impl Drop for Slirp {
    fn drop(&mut self) {
        // SAFETY:
        //
        // * self.slirp is a slirp pointer initialized by slirp_new;
        // it's private to the struct and is only constructed that
        // way.
        unsafe { libslirp_sys::slirp_cleanup(self.slirp) };
    }
}

fn slirp_thread(
    config: libslirp_config::SlirpConfig,
    tx_bytes: mpsc::Sender<Bytes>,
    tx_cmds: mpsc::Sender<SlirpCmd>,
    rx: mpsc::Receiver<SlirpCmd>,
    tx_poll: mpsc::Sender<PollRequest>,
    proxy_manager: Option<Box<dyn ProxyManager>>,
) {
    // Data structures wrapped in an RC are referenced through the
    // libslirp callbacks and this code (both in the same thread).

    let timer_manager = Rc::new(TimerManager {
        clock: RefCell::new(Instant::now()),
        map: RefCell::new(HashMap::new()),
        timers: AtomicUsize::new(1),
    });

    let poll_fds = Rc::new(RefCell::new(Vec::new()));

    let callback_context = Box::new(CallbackContext {
        tx_bytes,
        tx_cmds,
        poll_fds: poll_fds.clone(),
        proxy_manager,
        timer_manager: timer_manager.clone(),
    });

    let slirp = Slirp::new(config, callback_context);

    slirp.pollfds_fill_and_send(&poll_fds, &tx_poll);

    let min_duration = timer_manager.min_duration();
    loop {
        match rx.recv_timeout(min_duration) {
            // The dance to tell libslirp which FDs have IO ready
            // starts with a response from a worker thread sending a
            // PollResult, followed by pollfds_poll forwarding the FDs
            // to libslirp, followed by giving the worker thread
            // another set of fds to poll (and block).
            Ok(SlirpCmd::PollResult(poll_fds_result, select_error)) => {
                poll_fds.borrow_mut().clone_from_slice(&poll_fds_result);
                slirp.pollfds_poll(select_error);
                slirp.pollfds_fill_and_send(&poll_fds, &tx_poll);
            }
            Ok(SlirpCmd::Input(bytes)) => slirp.input(&bytes),

            // A timer has been modified, new expired_time value
            Ok(SlirpCmd::TimerModified) => continue,

            // Exit the while loop and shutdown
            Ok(SlirpCmd::Shutdown) => break,

            // SAFETY: we ensure that func (`SlirpProxyConnectFunc`)
            // and `connect_opaque` are valid because they originated
            // from the libslirp call to `try_connect_cb.`
            //
            // Parameter `fd` will be >= 0 and the descriptor for the
            // active socket to use, `af` will be either AF_INET or
            // AF_INET6. On failure `fd` will be negative.
            Ok(SlirpCmd::ProxyConnect(func, connect_id, fd, af)) => match func {
                Some(func) => unsafe { func(connect_id as *mut c_void, fd as c_int, af as c_int) },
                None => warn!("Proxy connect function not found"),
            },

            // Timeout... process any timers
            Err(mpsc::RecvTimeoutError::Timeout) => continue,

            // Error
            _ => break,
        }

        // Explicitly store expired timers to release lock
        let timers = timer_manager.collect_expired();
        // Handle any expired timers' callback in the slirp thread
        for timer in timers {
            slirp.handle_timer(timer);
        }
    }
    // Shuts down the instance of a slirp stack and release slirp storage. No callbacks
    // occur after this since it calls slirp_cleanup.
    drop(slirp);

    // Shutdown slirp_poll_thread -- worst case it sends a PollResult that is ignored
    // since this thread is no longer processing Slirp commands.
    drop(tx_poll);
}

#[derive(Clone, Debug)]
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
impl Slirp {
    fn pollfds_fill_and_send(
        &self,
        poll_fds: &RefCell<Vec<PollFd>>,
        tx: &mpsc::Sender<PollRequest>,
    ) {
        let mut timeout: u32 = u32::MAX;
        poll_fds.borrow_mut().clear();

        // Call libslrip "C" library to fill poll information using
        // slirp_add_poll_cb callback function.
        //
        // SAFETY: we ensure that:
        //
        // * self.slirp has a slirp pointer initialized by slirp_new,
        // as it's private to the struct is only constructed that way
        //
        // * timeout is a valid ptr to a mutable u32.  The "C" slirp
        // library stores into timeout.
        //
        // * slirp_add_poll_cb is a valid `SlirpAddPollCb` function.
        //
        // * self.callback_context is a CallbackContext
        unsafe {
            libslirp_sys::slirp_pollfds_fill(
                self.slirp,
                &mut timeout,
                Some(slirp_add_poll_cb),
                &*self.callback_context as *const CallbackContext as *mut c_void,
            );
        }
        if let Err(e) = tx.send((poll_fds.borrow().to_vec(), timeout)) {
            warn!("Failed to send poll fds: {}", e);
        }
    }
}

// "C" library callback that is called for each file descriptor that
// should be monitored.
//
// SAFETY:
//
// * opaque is a CallbackContext
unsafe extern "C" fn slirp_add_poll_cb(fd: c_int, events: c_int, opaque: *mut c_void) -> c_int {
    unsafe { callback_context_from_raw(opaque) }.add_poll(fd, events)
}

impl CallbackContext {
    fn add_poll(&mut self, fd: c_int, events: c_int) -> c_int {
        let idx = self.poll_fds.borrow().len();
        self.poll_fds.borrow_mut().push(PollFd {
            fd,
            events: events as libslirp_sys::SlirpPollType,
            revents: 0,
        });
        idx as i32
    }
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
// * select_error should be 1 if poll() returned an error, else 0.

impl Slirp {
    fn pollfds_poll(&self, select_error: c_int) {
        // Call libslrip "C" library to fill poll return event information
        // using slirp_get_revents_cb callback function.
        //
        // SAFETY: we ensure that:
        //
        // * self.slirp has a slirp pointer initialized by slirp_new,
        // as it's private to the struct is only constructed that way
        //
        // * slirp_get_revents_cb is a valid `SlirpGetREventsCb` callback
        // function.
        //
        // * select_error should be 1 if poll() returned an error, else 0.
        //
        // * self.callback_context is a CallbackContext
        unsafe {
            libslirp_sys::slirp_pollfds_poll(
                self.slirp,
                select_error,
                Some(slirp_get_revents_cb),
                &*self.callback_context as *const CallbackContext as *mut c_void,
            );
        }
    }
}

// "C" library callback that is called on each file descriptor, giving
// it the index that add_poll returned.
//
// SAFETY:
//
// * opaque is a CallbackContext
unsafe extern "C" fn slirp_get_revents_cb(idx: c_int, opaque: *mut c_void) -> c_int {
    unsafe { callback_context_from_raw(opaque) }.get_events(idx)
}

impl CallbackContext {
    fn get_events(&self, idx: c_int) -> c_int {
        if let Some(poll_fd) = self.poll_fds.borrow().get(idx as usize) {
            poll_fd.revents as c_int
        } else {
            0
        }
    }
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

// Worker thread loops issuing blocking poll requests, sending the
// results into the slirp thread

fn slirp_poll_thread(rx: mpsc::Receiver<PollRequest>, tx: mpsc::Sender<SlirpCmd>) {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    use libc::{
        nfds_t as OsPollFdsLenType, poll, pollfd, POLLERR, POLLHUP, POLLIN, POLLOUT, POLLPRI,
    };
    #[cfg(target_os = "windows")]
    use winapi::{
        shared::minwindef::ULONG as OsPollFdsLenType,
        um::winsock2::{
            WSAPoll as poll, POLLERR, POLLHUP, POLLOUT, POLLPRI, POLLRDBAND, POLLRDNORM,
            SOCKET as FdType, WSAPOLLFD as pollfd,
        },
    };
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    type FdType = c_int;

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    fn to_os_events(events: libslirp_sys::SlirpPollType) -> i16 {
        ternary!(events & libslirp_sys::SLIRP_POLL_IN, POLLIN)
            | ternary!(events & libslirp_sys::SLIRP_POLL_OUT, POLLOUT)
            | ternary!(events & libslirp_sys::SLIRP_POLL_PRI, POLLPRI)
            | ternary!(events & libslirp_sys::SLIRP_POLL_ERR, POLLERR)
            | ternary!(events & libslirp_sys::SLIRP_POLL_HUP, POLLHUP)
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    fn to_slirp_events(events: i16) -> libslirp_sys::SlirpPollType {
        ternary!(events & POLLIN, libslirp_sys::SLIRP_POLL_IN)
            | ternary!(events & POLLOUT, libslirp_sys::SLIRP_POLL_OUT)
            | ternary!(events & POLLPRI, libslirp_sys::SLIRP_POLL_PRI)
            | ternary!(events & POLLOUT, libslirp_sys::SLIRP_POLL_ERR)
            | ternary!(events & POLLHUP, libslirp_sys::SLIRP_POLL_HUP)
    }

    #[cfg(target_os = "windows")]
    fn to_os_events(events: libslirp_sys::SlirpPollType) -> i16 {
        ternary!(events & libslirp_sys::SLIRP_POLL_IN, POLLRDNORM)
            | ternary!(events & libslirp_sys::SLIRP_POLL_OUT, POLLOUT)
            | ternary!(events & libslirp_sys::SLIRP_POLL_PRI, POLLRDBAND)
    }

    #[cfg(target_os = "windows")]
    fn to_slirp_events(events: i16) -> libslirp_sys::SlirpPollType {
        ternary!(events & POLLRDNORM, libslirp_sys::SLIRP_POLL_IN)
            | ternary!(events & POLLERR, libslirp_sys::SLIRP_POLL_IN)
            | ternary!(events & POLLHUP, libslirp_sys::SLIRP_POLL_IN)
            | ternary!(events & POLLOUT, libslirp_sys::SLIRP_POLL_OUT)
            | ternary!(events & POLLERR, libslirp_sys::SLIRP_POLL_PRI)
            | ternary!(events & POLLHUP, libslirp_sys::SLIRP_POLL_PRI)
            | ternary!(events & POLLPRI, libslirp_sys::SLIRP_POLL_PRI)
            | ternary!(events & POLLRDBAND, libslirp_sys::SLIRP_POLL_PRI)
    }

    let mut prev_poll_fds_len = 0;
    while let Ok((poll_fds, timeout)) = rx.recv() {
        if poll_fds.len() != prev_poll_fds_len {
            prev_poll_fds_len = poll_fds.len();
            debug!("slirp_poll_thread recv poll_fds.len(): {:?}", prev_poll_fds_len);
        }
        // Create a c format array with the same size as poll
        let mut os_poll_fds: Vec<pollfd> = Vec::with_capacity(poll_fds.len());
        for fd in &poll_fds {
            os_poll_fds.push(pollfd {
                fd: fd.fd as FdType,
                events: to_os_events(fd.events),
                revents: 0,
            });
        }

        // SAFETY: we ensure that:
        //
        // `os_poll_fds` is a valid ptr to a vector of pollfd which
        // the `poll` system call can write into. Note `os_poll_fds`
        // is created and allocated above.
        let poll_result = unsafe {
            poll(os_poll_fds.as_mut_ptr(), os_poll_fds.len() as OsPollFdsLenType, timeout as i32)
        };

        let mut slirp_poll_fds: Vec<PollFd> = Vec::with_capacity(poll_fds.len());
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        for &fd in &os_poll_fds {
            slirp_poll_fds.push(PollFd {
                fd: fd.fd as c_int,
                events: to_slirp_events(fd.events),
                revents: to_slirp_events(fd.revents) & to_slirp_events(fd.events),
            });
        }
        #[cfg(target_os = "windows")]
        for (fd, poll_fd) in os_poll_fds.iter().zip(poll_fds.iter()) {
            slirp_poll_fds.push(PollFd {
                fd: fd.fd as c_int,
                events: poll_fd.events,
                revents: to_slirp_events(fd.revents) & poll_fd.events,
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
impl Slirp {
    fn input(&self, bytes: &[u8]) {
        // SAFETY: The "C" library ensure that the memory is not
        // referenced after the call and `bytes` does not need to remain
        // valid after the function returns.
        unsafe { libslirp_sys::slirp_input(self.slirp, bytes.as_ptr(), bytes.len() as i32) };
    }
}

// "C" library callback that is called to send an ethernet frame to
// the guest network. If the guest is not ready to receive a frame,
// the function can just drop the data. TCP will then handle
// retransmissions at a lower pace.  A return of < 0 reports an IO
// error.
//
// # Safety:
//
// * buf must be a valid pointer to `len` bytes of memory. The
// contents of buf must be valid for the duration of this call.
//
// * len is > 0
//
// * opaque is a CallbackContext
unsafe extern "C" fn send_packet_cb(
    buf: *const c_void,
    len: usize,
    opaque: *mut c_void,
) -> libslirp_sys::slirp_ssize_t {
    unsafe { callback_context_from_raw(opaque) }.send_packet(buf, len)
}

impl CallbackContext {
    fn send_packet(&self, buf: *const c_void, len: usize) -> libslirp_sys::slirp_ssize_t {
        // SAFETY: The caller ensures that `buf` is contains `len` bytes of data.
        let c_slice = unsafe { std::slice::from_raw_parts(buf as *const u8, len) };
        // Bytes::from(slice: &'static [u8]) creates a Bytes object without copying the data.
        // To own its data, copy &'static [u8] to Vec<u8> before converting to Bytes.
        let _ = self.tx_bytes.send(Bytes::from(c_slice.to_vec()));
        len as libslirp_sys::slirp_ssize_t
    }
}

// "C" library callback to print a message for an error due to guest
// misbehavior.
//
// # Safety:
//
// * msg must be a valid nul-terminated utf8 string.
//
// * opaque is a CallbackContext
unsafe extern "C" fn guest_error_cb(msg: *const c_char, opaque: *mut c_void) {
    // SAFETY: The caller ensures that `msg` is a nul-terminated string.
    let msg = String::from_utf8_lossy(unsafe { CStr::from_ptr(msg) }.to_bytes());
    unsafe { callback_context_from_raw(opaque) }.guest_error(msg.to_string());
}

impl CallbackContext {
    fn guest_error(&self, msg: String) {
        warn!("libslirp: {msg}");
    }
}

// SAFETY:
//
// * opaque is a CallbackContext
unsafe extern "C" fn clock_get_ns_cb(opaque: *mut c_void) -> i64 {
    unsafe { callback_context_from_raw(opaque) }.clock_get_ns()
}

impl CallbackContext {
    fn clock_get_ns(&self) -> i64 {
        self.timer_manager.get_elapsed().as_nanos() as i64
    }
}

// SAFETY:
//
// * opaque is a CallbackContext
unsafe extern "C" fn init_completed_cb(_slirp: *mut libslirp_sys::Slirp, opaque: *mut c_void) {
    unsafe { callback_context_from_raw(opaque) }.init_completed();
}

impl CallbackContext {
    fn init_completed(&self) {
        info!("libslirp: initialization completed.");
    }
}

// Create a new timer
//
// SAFETY:
//
// * opaque is a CallbackContext
unsafe extern "C" fn timer_new_opaque_cb(
    id: libslirp_sys::SlirpTimerId,
    cb_opaque: *mut c_void,
    opaque: *mut c_void,
) -> *mut c_void {
    unsafe { callback_context_from_raw(opaque) }.timer_new_opaque(id, cb_opaque)
}

impl CallbackContext {
    // SAFETY:
    //
    // * cb_opaque is only passed back to libslirp
    unsafe fn timer_new_opaque(
        &self,
        id: libslirp_sys::SlirpTimerId,
        cb_opaque: *mut c_void,
    ) -> *mut c_void {
        let timer = self.timer_manager.next_timer();
        self.timer_manager
            .insert(timer, Timer { expire_time: u64::MAX, id, cb_opaque: cb_opaque as usize });
        timer as *mut c_void
    }
}

// SAFETY:
//
// * timer is a TimerOpaque key for timer manager
//
// * opaque is a CallbackContext
unsafe extern "C" fn timer_free_cb(timer: *mut c_void, opaque: *mut c_void) {
    unsafe { callback_context_from_raw(opaque) }.timer_free(timer);
}

impl CallbackContext {
    fn timer_free(&self, timer: *mut c_void) {
        let timer = timer as TimerOpaque;
        if self.timer_manager.remove(&timer).is_none() {
            warn!("Unknown timer {timer}");
        }
    }
}

// SAFETY:
//
// * timer is a TimerOpaque key for timer manager
//
// * opaque is a CallbackContext
unsafe extern "C" fn timer_mod_cb(timer: *mut c_void, expire_time: i64, opaque: *mut c_void) {
    unsafe { callback_context_from_raw(opaque) }.timer_mod(timer, expire_time);
}

impl CallbackContext {
    fn timer_mod(&self, timer: *mut c_void, expire_time: i64) {
        let timer_key = timer as TimerOpaque;
        let expire_time = std::cmp::max(expire_time, 0) as u64;
        self.timer_manager.timer_mod(&timer_key, expire_time);
        // Wake up slirp command thread to reset sleep duration
        let _ = self.tx_cmds.send(SlirpCmd::TimerModified);
    }
}

extern "C" fn register_poll_fd_cb(_fd: c_int, _opaque: *mut c_void) {
    //TODO: Need implementation for Windows
}

extern "C" fn unregister_poll_fd_cb(_fd: c_int, _opaque: *mut c_void) {
    //TODO: Need implementation for Windows
}

extern "C" fn notify_cb(_opaque: *mut c_void) {
    //TODO: Un-implemented
}

// Called by libslirp to initiate a proxy connection to address
// `addr.` Eventually this will notify libslirp with a result by
// calling the passed `connect_func.`
//
// SAFETY:
//
// * opaque is a CallbackContext
unsafe extern "C" fn try_connect_cb(
    addr: *const libslirp_sys::sockaddr_storage,
    connect_func: libslirp_sys::SlirpProxyConnectFunc,
    connect_opaque: *mut c_void,
    opaque: *mut c_void,
) -> bool {
    unsafe { callback_context_from_raw(opaque) }.try_connect(
        addr,
        connect_func,
        connect_opaque as usize,
    )
}

impl CallbackContext {
    fn try_connect(
        &self,
        addr: *const libslirp_sys::sockaddr_storage,
        connect_func: libslirp_sys::SlirpProxyConnectFunc,
        connect_id: usize,
    ) -> bool {
        if let Some(proxy_connector) = &self.proxy_manager {
            // SAFETY: We ensure that addr is valid when `try_connect` is called from libslirp
            let storage = unsafe { *addr };
            let af = storage.ss_family as i32;
            let socket_addr: SocketAddr = storage.into();
            proxy_connector.try_connect(
                socket_addr,
                connect_id,
                Box::new(ConnectRequest {
                    tx_cmds: self.tx_cmds.clone(),
                    connect_func,
                    connect_id,
                    af,
                }),
            )
        } else {
            false
        }
    }
}

unsafe extern "C" fn remove_cb(connect_opaque: *mut c_void, opaque: *mut c_void) {
    unsafe { callback_context_from_raw(opaque) }.remove(connect_opaque as usize);
}

impl CallbackContext {
    fn remove(&self, connect_id: usize) {
        if let Some(proxy_connector) = &self.proxy_manager {
            proxy_connector.remove(connect_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_string() {
        // Safety
        // Function returns a constant c_str
        let c_version_str = unsafe { CStr::from_ptr(crate::libslirp_sys::slirp_version_string()) };
        assert_eq!("4.7.0", c_version_str.to_str().unwrap());
    }
}
