# Requirements: Device Actor

Device-Actor - Manager of Simulated Devices, Chips, and Spatial Properties

## 1. Introduction

### 1.1 Purpose

Device-Actor is a core user-space microservice responsible for the lifecycle
management of simulated devices (e.g., Android phones, beacons, sniffers, access
points) and their associated radio chips (WiFi, Bluetooth, UWB).

It acts as the central **Topology Manager** and orchestration layer for:

- **Lifecycle Management**: Creation and deletion of simulated devices.
- **Spatial Consistency**: Maintaining the Single Source of Truth for 3D
  position/orientation and synchronously propagating updates to all subsystems.
- **Chip Orchestration**: Routing chip lifecycle events to specialized actors
  (WiFi-Actor, Bluetooth-Actor) without leaking domain-specific logic.
- **Unified Control Plane**: providing a stable, versioned API surface for
  `netsim` frontends (CLI, gRPC, Web UI) that abstracts the complexity of the
  underlying radio simulation.

### 1.2 Scope

The Device-Actor component handles the logic for:

- **Device Lifecycle**: Creation, Update, Deletion, and Listing of devices.
- **Chip Orchestration**: Requesting radio actors to create/destroy chip state
  when chips are added/removed from a device.
- **Spatial Management**: Managing 3D position and orientation, ensuring all
  attached chips are updated synchronously.
- **Chip Lifecycle Notification**: Informing the `Link-Actor` and
  `Capture-Actor` of active chips to enable radio medium simulation.

The following are **out of scope**:

- **Radio Protocol Logic**: The Device-Actor does not simulate radio frames or
  state machines (e.g., 802.11 beacons, BLE advertising). This is delegated to
  `WiFi-Actor`, `Bluetooth-Actor`, etc.
- **Medium Simulation**: Path loss and interference calculations are handled by
  the `Link-Actor`.

### 1.3 Definitions and Acronyms (Glossary)

| Term              | Definition                                                                                                                                                                                                                                                                |
| ----------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Device**        | A logical entity representing a physical object. Categorized into: <br> 1. **External Device**: Externally provisioned (AVD, Bumble). Has external packet streams. <br> 2. **Internal Device**: Created by the user via CLI/gRPC (e.g., Beacons, APs). Simulation-driven. |
| **Chip**          | A simulated radio component (e.g., WLAN Interface, Bluetooth Controller).                                                                                                                                                                                                 |
| **ChipId**        | A unique identifier for a radio chip across the entire simulation.                                                                                                                                                                                                        |
| **DeviceId**      | A unique identifier for a device.                                                                                                                                                                                                                                         |
| **NetworkKind**   | The technology type of a chip (WiFi, Bluetooth, UWB, Thread, NFC).                                                                                                                                                                                                        |
| **Link-Actor**    | The component responsible for managing static RF path loss between chips.                                                                                                                                                                                                 |
| **Capture-Actor** | The component responsible for collecting packet traces (PCAP) from chips.                                                                                                                                                                                                 |

## 2. Overall Description

### 2.1 Product Perspective

Device-Actor is the main orchestrator for the network simulation. When a user
runs `netsim device create p1`, they are instantiating a virtual presence in the
simulated world. It sits "above" the specific radio actors in the architecture,
strictly enforcing:

1.  **Physical Presence**: A chip cannot exist without a parent Device.
2.  **Spatial Locality**: A chip cannot be at a different location than its
    parent Device.

**Key Concept: Rigid Spatial Coupling** A Device acts as a rigid container for
multiple chips. Moving the Device (changing its (x,y,z)) **instantaneously and
atomically** moves all attached chips (WiFi, BLE, UWB) to the exact same global
coordinate. Chips cannot have independent positions relative to the Device.

It acts as a facade, hiding the complexity of `WiFi-Actor` and `Bluetooth-Actor`
internals from the user.

It interacts with:

- **Netsim Frontends**: Receives CRUD commands.
- **Chip Clients**: Sends `Create`/`Delete`/`Update` commands to `WifiActor`,
  `BluetoothActor`, etc.
- **Link Client**: Notifies when chips enter or leave the simulation
  capabilities.
- **Capture Client**: Wraps streams to intercept packets for logging.

### 2.2 Device Provisioning Model

The system supports two distinct provisioning flows:

1.  **External Devices** (e.g., AVD, Bumble):
    - **Source**: Externally provisioned entities connecting to the netsim
      daemon.
      - **Android Virtual Devices (AVD)**: Full Android Emulator instances.
      - **Bumble**: Python host programs creating HCI connections for
        testing/scripting.
    - **Identity**: Identified by a unique **GUID**.
    - **Association**: Multiple packet streams (WiFi, Bluetooth) presenting the
      same GUID are automatically grouped into the **same** External Device.
    - **Lifecycle**: Bound to the connection lifespan of the external process.

