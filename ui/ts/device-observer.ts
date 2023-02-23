// URL for netsim
const GET_DEVICES_URL = 'http://localhost:7681/get-devices';
const REGISTER_UPDATE_URL = 'http://localhost:7681/netsim/register-updates';
const UPDATE_DEVICE_URL = 'http://localhost:7681/netsim/update-device';
const SET_PACKET_CAPTURE_URL =
  'http://localhost:7681/netsim/set-packet-capture';

/**
 * Interface for a method in notifying the subscribed observers.
 * Subscribed observers must implement this interface.
 */
export interface Notifiable {
  onNotify(data: {}): void;
}

// TODO(b/255353541): import message interfaces in model.proto
interface Radio {
  state?: string;
  range?: number;
  txCount?: number;
  rxCount?: number;
}

interface Bluetooth {
  lowEnergy?: Radio;
  classic?: Radio;
}

interface Chip {
  chipId?: string;
  manufacturer?: string;
  model?: string;
  capture?: string;
  bt?: Bluetooth;
  uwb?: Radio;
  wifi?: Radio;
}

/**
 * Data structure of Device.
 * Used as a reference for subscribed observers to get proper attributes.
 */
export interface Device {
  deviceSerial: string;
  name?: string;
  position?: {
    x?: number;
    y?: number;
    z?: number;
  };
  orientation?: {
    yaw?: number;
    pitch?: number;
    roll?: number;
  };
  chips?: Chip[];
  visible?: boolean;
}

/**
 * The most updated state of the simulation.
 * Subscribed observers must refer to this info and update accordingly.
 */
export interface SimulationInfo {
  devices: Device[];
  selectedSerial: string;
  dimension: {
    x: number;
    y: number;
    z: number;
  };
}

interface Observable {
  registerObserver(elem: Notifiable): void;
  removeObserver(elem: Notifiable): void;
}

class SimulationState implements Observable {
  private observers: Notifiable[] = [];

  private simulationInfo: SimulationInfo = {
    devices: [],
    selectedSerial: '',
    dimension: { x: 10, y: 10, z: 0 },
  };

  constructor() {
    // initial GET
    fetch(GET_DEVICES_URL)
      .then(response => response.json())
      .then(data => {
        this.fetchDevice(data.devices);
      })
      .catch(error => {
        // eslint-disable-next-line
        console.log('Cannot connect to netsim web server', error);
      });
  }

  fetchDevice(devices: Device[]) {
    this.simulationInfo.devices = devices;
    this.notifyObservers();
  }

  updateSelected(serial: string) {
    this.simulationInfo.selectedSerial = serial;
    this.notifyObservers();
  }

  handleDrop(serial: string, x: number, y: number) {
    for (const device of this.simulationInfo.devices) {
      if (serial === device.deviceSerial) {
        if ("position" in device && device.position) {
          device.position.x = x;
          device.position.y = y;
        } else {
          device.position = {x: x, y: y, z: 0};
        }
        this.updateDevice({
          device: {
            deviceSerial: serial,
            position: device.position,
          },
        });
        break;
      }
    }
  }

  updateDevice(obj: object) {
    fetch(UPDATE_DEVICE_URL, {
      method: 'POST',
      body: JSON.stringify(obj),
    })
      .then(response => response.json())
      .catch(error => {
        // eslint-disable-next-line
        console.error('Error:', error);
      });
    this.notifyObservers();
  }

  updateCapture(obj: object) {
    fetch(SET_PACKET_CAPTURE_URL, {
      method: 'POST',
      body: JSON.stringify(obj),
    })
      .then(response => response.json())
      .catch(error => {
        // eslint-disable-next-line
        console.error('Error:', error);
      });
    this.notifyObservers();
  }

  registerObserver(elem: Notifiable) {
    this.observers.push(elem);
    elem.onNotify(this.simulationInfo)
  }

  removeObserver(elem: Notifiable) {
    const index = this.observers.indexOf(elem);
    this.observers.splice(index, 1);
  }

  notifyObservers() {
    for (const observer of this.observers) {
      observer.onNotify(this.simulationInfo);
    }
  }

  getDeviceList() {
    return this.simulationInfo.devices;
  }
}

/** Subscribed observers must register itself to the simulationState */
export const simulationState = new SimulationState();

async function subscribe() {
  // net::ERR_EMPTY_RESPONSE --> subscribe rightaway
  // net::ERR_CONNECTION_REFUSED --> subscribe after 15 seconds
  // eslint-disable-next-line
  let request = 0;
  let start = new Date().getTime();
  while (true) {
    await fetch(REGISTER_UPDATE_URL) // eslint-disable-line
      .then(response => response.json())
      .then(data => {
        simulationState.fetchDevice(data.devices);
      })
      .catch(error => {
        console.log(error); // eslint-disable-line
        request += 1;
      });
    // Send out Fail to connect when 3 requests fail in 1 second
    if (request >= 3) {
      if ((new Date().getTime() - start) < 1000) {
        alert("Failed to Connect to netsim")
        return;
      } else {
        request = 0;
        start = new Date().getTime();
      }
    }
  }
}

subscribe();
