# Netsim TAP Networking

This document describes how to use and verify the TAP (Network Tap) networking feature in `netsim-next`.

The TAP feature allows the emulated Android device (Goldfish) to have a direct layer 2 presence on the host network, instead of using the default user-space SLiRP stack.

## How it Works

When enabled, `netsimd` manages a TAP interface (or a pool of interfaces) on the host.
- Packets from the emulator's Wi-Fi interface are sent to `netsimd` via gRPC.
- `netsimd` writes these packets to the host TAP interface.
- Packets received on the host TAP interface are sent back to the emulator via gRPC.

This enables features like DHCP, IPv6, and direct connectivity between the emulator and other devices on the host network.

## Prerequisites

To use the TAP feature, you need to have TAP interfaces set up on your host and a DHCP server running to assign IP addresses to the emulators.

### Option 1: Using Cuttlefish TAP Pool (Recommended for Google Devs)

If you are developing on a gLinux machine (desktop or Cloudtop), you can use the pre-configured Cuttlefish TAP pool. If you haven't set it up yet, follow the instructions at **[go/cuttlefish-glinux](http://go/cuttlefish-glinux)** to install and configure the `cuttlefish-base` package.

1.  **Verify TAP devices are available:**
    You can check if the Cuttlefish TAP interfaces are created and up by running:
    ```bash
    ip link show | grep cvd-etap
    ```
    *Expected Output:* You should see a list of interfaces like `cvd-etap-01` through `cvd-etap-10`.

2.  **Ensure cuttlefish-host-resources is running:**
    If they are not visible or DOWN, start the host resources:
    ```bash
    sudo /etc/init.d/cuttlefish-host-resources start
    ```

3.  **Add your user to the `cvdnetwork` group:**
    To access the TAP devices without root, your user must be in the `cvdnetwork` group:
    ```bash
    sudo usermod -aG cvdnetwork $USER
    ```
    *Note: You may need to log out and log back in for group changes to take effect.*

`netsimd` will automatically use the sub-range `cvd-etap-06` to `cvd-etap-10` to avoid conflicts with active Cuttlefish instances.

### Option 2: Manual Setup (Custom TAP)

If you don't have Cuttlefish installed, you can set up a manual TAP interface.

1.  **Create the TAP interface (e.g., `tap0`):**
    ```bash
    sudo ip tuntap add mode tap tap0
    sudo ip link set tap0 up
    sudo ip addr add 192.168.1.1/24 dev tap0
    ```

2.  **Start a DHCP server (e.g., `dnsmasq`):**
    The emulator needs to acquire an IP address. Start `dnsmasq` on `tap0` (runs in the foreground, run this in a separate terminal window):
    ```bash
    sudo dnsmasq --interface=tap0 --bind-interfaces --dhcp-range=192.168.1.10,192.168.1.50,12h --no-daemon --port=0
    ```

3.  **Enable IP Forwarding (for Internet Access):**
    ```bash
    sudo sysctl -w net.ipv4.ip_forward=1
    ```

## How to Run

To run the emulator with the TAP gateway, you must ensure you are using a compatible emulator version and enable the next-gen Netsim feature.

### 1. Emulator Version Requirement
Ensure you are using Android Emulator version **`37.1.2.0`** or newer (available on the `git_emu-main-dev` branch). You can check your version by running `./objs/distribution/emulator/emulator -version` from `tools/netsim`.

### 2. Start the Emulator with NetsimX enabled
First, **`cd` into the `tools/netsim` directory** to ensure you are using the correct prebuilt emulator:
```bash
cd tools/netsim
```

Then, use the `-feature NetsimX` flag to launch `netsimdx` (the next-gen Netsim daemon) instead of `netsimd`:

*   **For Option 1 (Cuttlefish pool - Recommended)**:
    ```bash
    ./objs/distribution/emulator/emulator @Pixel_6 -feature NetsimX -netsim-args "--wifi-cvd-tap"
    ```
*   **For Option 2 (Custom TAP)**:
    ```bash
    ./objs/distribution/emulator/emulator @Pixel_6 -feature NetsimX -netsim-args "--wifi-tap tap0"
    ```

## How to Verify It Works

To confirm that the emulator is actually using the TAP interface and routing traffic through your host network (instead of falling back to the default SLiRP stack):

### 1. Check the IP Address on the AVD (Guest)
The default SLiRP stack assigns a `10.0.x.x` IP. If TAP is working, **the emulator's Wi-Fi interface must have an IP that matches the range configured in your DHCP server.**

Run the following command:
```bash
adb shell ip addr show wlan0
```
**Expected Output:**
You should see a line showing a global IP address matching your network:
*   **For Option 1 (Cuttlefish Pool)**: Typically in the **`192.168.98.x`** or **`192.168.96.x`** range.
    ```
    inet 192.168.98.10/24 brd 192.168.98.255 scope global wlan0
    ```
*   **For Option 2 (Custom TAP `tap0`)**: Matches the range you configured in `dnsmasq` (e.g., **`192.168.1.x`**).
    ```
    inet 192.168.1.21/24 brd 192.168.1.255 scope global wlan0
    ```

### 2. Check the `netsimd` Logs (Host)
Verify that `netsimd` successfully bound to the TAP interface by inspecting its stdout log (grep for both `tap0` and Cuttlefish's `cvd-etap`):
```bash
cat /tmp/android-$USER/netsimd/netsim_stdout.log | grep -E "tap0|cvd-etap"
```
**Expected Output:**
You should see logs indicating `netsimd` opened and attached the TAP interface:
*   **For Option 1 (Cuttlefish)**:
    ```
    netsim I ... tap_gateway.rs:114 - Opened TAP interface: cvd-etap-06
    netsim I ... tap_gateway.rs:446 - Attached TAP cvd-etap-06 to chip 0
    ```
*   **For Option 2 (Custom TAP)**:
    ```
    netsim I ... tap_gateway.rs:114 - Opened TAP interface: tap0
    netsim I ... tap_gateway.rs:446 - Attached TAP tap0 to chip 0
    ```
If you do not see these lines, double check that your host setup (Cuttlefish resources or custom `dnsmasq`/`tap0`) is running and active.
