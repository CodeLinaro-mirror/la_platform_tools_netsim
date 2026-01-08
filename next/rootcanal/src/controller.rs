// Copyright 2025 The Android Open Source Project

//! This module defines the `Controller` struct, which represents a Bluetooth
//! controller.

/// A unique ID for the Controller
pub type Id = u32;

use crate::{
    ffi,
    types::{Address, Phy},
};
use bytes::{BufMut, Bytes, BytesMut};
use std::ffi::{c_int, c_void};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Mutex, Weak,
};

/// Callbacks for the Bluetooth controller.
pub trait Callbacks: Send + Sync {
    /// Request from rootcanal to send an HCI packet to the host.
    fn send_hci(&self, source_id: Id, h4_packet: Bytes);

    /// Called when the controller is receiving a packet
    fn on_receive_ll(&self, sender_id: Id, packet: &[u8], phy: Phy, rssi: i32);

    /// Called when an invalid packet is received.
    fn invalid_packet_received(&self, source_id: Id, reason: c_int, message: &str, data: &[u8]);
}

/// Trait used by the controller to call the Bluetooth manager.
pub trait BtOps: Send + Sync {
    /// Controller received a LL packet from rootcanal that needs to be broadcasted
    fn broadcast_rootcanal_ll_packet(&self, send_id: Id, packet: &[u8], phy: Phy, tx_power: i32);
}

// A wrapper around the raw C++ controller pointer.
struct FfiController(*mut c_void);

// SAFETY: This wrapper is only ever accessed through a Mutex in ControllerImpl,
// which guarantees exclusive access and makes it safe to send across threads.
unsafe impl Send for FfiController {}

/// The internal state of a Bluetooth controller.
pub(crate) struct ControllerImpl {
    controller: Mutex<FfiController>,
    id: Id,
    address: Address,
    pub(crate) callbacks: Box<dyn Callbacks>,
    bt_ops: Box<dyn BtOps>,
    hci_commands_in: AtomicU64,
    hci_events_out: AtomicU64,
    invalid_packets: AtomicU64,
    ll_packets_in: AtomicU64,
    ll_packets_out: AtomicU64,
    ll_packets_dropped: AtomicU64,
}

/// A Bluetooth controller.
pub(crate) type Controller = Arc<ControllerImpl>;

/// A collection of packet statistics.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Stats {
    /// The number of HCI packets received from the host.
    pub hci_commands_in: u64,
    /// The number of HCI packets sent to the host.
    pub hci_events_out: u64,
    /// The number of invalid packets received.
    pub invalid_packets: u64,
    /// The number of link layer packets received from the peer.
    pub ll_packets_in: u64,
    /// The number of link layer packets sent to the peer.
    pub ll_packets_out: u64,
    /// The number of link layer packets dropped.
    pub ll_packets_dropped: u64,
}

// The context that is passed to the C++ code.
type CallbackContext = Weak<ControllerImpl>;

