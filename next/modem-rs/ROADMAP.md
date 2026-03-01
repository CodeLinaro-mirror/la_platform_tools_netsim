# Modem Simulator Rewrite Plan

This document outlines the active and planned development tasks for the modem
simulator.

## Completed Milestones (Phases 1-14)

- **Architectural Refactoring & Unification**: Completed.
- **Event Loop & Timers**: Implemented.
- **Initial Test Coverage**: Expanded to over 100 passing tests across all
  services (Call, Data, Network, SIM, SMS, STK, Sup).

## Phase 14b: Android Emulator Console Support

Required features to support the standard `emulator -console` telephony
commands. These commands allow users (and Android Studio) to inject events into
the modem.

- [ ] **Incoming Call (`gsm call`)**:
  - [ ] Implement logic to trigger an incoming call (`RING`, `+CLIP`) from an
        external event.
- [ ] **Remote Call State Changes (`gsm accept`, `gsm hold`)**:
  - [ ] Implement logic to simulate remote party answering (Active) or putting
        call on hold (Held).
- [ ] **Call Termination (`gsm cancel`)**:
  - [ ] Implement logic to terminate a call from the network side (transitions
        to `SimState::Ready` w/ `NO CARRIER` etc).
- [ ] **Data/Voice State (`gsm data`, `gsm voice`)**:
  - [ ] Allow external control of `+CREG` and `+CGREG` states (Roaming,
        Searching, Denied, etc.) via unsolicited updates.
  - [ ] Support "Metered" network status if applicable.
- [ ] **Signal Strength (`gsm signal`)**:
  - [ ] Allow external updates to signal strength (RSSI) and BER.
- [ ] **Network Time (`updateClock`)**:
  - [ ] Support external injection of Network Identity and Time Zone (NITZ)
        updates.
- [ ] **Incoming SMS (`sms send`, `sms pdu`)**:
  - [ ] Implement injection of text-mode SMS (`+CMT`).
  - [ ] Implement injection of PDU-mode SMS (`+CMT`).

## Phase 15: Cuttlefish Drop-in Compatibility

Ensure the binary functions as a direct drop-in replacement for the legacy modem
simulator used by Cuttlefish (and eventually Netsim). Focus on simplifying the
execution model to a single binary where possible, ensuring strictly compatible
CLI arguments and FD handling.

- [ ] **Verify CLI Compatibility**: Ensure arguments like `--server-fds` match
      legacy behavior.
- [ ] **Simplify Execution Model**: Evaluate removing the "Daemon/Proxy" split
      in favor of a monolithic run loop for Cuttlefish.
- [ ] **Integration Test**: Verify the binary runs correctly within a Cuttlefish
      instance.

## Phase 16: Feature Parity with C++ Implementation

Based on a detailed comparison of the C++ and Rust source code, the following
features are missing from the Rust implementation. This plan outlines the work
required to achieve feature parity.

### Call Service (`src/call_service.rs`)

**Context:** The Rust `call_service.rs` is a good starting point, but it is
missing several key features and the detailed logic that is present in the C++
`call_service.cpp`.

**Tasks:**

- [ ] **Implement full `ATD` command handling:**
  - [ ] Add support for emergency number dialing with categories and CLIR
        (`ATDnumber@[category],#[clir];`).
  - [ ] Integrate with `SimService` to perform FDN (Fixed Dialing Number)
        checks.
- [ ] **Expand `AT+CHLD` command handling:**
  - [ ] Implement all modes (0, 1, 2, 3, 4) of the `AT+CHLD` command.
- [ ] **Add `AT+CUSD` command handling:**
  - [ ] Implement the `AT+CUSD=` command for canceling USSD sessions.
- [ ] **Improve error handling:**
  - [ ] Use specific CME (Cellular Messaging Entity) error codes.
- [ ] **Enhance state management:**
  - [ ] Add `is_international`, `can_present_number`, and `timeout_serial`
        fields to the `CallStatus` struct.
