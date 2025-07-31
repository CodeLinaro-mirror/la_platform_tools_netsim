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
    - [ ] `ATS10=` (SetAutomaticDisconnectDelay)
    - [ ] `AT+GCAP` (GetCapabilities)
    - [ ] `AT+GMI` (GetManufacturerIdentification)
    - [ ] `AT+GMM` (GetModelId)
    - [ ] `AT+GMR` (GetRevision)
    - [ ] `AT+GSN` (GetSerialNumber)
    - [ ] `AT+ICF=` (SetTeTaControlCharacterFraming)
    - [ ] `AT+IFC=` (SetTeTaLocalDataFlowControl)

## Phase 15: Architectural Shift to TCP-based Daemon (Lower Priority)

This phase refactors the application to a centralized server model using TCP for cross-platform communication and a proper daemonization library for robust, detached execution of the server process.

- [ ] **Phase 15.1: Dependency Updates**
- [ ] **Phase 15.2: Server-Side Implementation (`server_main`)**