2.  **Internal Devices (Simulation-Driven)**:
    - **Source**: Explicitly created by the user via `netsim device create`
      (CLI/gRPC).
    - **Characteristics**: Represents purely simulated entities like **Bluetooth
      Beacons** or **Access Points**. typically instantiated as a
      **Single-Chip** device.
    - **Lifecycle**: Persists until explicitly deleted by the user or the
      generic idle timeout occurs.

### 2.3 User Characteristics

- **Application Developers**: Use the simulator transparently via Android
  Studio/AVD to test applications. They require zero-configuration
  "plug-and-play" behavior.
- **Test Engineers (Automation)**: Write deterministic scripts (e.g., Mobly,
  Python) to validate range/roaming scenarios in CI/CD pipelines. They require
  precise API control.
- **Simulation Operators (Interactive)**: Manually inspect the virtual world via
  the Web UI or CLI to debug complex topologies or visualize device
  relationships.

### 2.3 User Needs

- **UN-001**: **Persistence**: Devices must maintain their configuration and
  chip set until explicitly removed or the simulation ends.
- **UN-002**: **Spatial Consistency**: When a device moves, all its chips (WiFi,
  BLE, UWB) must move instantly to correct coordinates.
- **UN-003**: **Dynamic Reconfiguration**: Users must be able to add or remove
  chips at runtime (e.g., simulating a broken radio).

### 2.4 Constraints

- **Startup Dependency**: Must not block startup; however, it generally depends
  on Radio Actors being available to create chips.
- **State Synchronization**: Must ensure `Link-Actor` and Radio Actors have a
  consistent view of chip existence.

### 2.5 Assumptions and Dependencies

- Radio Actors (`WiFi`, `Bluetooth`, `UWB`) are running and registered as
  `ChipClients`.
- `Link-Actor` is available to receive topology updates.

## 3. Standards and References

- Netsim internal gRPC / JSON-RPC definitions.
- [`netsim-model`](../netsim-model): Defines shared types like `Position`,
  `Orientation`, `ChipConfig`.

## 4. Functional Requirements

> [!IMPORTANT] All functional requirements listed below are mandatory (Priority
> P0).

### 4.1 Device Lifecycle

- **RQ-DEV-01**: The system shall allow creating a device with an optional name,
  initial position, and orientation.
- **RQ-DEV-02**: The system shall assign a unique `DeviceId` to every created
  device.
- **RQ-DEV-03**: The system shall allow deleting a device, which must trigger
  the removal of all its associated chips.
- **RQ-DEV-04**: The system shall provide a list of all active devices,
  including their properties and attached chips.

### 4.2 Switchboard & Chip Orchestration

- **RQ-CHIP-01**: The system shall support "hot-plugging" chips to an existing
  device (dynamic radio attachment).
- **RQ-CHIP-02**: The system shall categorize chips by `NetworkKind` (WiFi,
  Bluetooth, UWB, etc.) and route creation requests to the appropriate actor
  **via a strict client interface**.
- **RQ-CHIP-03**: The system shall notify **downstream observers** (`Link-Actor`
  for RF model, `Capture-Actor` for logging state) **synchronously** whenever a
  chip is added or removed to ensure consistent simulation topology (RF paths)
  and resource cleanup (capture handles).
- **RQ-CHIP-04**: If a requested chip type lacks a registered client (e.g., UWB
  actor is disabled), the system shall **fail the request hard** with a specific
  error code (`ActorUnavailable`), ensuring tests do not proceed with a
  partially broken device state.

### 4.3 Spatial Management

- **RQ-SPACE-01**: The system shall allow updating a device's global position
  (x, y, z) with **double-precision** coordinates.
- **RQ-SPACE-02**: Upon a device position update, the system shall propagate the
  new position to **all** attached chips via their respective actors.
  - _Constraint_: This propagation must use asynchronous notification patterns
    that do **not** block the main actor loop for >1ms.
- **RQ-SPACE-03**: The system shall allow updating a device's orientation (yaw,
  pitch, roll) and propagate this to all attached chips.

### 4.4 Observability & Capture

- **RQ-OBS-01**: The system shall support **transparent interception** of packet
  streams (Rx/Tx) for **External Devices**. The system must wrap the stream into
  a `CapturedPacketStream` that allows the `Capture-Actor` to dynamically toggle
  PCAP logging on or off _before_ passing it to the radio actor.
- **RQ-OBS-02**: The system shall provide a `List` API returning a snapshot of
  all devices, their Attached Chips, and current Position.
  - _Constraint_: This list must be strongly consistent (reflecting the exact
    state of the `devices` HashMap).

### 4.5 Orchestration Policies

