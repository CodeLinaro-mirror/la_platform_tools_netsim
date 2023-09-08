/* eslint-disable */
import type {ChipKind} from './common';

export const protobufPackage = 'netsim.model';

export enum PhyKind {
  NONE = 'NONE',
  BLUETOOTH_CLASSIC = 'BLUETOOTH_CLASSIC',
  BLUETOOTH_LOW_ENERGY = 'BLUETOOTH_LOW_ENERGY',
  WIFI = 'WIFI',
  UWB = 'UWB',
  WIFI_RTT = 'WIFI_RTT',
  UNRECOGNIZED = 'UNRECOGNIZED',
}

/** An explicit valued boolean. */
export enum State {
  UNKNOWN = 'UNKNOWN',
  ON = 'ON',
  OFF = 'OFF',
  UNRECOGNIZED = 'UNRECOGNIZED',
}

/**
 * A 3D position. A valid Position must have both x and y coordinates.
 * The position coordinates are in meters.
 */
export interface Position {
  x: number;
  y: number;
  z: number;
}

export interface Orientation {
  yaw: number;
  pitch: number;
  roll: number;
}

export interface Chip {
  kind: ChipKind;
  id: number;
  /** optional like "rear-right" */
  name: string;
  /** optional like Quorvo */
  manufacturer: string;
  /** optional like DW300 */
  productName: string;
  /** dual mode. */
  bt?:|Chip_Bluetooth|undefined;
  /** low energy for beacon. */
  bleBeacon?: Chip_BluetoothBeacon|undefined;
  uwb?: Chip_Radio|undefined;
  wifi?: Chip_Radio|undefined;
}

/** Radio state associated with the Chip */
export interface Chip_Radio {
  state: State;
  range: number;
  txCount: number;
  rxCount: number;
}

/** Bluetooth has 2 radios */
export interface Chip_Bluetooth {
  lowEnergy: Chip_Radio|undefined;
  classic: Chip_Radio|undefined;
}

export interface Chip_BluetoothBeacon {
  /** TODO: Only include Radio low_energy. */
  bt: Chip_Bluetooth|undefined;
  address: string;
  settings: Chip_BluetoothBeacon_AdvertiseSettings|undefined;
  advData: Chip_BluetoothBeacon_AdvertiseData|undefined;
}

export interface Chip_BluetoothBeacon_AdvertiseSettings {
  advertiseMode?:|Chip_BluetoothBeacon_AdvertiseSettings_AdvertiseMode|
      undefined;
  /** Numeric time interval between advertisements in ms. */
  milliseconds?: number|undefined;
  txPowerLevel?:|Chip_BluetoothBeacon_AdvertiseSettings_AdvertiseTxPower|
      undefined;
  /** Numeric transmission power in dBm. Must be within [-127, 127]. */
  dbm?: number|undefined;
  scannable: boolean;
  timeout: number;
}

/**
 * From
 * packages/modules/Bluetooth/framework/java/android/bluetooth/le/BluetoothLeAdvertiser.java#151
 */
export enum Chip_BluetoothBeacon_AdvertiseSettings_AdvertiseMode {
  /** LOW_POWER - 1 second advertise interval */
  LOW_POWER = 'LOW_POWER',
  /** BALANCED - 250 ms advertise interval */
  BALANCED = 'BALANCED',
  /** LOW_LATENCY - 100 ms advertise interval */
  LOW_LATENCY = 'LOW_LATENCY',
  UNRECOGNIZED = 'UNRECOGNIZED',
}

/**
 * From
 * packages/modules/Bluetooth/framework/java/android/bluetooth/le/BluetoothLeAdvertiser.java#159
 */
export enum Chip_BluetoothBeacon_AdvertiseSettings_AdvertiseTxPower {
  /** ULTRA_LOW - -21 dBm */
  ULTRA_LOW = 'ULTRA_LOW',
  /** LOW - -15 dBm */
  LOW = 'LOW',
  /** MEDIUM - -7 dBm */
  MEDIUM = 'MEDIUM',
  /** HIGH - 1 dBm */
  HIGH = 'HIGH',
  UNRECOGNIZED = 'UNRECOGNIZED',
}

export interface Chip_BluetoothBeacon_AdvertiseData {
  /** Whether the device name should be included in advertise packet. */
  includeDeviceName: boolean;
  /**
   * Whether the transmission power level should be included in the
   * advertise packet.
   */
  includeTxPowerLevel: boolean;
  /** Manufacturer specific data. */
  manufacturerData: Uint8Array;
  /** GATT services supported by the devices */
  services: Chip_BluetoothBeacon_AdvertiseData_Service[];
}

export interface Chip_BluetoothBeacon_AdvertiseData_Service {
  uuid: string;
  data: Uint8Array;
}

export interface ChipCreate {
  kind: ChipKind;
  name: string;
  manufacturer: string;
  productName: string;
  bleBeacon?: ChipCreate_BluetoothBeaconCreate|undefined;
}

export interface ChipCreate_BluetoothBeaconCreate {
  address: string;
  settings: Chip_BluetoothBeacon_AdvertiseSettings|undefined;
  advData: Chip_BluetoothBeacon_AdvertiseData|undefined;
}

export interface Device {
  id: number;
  /** settable at creation */
  name: string;
  visible: State;
  position: Position|undefined;
  orientation:|Orientation|undefined;
  /** Device can have multiple chips of the same kind. */
  chips: Chip[];
}

export interface DeviceCreate {
  name: string;
  position: Position|undefined;
  orientation: Orientation|undefined;
  chips: ChipCreate[];
}

export interface Scene {
  devices: Device[];
}

export interface Capture {
  /** same as chip_id */
  id: number;
  chipKind: ChipKind;
  /** device AVD name */
  deviceName: string;
  /** capture state */
  state: State;
  /** size of current capture */
  size: number;
  /** number of records in current capture */
  records: number;
  timestamp: Date|undefined;
  valid: boolean;
}