- [ ] **Clarify dependencies:**
  - [ ] Make the dependencies on `SimService` and `NetworkService` more
        explicit.

### Data Service (`src/data_service.rs`)

**Context:** The Rust `data_service.rs` is a mix of being more and less
feature-complete than the C++ `data_service.cpp`. The Rust version has better
support for QoS commands, but it is missing the detailed logic for handling PDP
contexts, data call activation, and physical channel configurations.

**Tasks:**

- [ ] **Implement `AT+CGACT?` (Query Data Call List):**
  - [ ] Add a function to return a list of active PDP contexts.
- [ ] **Enhance `AT+CGDCONT` (Define PDP Context):**
  - [ ] Modify the `handle_define_pdp_context` function to get the IP address,
        DNS servers, and gateways from a configuration source.
- [ ] **Improve `AT+CGDATA` (Enter Data State):**
  - [ ] Add a check to the `handle_enter_data_state` function to ensure the
        specified PDP context is active.
- [ ] **Implement `AT+CGCONTRDP` (Read Dynamic Parameters):**
  - [ ] Modify the `handle_read_dynamic_param` function to return the correct
        values for the specified PDP context.
- [ ] **Add Physical Channel Configuration:**
  - [ ] Implement the logic for updating and sending physical channel
        configuration updates (`%CGFPCCFG`).

### Misc Service (`src/misc_service.rs`)

**Context:** The Rust `misc_service.rs` is significantly less feature-complete
than the C++ `misc_service.cpp`. The C++ version has much more advanced logic
for handling time, time zones, and initialization commands.

**Tasks:**

- [ ] **Implement full `AT+CGSN` command handling:**
  - [ ] Add support for the `snt` parameter to return different types of
        identification information (IMEI, SVN, etc.).
- [ ] **Add time and time zone support:**
  - [ ] Implement logic for parsing the time zone from the system.
  - [ ] Implement logic for calculating the time zone offset.
  - [ ] Implement the `%CTZV` unsolicited response for time updates.
- [ ] **Add initialization commands:**
  - [ ] Implement the missing initialization commands (e.g., `E0Q0V1`, `S0=0`,
        `+CMEE=1`).

### Network Service (`src/network_service.rs`)

**Context:** The Rust `network_service.rs` is a skeleton of a service and needs
a massive amount of work to reach feature parity with the C++
`network_service.cpp`.

**Tasks:**

- [ ] **Implement Radio Power Management (`AT+CFUN`):**
  - [ ] Add support for setting and querying the radio power state.
- [ ] **Implement Network Selection (`AT+COPS`):**
  - [ ] Add support for querying and setting the network selection mode.
  - [ ] Add support for querying available networks.
  - [ ] Add support for requesting the current operator.
- [ ] **Implement Network Registration (`AT+CREG`, `AT+CGREG`, `AT+CEREG`):**
  - [ ] Add support for voice and data network registration.
  - [ ] Implement unsolicited registration status updates.
- [ ] **Implement Signal Strength (`AT+CSQ`):**
  - [ ] Implement the Cuttlefish-specific `+CSQ` command with a detailed
        `SignalStrength` struct.
  - [ ] Implement a loop to continuously update the signal strength.
- [ ] **Implement Preferred Network Type (`AT+CTEC`):**
  - [ ] Add support for getting and setting the preferred network type.
- [ ] **Integrate with NVRAM Configuration:**
  - [ ] Use a configuration management system to store and retrieve
        network-related settings.
- [ ] **Integrate with `SimService`:**
  - [ ] Use the `SimService` to initialize the network operator.

### SIM Service (`src/sim_service.rs`)

**Context:** The Rust `sim_service.rs` is a very basic implementation that is
missing most of the features and complexity of the C++ `sim_service.cpp`.

**Tasks:**

