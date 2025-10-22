# Modem Simulator Rewrite Plan

This document outlines the active and planned development tasks for the modem simulator.

## Phase 10: Architectural Refactoring (Contextual Strangler)

**[COMPLETED]**

## Phase 11: Unify Service Structs

**[COMPLETED]**

## Phase 12: Rename ModemManager to CellularNetworkSimulator

**[COMPLETED]**

## Phase 13: Implement Event Loop and Timers

**[COMPLETED]**

## Phase 14: Test Coverage Expansion (Lower Priority)

- [X] **SIM Service:**
    - [X] `AT+CCHO=` (OpenLogicalChannel)
    - [X] `AT+CCHC=` (CloseLogicalChannel)
    - [X] `AT+CGLA=` (TransmitLogicalChannel)
    - [X] `AT+CPWD=` (ChangePassword)
    - [X] `AT+CCSS=` (SetCdmaSubscriptionSource)
    - [X] `AT+WRMP=` (SetCdmaRoamingPreference)
    - [X] `AT+MBAU=` (SimAuthentication)
    - [X] `AT+REMOTEUPADATEPHONENUMBER` (UpdatePhoneNumber)
- [X] **Call Service:**
    - [X] `AT+CMUT=` (SetMute)
    - [X] `AT+CMUT?` (QueryMute)
    - [X] `AT+VTS=` (SendDtmf)
    - [X] `AT+WSOS=` (SetEmergencyMode)
    - [X] `AT+WSOS?` (QueryEmergencyMode)
- [X] **Network Service:**
    - [X] `AT+CESQ` (QueryExtendedSignalQuality)
- [X] **SMS Service:**
    - [X] `AT+CMGF=` (SetSmsMessageFormat)
    - [X] `AT+CPMS=` (SetPreferredMessageStorage)
- [X] **STK Service:**
    - [X] `AT+CUSATD?` (QueryStkReady)
    - [X] `AT+STK=` (SetStk)
    - [X] `AT+STKEN=` (SetStkEnabled)
    - [X] `AT+STKUR=` (SetStkUnsolicitedResult)
- [X] **Supplementary Service:**
    - [X] `AT+CLCK=` (SetFacilityLock)
- [X] **Data Service:**
    - [X] `AT+CGDCONT?` (QueryPdpContext)
    - [X] `AT+CGEQMIN?` (QueryQualityOfServiceMinimum)
    - [X] `AT+CGEQREQ=` (SetQualityOfServiceRequested)
    - [X] `AT+CGQMIN=` (SetQualityOfServiceMinimumGprs)
    - [X] `AT+CGQREQ=` (SetQualityOfServiceRequestedGprs)
    - [X] `AT+CGATT=` (SetPsAttach)
    - [X] `AT+CGCMOD=` (SetPdpContextModify)
    - [X] `AT+CGDATA=` (EnterDataState)
    - [X] `AT+CGEREP=` (SetPacketEventReporting)
    - [X] `AT+CGPADDR=` (ShowPdpAddress)
- [ ] **Miscellaneous:**
    - [X] `ATE` (SetEcho)
    - [X] `ATL` (SetSpeakerVolume)
    - [X] `ATM` (SetSpeakerMute)
    - [X] `ATQ` (SetQuietMode)
    - [X] `ATV` (SetVerboseMode)
    - [X] `AT&F` (ResetToFactoryDefaults)
    - [X] `AT&V` (ViewActiveConfiguration)
    - [X] `AT&W` (WriteActiveConfiguration)
    - [X] `ATZ` (Reset)
    - [X] `ATI` (GetIdentificationInformation)
    - [X] `ATS0=` (SetAutoAnswer)
    - [X] `ATS3=` (SetCommandTerminationCharacter)
    - [X] `ATS4=` (SetResponseFormattingCharacter)
    - [X] `ATS5=` (SetCommandLineEditingCharacter)
    - [X] `ATS6=` (SetPauseBeforeBlindDialing)
    - [X] `ATS7=` (SetConnectionCompletionTimeout)
    - [X] `ATS8=` (SetCommaDialModifierTime)
    - [X] `ATS10=` (SetAutomaticDisconnectDelay)
    - [X] `AT+GCAP` (GetCapabilities)
    - [X] `AT+GMI` (GetManufacturerIdentification)
    - [X] `AT+GMM` (GetModelId)
    - [X] `AT+GMR` (GetRevision)
    - [X] `AT+GSN` (GetSerialNumber)
    - [X] `AT+ICF=` (SetTeTaControlCharacterFraming)
    - [X] `AT+IFC=` (SetTeTaLocalDataFlowControl)

## Phase 15: Architectural Shift to TCP-based Daemon (Lower Priority)

This phase refactors the application to a centralized server model using TCP for cross-platform communication and a proper daemonization library for robust, detached execution of the server process.

- [ ] **Phase 15.1: Dependency Updates**
- [ ] **Phase 15.2: Server-Side Implementation (`server_main`)**

## Phase 16: Feature Parity with C++ Implementation

Based on a detailed comparison of the C++ and Rust source code, the following features are missing from the Rust implementation. This plan outlines the work required to achieve feature parity.

### Call Service (`src/call_service.rs`)

**Context:** The Rust `call_service.rs` is a good starting point, but it is missing several key features and the detailed logic that is present in the C++ `call_service.cpp`.

**Tasks:**
- [ ] **Implement full `ATD` command handling:**
    - [ ] Add support for emergency number dialing with categories and CLIR (`ATDnumber@[category],#[clir];`).
    - [ ] Integrate with `SimService` to perform FDN (Fixed Dialing Number) checks.
