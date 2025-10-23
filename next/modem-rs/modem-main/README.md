# Modem Simulator Binary

`modem-main` is the main binary for the `modem-rs` project. It can run in two modes: a TCP-based server that allows multiple clients to connect and interact with simulated modems, and a command-line client for injecting events.

## Architecture

The `modem-main` binary implements a client-server architecture. The server runs as a daemon process and manages a central `CellularNetworkSimulator` instance. Clients connect to the server over TCP and are assigned a dedicated modem instance to interact with.

### Server

The server is responsible for the following:

*   **Daemonization**: The server runs as a background daemon process.
*   **TCP Listener**: The server listens for incoming TCP connections on a well-known port.
*   **Network Simulation**: The server creates and manages a `CellularNetworkSimulator` instance, which in turn manages all the `Modem` instances.
*   **Client Handling**: For each incoming client connection, the server spawns a new task to handle the client's requests.

### Client

The client is responsible for the following:

*   **Connecting to the Server**: The client connects to the server over TCP.
*   **Handshake**: The client sends a handshake message to the server to identify itself and request a modem instance.
*   **Proxying I/O**: The client proxies I/O between its standard input/output and the TCP socket connected to the server. This allows the client to be used with tools that expect to interact with a serial device.

### Communication Protocol

The client and server communicate over a simple TCP-based protocol.

1.  **Handshake**: When a client connects, it sends a handshake message to the server. The handshake is an HTTP-style request that includes a unique `modem_id` in the query string.
    - To register as a modem proxy: `REGISTER /modem?modem_id=123\r\n\r\n`
    - To inject a single command: `INJECT /modem?modem_id=123\r\n\r\n`
    - To list all active modems: `GET /modems\r\n\r\n`
2.  **AT Command Exchange**: After the handshake, the client and server exchange AT commands and responses over the raw TCP socket. The client sends AT commands to the server, and the server sends back the responses from the simulated modem.

## Usage

To run the modem simulator, you first need to start the server daemon. The binary will automatically daemonize if it's not already running. The primary way to interact with the simulator is through a client connection.

To start the server and a client simultaneously (the server will daemonize automatically):
```bash
# The server will start if not running. A client will connect for modem_id 1.
./modem-main --instance-id 1 --server-fds 0,1,2
```

To use the command-line interface to send an SMS:
```bash
./modem-main cli send-sms --target-instance 1 --destination "555-1234" --message "Hello"
```

To list all active modems:
```bash
./modem-main cli list-modems
```

```