impl ControllerImpl {
    pub(crate) fn new(
        id: Id,
        address: Address,
        callbacks: Box<dyn Callbacks>,
        bt_ops: Box<dyn BtOps>,
    ) -> Controller {
        // The initialization process is carefully ordered to manage lifetimes
        // across the FFI boundary and avoid memory leaks or reference cycles.
        // 1. A placeholder `ControllerImpl` is created with a null pointer.
        // 2. A `CallbackContext` is created with a `Weak` pointer back to the
        //    `ControllerImpl` to prevent ownership cycles.
        // 3. The `CallbackContext` is passed to the C++ FFI, which takes ownership.
        // 4. The FFI returns a real C++ controller pointer.
        // 5. The real pointer is stored in the `ControllerImpl`, replacing the null.
        // The C++ side is responsible for eventually calling `ffi_controller_delete`,
        // which frees the C++ controller and drops the `CallbackContext`.

        // Create the ControllerImpl with a placeholder (null) controller pointer
        // inside the Mutex.
        let controller_impl = Arc::new(ControllerImpl {
            controller: Mutex::new(FfiController(std::ptr::null_mut())),
            id,
            address,
            callbacks,
            bt_ops,
            hci_commands_in: AtomicU64::new(0),
            hci_events_out: AtomicU64::new(0),
            invalid_packets: AtomicU64::new(0),
            ll_packets_in: AtomicU64::new(0),
            ll_packets_out: AtomicU64::new(0),
            ll_packets_dropped: AtomicU64::new(0),
        });

        // Create the context for the C++ side, using a Weak pointer to avoid cycles.
        let context = Box::new(Arc::downgrade(&controller_impl));
        let context_ptr = Box::into_raw(context);

        // Call the FFI to get the real controller pointer.
        let controller_ptr =
            // SAFETY: The `address` pointer is valid for the duration of this call.
            // The C++ side is expected to copy the address data, not store the pointer.
            // We transfer ownership of `context_ptr` to the C++ library. The C++
            // library is responsible for calling `ffi_controller_delete`, which
            // will eventually drop the `CallbackContext` and its contents.
            unsafe {
                ffi::ffi_controller_new(
                    address.as_bytes().as_ptr(),
                    Some(send_hci_trampoline),
                    Some(send_ll_trampoline),
                    Some(invalid_packet_trampoline),
                    None,
                    context_ptr as *mut c_void,
                )
            };

        // Lock the mutex and store the real controller pointer. This is safe
        // because no other thread can have access to the `controller_impl` yet.
        {
            let mut controller_guard = controller_impl.controller.lock().unwrap();
            *controller_guard = FfiController(controller_ptr);
        }

        controller_impl
    }

    /// Receives an HCI packet from the host.
    pub(crate) fn receive_hci(&self, data: Bytes) {
        self.hci_commands_in.fetch_add(1, Ordering::Relaxed);
        let controller = self.controller.lock().unwrap();
        // SAFETY: The `controller.0` pointer is guaranteed to be valid
        // as long as `self` exists. `data` is a valid slice, and we provide its
        // length to ensure the C++ side does not read out of bounds.
        let idc = data[0] as c_int;
        let data: &[u8] = &data[1..];
        unsafe {
            ffi::ffi_controller_receive_hci(
                controller.0,
                idc,
                data.as_ptr(),
                data.len() as ffi::size_t,
            )
        };
    }

    /// Receives a link layer packet from a peer.
    pub(crate) fn receive_ll(&self, data: &[u8], phy: Phy, rssi: i32) {
        self.ll_packets_in.fetch_add(1, Ordering::Relaxed);
        let controller = self.controller.lock().unwrap();
        // SAFETY: The `controller.0` pointer is guaranteed to be valid
        // as long as `self` exists. `data` is a valid slice, and we provide its
        // length to ensure the C++ side does not read out of bounds.
        unsafe {
            ffi::ffi_controller_receive_ll(
                controller.0,
                data.as_ptr(),
                data.len() as ffi::size_t,
                phy as c_int,
                rssi,
            )
        };
    }

    /// Advances the controller's state by one tick.
    pub(crate) fn tick(&self) {
        let controller = self.controller.lock().unwrap();
        // SAFETY: The `controller.0` pointer is guaranteed to be valid
        // as long as `self` exists, as its lifetime is tied to the `ControllerImpl`.
        unsafe { ffi::ffi_controller_tick(controller.0) };
    }

    /// Returns the controller's address.
    pub(crate) fn get_address(&self) -> Address {
        self.address
    }

    /// Returns the controller's unique id.
    pub(crate) fn get_id(&self) -> Id {
        self.id
    }

    /// Returns the current packet statistics.
    pub(crate) fn get_stats(&self) -> Stats {
        Stats {
            hci_commands_in: self.hci_commands_in.load(Ordering::Relaxed),
            hci_events_out: self.hci_events_out.load(Ordering::Relaxed),
            invalid_packets: self.invalid_packets.load(Ordering::Relaxed),
            ll_packets_in: self.ll_packets_in.load(Ordering::Relaxed),
            ll_packets_out: self.ll_packets_out.load(Ordering::Relaxed),
            ll_packets_dropped: self.ll_packets_dropped.load(Ordering::Relaxed),
        }
    }