- **RQ-POL-01**: **Idle Shutdown Policy**: The system shall support a
  configurable idle timeout (default: 0s). If the device count drops to zero
  for `N` seconds, the actor shall signal termination to the Daemon. (A default
  of 0s matches legacy netsimd's immediate exit on last device removal).
- **RQ-POL-02**: **Lazy Startup Policy**: The system shall allow a "Startup
  Grace Period" (default: 15s) where the idle timer is suppressed, allowing time
  for the first emulator to connect before self-terminating.

## 5. Non-Functional Requirements

- **RQ-NFR-SCALE**: The system shall support **64 concurrent devices**, each
  with 3 chips (WiFi, BLE, UWB), with no degradation in command processing
  latency.
- **RQ-NFR-LATENCY**: Position updates must be propagated to radio actors within
  **<5ms** (99th percentile) to ensure RTT/ranging emulation integrity.
- **RQ-NFR-RESILIENCE**: parameter validation must reject invalid inputs (e.g.,
  NaN coordinates, empty names) with strict `InvalidArgument` errors rather than
  panicking or defining undefined behavior.

## 6. Verification

Verification is performed via the standard Netsim testing strategy:

### 6.1 Automated Tests

- **Unit Tests**:
  - `handle_create`: Verify `DeviceId` generation and `InternalDevice` state.
  - `handle_update`: Verify position updates are pushed to `ChipClient` mocks.
  - `handle_delete`: Verify cleanup calls to `ChipClient` and `LinkClient`.
- **Integration Tests**:
  - **End-to-End**: Create a device with WiFi and Bluetooth chips, verifying
    they appear in `Link-Actor`'s discovery.
  - **Capture**: Verify that streams created via `DeviceActor` produce PCAP
    files when capture is enabled.

### 6.2 BDD Coverage Mapping

| Requirement ID  | Requirement Description           | Test File                | Test Scenario                                             | Coverage    |
| :-------------- | :-------------------------------- | :----------------------- | :-------------------------------------------------------- | :---------- |
| **RQ-DEV-01**   | Create device (name, pos, orient) | `create_device_test.rs`  | Successfully create a device                              | **Full**    |
| **RQ-DEV-02**   | Unique `DeviceId`                 | `create_device_test.rs`  | (Implicit in `when_create_device` returning distinct IDs) | **Partial** |
| **RQ-DEV-03**   | Delete device & remove chips      | `delete_device_test.rs`  | Delete a device with chips                                | **Full**    |
| **RQ-DEV-04**   | List active devices               | `list_device_test.rs`    | List devices populated                                    | **Full**    |
| **RQ-CHIP-01**  | Add chips to existing device      | `add_chip_test.rs`       | Add chip to existing device                               | **Full**    |
| **RQ-CHIP-02**  | Route chip creation by Kind       | `add_chip_test.rs`       | Create single Bluetooth chip (verifies Kind routing)      | **Full**    |
| **RQ-CHIP-03**  | Notify Observers (Link, Capture)  | `add_chip_test.rs`       | Link notification mocked; Capture missing                 | **Mixed**   |
| **RQ-CHIP-04**  | Error if no chip client           | `N/A`                    | **Missing**                                               | **None**    |
| **RQ-SPACE-01** | Update device position            | `update_device_test.rs`  | Update device properties propagates                       | **Full**    |
| **RQ-SPACE-02** | Propagate Position to Chips       | `update_device_test.rs`  | Update device propagates to chips                         | **Full**    |
| **RQ-SPACE-03** | Update Orientation                | `update_device_test.rs`  | Update device properties propagates                       | **Full**    |
| **RQ-OBS-01**   | Packet Capture Wrapping           | `N/A`                    | **Missing**                                               | **None**    |
| **RQ-OBS-02**   | Track active count                | `list_device_test.rs`    | List devices populated                                    | **Full**    |
| **RQ-POL-01**   | Idle Timeout                      | `actor_shutdown_test.rs` | Server shuts down after idle timeout                      | **Full**    |
| **RQ-POL-02**   | Startup Timeout                   | `actor_shutdown_test.rs` | Server shuts down on startup timeout                      | **Full**    |

### 6.3 Coverage Gaps

The following requirements currently lack automated test coverage and should be
prioritized for the next test sprint:

- **RQ-CHIP-04**: Handling of missing chip clients (e.g., if UWB is disabled) is
  not verified.
- **RQ-OBS-01**: Capture stream wrapping logic is implemented but not explicitly
  tested in `device-actor` integration tests.
- **RQ-CHIP-03**: **Partial Coverage**. Notification to `Link-Actor` is verified
  via mocks, but notification to `Capture-Actor` (for lifecycle management) is
  **missing** in both implementation and tests.
- **RQ-NFR-SCALE**: No load test exists to verify support for 32+ simultaneous
  devices.
- **RQ-NFR-SYNC**: No latency benchmark exists for position propagation.