- [ ] **Expand `AT+CHLD` command handling:**
    - [ ] Implement all modes (0, 1, 2, 3, 4) of the `AT+CHLD` command.
- [ ] **Add `AT+CUSD` command handling:**
    - [ ] Implement the `AT+CUSD=` command for canceling USSD sessions.
- [ ] **Improve error handling:**
    - [ ] Use specific CME (Cellular Messaging Entity) error codes.
- [ ] **Enhance state management:**
    - [ ] Add `is_international`, `can_present_number`, and `timeout_serial` fields to the `CallStatus` struct.
- [ ] **Clarify dependencies:**
    - [ ] Make the dependencies on `SimService` and `NetworkService` more explicit.

### Data Service (`src/data_service.rs`)

**Context:** The Rust `data_service.rs` is a mix of being more and less feature-complete than the C++ `data_service.cpp`. The Rust version has better support for QoS commands, but it is missing the detailed logic for handling PDP contexts, data call activation, and physical channel configurations.

**Tasks:**
- [ ] **Implement `AT+CGACT?` (Query Data Call List):**
    - [ ] Add a function to return a list of active PDP contexts.
- [ ] **Enhance `AT+CGDCONT` (Define PDP Context):**
    - [ ] Modify the `handle_define_pdp_context` function to get the IP address, DNS servers, and gateways from a configuration source.
- [ ] **Improve `AT+CGDATA` (Enter Data State):**
    - [ ] Add a check to the `handle_enter_data_state` function to ensure the specified PDP context is active.
- [ ] **Implement `AT+CGCONTRDP` (Read Dynamic Parameters):**
    - [ ] Modify the `handle_read_dynamic_param` function to return the correct values for the specified PDP context.
- [ ] **Add Physical Channel Configuration:**
    - [ ] Implement the logic for updating and sending physical channel configuration updates (`%CGFPCCFG`).

### Misc Service (`src/misc_service.rs`)

**Context:** The Rust `misc_service.rs` is significantly less feature-complete than the C++ `misc_service.cpp`. The C++ version has much more advanced logic for handling time, time zones, and initialization commands.

**Tasks:**
- [ ] **Implement full `AT+CGSN` command handling:**
    - [ ] Add support for the `snt` parameter to return different types of identification information (IMEI, SVN, etc.).
- [ ] **Add time and time zone support:**
    - [ ] Implement logic for parsing the time zone from the system.
    - [ ] Implement logic for calculating the time zone offset.
    - [ ] Implement the `%CTZV` unsolicited response for time updates.
- [ ] **Add initialization commands:**
    - [ ] Implement the missing initialization commands (e.g., `E0Q0V1`, `S0=0`, `+CMEE=1`).

### Network Service (`src/network_service.rs`)

**Context:** The Rust `network_service.rs` is a skeleton of a service and needs a massive amount of work to reach feature parity with the C++ `network_service.cpp`.

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
    - [ ] Implement the Cuttlefish-specific `+CSQ` command with a detailed `SignalStrength` struct.
    - [ ] Implement a loop to continuously update the signal strength.
- [ ] **Implement Preferred Network Type (`AT+CTEC`):**
    - [ ] Add support for getting and setting the preferred network type.
- [ ] **Integrate with NVRAM Configuration:**
    - [ ] Use a configuration management system to store and retrieve network-related settings.
- [ ] **Integrate with `SimService`:**
    - [ ] Use the `SimService` to initialize the network operator.

### SIM Service (`src/sim_service.rs`)

**Context:** The Rust `sim_service.rs` is a very basic implementation that is missing most of the features and complexity of the C++ `sim_service.cpp`.

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
    - [ ] Implement functions for getting and setting the phone number from the SIM file system.
- [ ] **Implement FDN (Fixed Dialing Number):**
    - [ ] Implement the FDN check.

### SMS Service (`src/sms_service.rs`)

**Context:** The Rust `sms_service.rs` and C++ `sms_service.cpp` have different strengths and weaknesses. The focus should be on implementing the missing features in the Rust version.

**Tasks:**
- [ ] **Implement PDU Parsing:**
    - [ ] Add a PDU parser to handle SMS messages in PDU mode.
- [ ] **Implement SMS Status Reports:**
    - [ ] Add the logic for generating and sending SMS status reports.
- [ ] **Improve Error Handling:**
    - [ ] Use more specific CMS (Cellular Messaging Service) error codes.

### STK Service (`src/stk_service.rs`)

**Context:** The Rust `stk_service.rs` is a skeleton of a service and needs a complete rewrite to achieve feature parity with the C++ `stk_service.cpp`.

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

**Context:** The Rust `sup_service.rs` is a very basic implementation that is missing most of the features and complexity of the C++ `sup_service.cpp`.

**Tasks:**
- [ ] **Implement full USSD handling (`AT+CUSD`):**
    - [ ] Add logic for managing USSD sessions.
- [ ] **Implement full CLIR handling (`AT+CLIR`):**
    - [ ] Add the `ClirStatusInfo` struct and logic for setting and querying the CLIR status.
- [ ] **Implement full CLIP handling (`AT+CLIP`):**
    - [ ] Add logic for setting and querying the CLIP status.
- [ ] **Implement full Call Waiting handling (`AT+CCWA`):**
    - [ ] Add the `CallWaitingInfo` struct and logic for setting and querying the call waiting status.
- [ ] **Enhance Call Forwarding handling (`AT+CCFCU`):**
    - [ ] Expand the `handle_call_forwarding` function to support multiple call forwarding rules.
- [ ] **Implement Supplementary Service Notifications (`AT+CSSN`):**
    - [ ] Add logic for handling supplementary service notifications.