    /// Increments the number of link layer packets dropped.
    pub(crate) fn increment_ll_packets_dropped(&self) {
        self.ll_packets_dropped.fetch_add(1, Ordering::Relaxed);
    }

    /// Clears the packet statistics.
    pub(crate) fn clear_stats(&self) {
        self.hci_commands_in.store(0, Ordering::Relaxed);
        self.hci_events_out.store(0, Ordering::Relaxed);
        self.invalid_packets.store(0, Ordering::Relaxed);
        self.ll_packets_in.store(0, Ordering::Relaxed);
        self.ll_packets_out.store(0, Ordering::Relaxed);
        self.ll_packets_dropped.store(0, Ordering::Relaxed);
    }
}

impl Drop for ControllerImpl {
    fn drop(&mut self) {
        let controller = self.controller.lock().unwrap();
        // SAFETY: This is called when the last `Arc<ControllerImpl>` is dropped,
        // ensuring the C++ object is deleted exactly once. The `controller.0`
        // pointer is valid at this point.
        unsafe { ffi::ffi_controller_delete(controller.0) };
    }
}

// A helper function to get the context back from the cookie.
unsafe fn context_from_cookie<'a>(cookie: *mut c_void) -> &'a mut CallbackContext {
    // SAFETY: The `cookie` is an opaque pointer passed to us from the C++ side.
    // The FFI contract guarantees that this is the same pointer we provided
    // during `ffi_controller_new` and that it points to a valid `CallbackContext`.
    // The lifetime of the returned reference is tied to the scope of the calling
    // trampoline function, which is safe.
    unsafe { &mut *(cookie as *mut CallbackContext) }
}

// The trampoline function that is called by the C++ code.
extern "C" fn send_hci_trampoline(
    cookie: *mut c_void,
    idc: c_int,
    data: *const u8,
    data_len: ffi::size_t,
) {
    // SAFETY: The FFI contract guarantees that the C++ side provides a valid `cookie`.
    let context = unsafe { context_from_cookie(cookie) };

    if let Some(controller) = context.upgrade() {
        // SAFETY: The FFI contract guarantees that `data` points to a buffer of at
        // least `data_len` bytes that is valid for the duration of this call.
        let data_slice = unsafe { std::slice::from_raw_parts(data, data_len as ffi::size_t) };
        controller.hci_events_out.fetch_add(1, Ordering::Relaxed);
        let mut h4_buffer = BytesMut::with_capacity(data_len + 1);
        h4_buffer.put_u8(idc as u8);
        h4_buffer.extend_from_slice(data_slice);
        controller.callbacks.send_hci(controller.get_id(), h4_buffer.into());
    }
}

// The trampoline function that is called by the C++ code.
extern "C" fn invalid_packet_trampoline(
    cookie: *mut c_void,
    reason: c_int,
    message: *const std::os::raw::c_char,
    data: *const u8,
    data_len: ffi::size_t,
) {
    println!("invalid packet received");
    // SAFETY: The `cookie` is an opaque pointer passed to us from the C++ side.
    let context = unsafe { context_from_cookie(cookie) };
    if let Some(controller) = context.upgrade() {
        controller.invalid_packets.fetch_add(1, Ordering::Relaxed);
        // SAFETY: The FFI contract guarantees that `message` is a valid, null-terminated
        // C string.
        let message_str = unsafe { std::ffi::CStr::from_ptr(message) }.to_str().unwrap_or("");
        // SAFETY: The FFI contract guarantees that `data` points to a buffer of at
        // least `data_len` bytes.
        let data_slice = unsafe { std::slice::from_raw_parts(data, data_len as ffi::size_t) };
        controller.callbacks.invalid_packet_received(
            controller.get_id(),
            reason,
            message_str,
            data_slice,
        );
    }
}