- [ ] **Implement a robust SIM File System:**
  - [ ] Create a hierarchical file system model (MF, DF, EF).
  - [ ] Implement a parser for the XML-based SIM profile.
  - [ ] Implement full `AT+CRSM` and `AT+CSIM` command handling.
- [ ] **Implement full PIN/PUK Management (`AT+CPIN`):**
  - [ ] Handle all SIM states (ABSENT, NOT_READY, READY, PIN, PUK).
  - [ ] Handle all PIN/PUK operations correctly.
- [ ] **Implement Facility Lock (`AT+CLCK`):**
  - [ ] Add support for locking, unlocking, and querying all facilities.
- [ ] **Enhance Logical Channel Support (`AT+CCHO`, `AT+CCHC`, `AT+CGLA`):**
  - [ ] Add support for Application Identifiers (AIDs).
- [ ] **Implement CDMA Features (`AT+CCSS`, `AT+WRMP`):**
  - [ ] Implement the CDMA-specific commands.
- [ ] **Implement SIM Authentication (`^MBAU`):**
  - [ ] Implement the `^MBAU` command.
- [ ] **Implement Phone Number Management:**
  - [ ] Implement functions for getting and setting the phone number from the
        SIM file system.
- [ ] **Implement FDN (Fixed Dialing Number):**
  - [ ] Implement the FDN check.

### SMS Service (`src/sms_service.rs`)

**Context:** The Rust `sms_service.rs` and C++ `sms_service.cpp` have different
strengths and weaknesses. The focus should be on implementing the missing
features in the Rust version.

**Tasks:**

- [ ] **Implement PDU Parsing:**
  - [ ] Add a PDU parser to handle SMS messages in PDU mode.
- [ ] **Implement SMS Status Reports:**
  - [ ] Add the logic for generating and sending SMS status reports.
- [ ] **Improve Error Handling:**
  - [ ] Use more specific CMS (Cellular Messaging Service) error codes.

### STK Service (`src/stk_service.rs`)

**Context:** The Rust `stk_service.rs` is a skeleton of a service and needs a
complete rewrite to achieve feature parity with the C++ `stk_service.cpp`.

**Tasks:**

- [ ] **Implement an STK Profile Parser:**
  - [ ] Create a parser for the XML-based STK profile.
- [ ] **Implement Proactive Command Handling:**
  - [ ] Handle all proactive commands defined in the STK profile.
- [ ] **Implement Terminal Response Handling (`AT+CUSATT`):**
  - [ ] Process terminal responses and trigger the correct unsolicited commands.
- [ ] **Implement Menu Navigation:**
  - [ ] Create a state management system for navigating the STK menu.
- [ ] **Integrate with `SimService`:**
  - [ ] Use the `SimService` to access the ICC profile.

### Supplementary Service (`src/sup_service.rs`)

**Context:** The Rust `sup_service.rs` is a very basic implementation that is
missing most of the features and complexity of the C++ `sup_service.cpp`.

**Tasks:**

- [ ] **Implement full USSD handling (`AT+CUSD`):**
  - [ ] Add logic for managing USSD sessions.
- [ ] **Implement full CLIR handling (`AT+CLIR`):**
  - [ ] Add the `ClirStatusInfo` struct and logic for setting and querying the
        CLIR status.
- [ ] **Implement full CLIP handling (`AT+CLIP`):**
  - [ ] Add logic for setting and querying the CLIP status.
- [ ] **Implement full Call Waiting handling (`AT+CCWA`):**
  - [ ] Add the `CallWaitingInfo` struct and logic for setting and querying the
        call waiting status.
- [ ] **Enhance Call Forwarding handling (`AT+CCFCU`):**
  - [ ] Expand the `handle_call_forwarding` function to support multiple call
        forwarding rules.
- [ ] **Implement Supplementary Service Notifications (`AT+CSSN`):**
  - [ ] Add logic for handling supplementary service notifications.