// The trampoline function that is called by the C++ code.
extern "C" fn send_ll_trampoline(
    cookie: *mut c_void,
    data: *const u8,
    data_len: ffi::size_t,
    phy: c_int,
    tx_power: c_int,
) {
    // SAFETY: The FFI contract guarantees that the C++ side provides a valid `cookie`.
    let context = unsafe { context_from_cookie(cookie) };
    if let Some(controller) = context.upgrade() {
        // SAFETY: The FFI contract guarantees that `data` points to a buffer of at
        // least `data_len` bytes that is valid for the duration of this call.
        let data_slice = unsafe { std::slice::from_raw_parts(data, data_len as ffi::size_t) };
        controller.ll_packets_out.fetch_add(1, Ordering::Relaxed);
        // rootcanal request to send ll through bluetooth medium
        controller.bt_ops.broadcast_rootcanal_ll_packet(
            controller.get_id(),
            data_slice,
            Phy::from(phy),
            tx_power,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rootcanal::{self, Rootcanal};
    use std::str::FromStr;

    struct MockRootcanalCallbacks;
    impl rootcanal::Callbacks for MockRootcanalCallbacks {
        fn on_send_ll(
            &self,
            _source_id: Id,
            _destination_id: Id,
            _packet: &[u8],
            _phy: Phy,
            tx_power: i32,
        ) -> Option<i32> {
            Some(tx_power)
        }
    }

    struct MockControllerCallbacks;
    impl Callbacks for MockControllerCallbacks {
        fn send_hci(&self, _source_id: Id, _data: &[u8]) {}
        fn send_ll(&self, _source_id: Id, _packet: &[u8], _phy: Phy, _tx_power: i32) {}
        fn invalid_packet_received(
            &self,
            _source_id: Id,
            _reason: c_int,
            _message: &str,
            _data: &[u8],
        ) {
        }
    }

    #[test]
    fn test_controller_stats() {
        let _rootcanal = Rootcanal::new(Box::new(MockRootcanalCallbacks));
        let address = Address::from_str("01:02:03:04:05:06").unwrap();
        let controller = ControllerImpl::new(1, address, Box::new(MockControllerCallbacks));

        // Check initial stats.
        assert_eq!(controller.get_stats(), Stats::default());

        // Modify some stats.
        controller.hci_commands_in.fetch_add(1, Ordering::Relaxed);
        controller.ll_packets_out.fetch_add(5, Ordering::Relaxed);

        let stats = controller.get_stats();
        assert_eq!(stats.hci_commands_in, 1);
        assert_eq!(stats.ll_packets_out, 5);
        assert_eq!(stats.hci_events_out, 0);

        // Clear stats.
        controller.clear_stats();
        assert_eq!(controller.get_stats(), Stats::default());
    }

    #[test]
    fn test_receive_hci_increments_counter() {
        let _rootcanal = Rootcanal::new(Box::new(MockBluetoothCallbacks));
        let address = Address::from_str("01:02:03:04:05:06").unwrap();
        let controller = ControllerImpl::new(1, address, Box::new(MockControllerCallbacks));

        assert_eq!(controller.get_stats().hci_commands_in, 0);
        controller.receive_hci(&[1, 1, 2, 3]);
        assert_eq!(controller.get_stats().hci_commands_in, 1);
    }

    #[test]
    fn test_receive_ll_increments_counter() {
        let _rootcanal = Rootcanal::new(Box::new(MockBluetoothCallbacks));
        let address = Address::from_str("01:02:03:04:05:06").unwrap();
        let controller = ControllerImpl::new(1, address, Box::new(MockControllerCallbacks));

        assert_eq!(controller.get_stats().ll_packets_in, 0);
        controller.receive_ll(&[1, 2, 3], Phy::LowEnergy, -80);
        assert_eq!(controller.get_stats().ll_packets_in, 1);
    }
}
