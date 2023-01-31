// URL for netsim
const GET_DEVICES_URL = 'http://localhost:7681/netsim/get-devices';
const REGISTER_UPDATE_URL = 'http://localhost:7681/netsim/register-updates';
const UPDATE_DEVICE_URL = 'http://localhost:7681/netsim/update-device';
const SET_PACKET_CAPTURE_URL = 'http://localhost:7681/netsim/set-packet-capture';
class SimulationState {
    constructor() {
        this.observers = [];
        this.simulationInfo = {
            devices: [],
            selectedSerial: '',
            dimension: { x: 10, y: 10, z: 0 },
        };
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
    fetchDevice(devices) {
        this.simulationInfo.devices = devices;
        this.notifyObservers();
    }
    updateSelected(serial) {
        this.simulationInfo.selectedSerial = serial;
        this.notifyObservers();
    }
    handleDrop(serial, x, y) {
        for (const device of this.simulationInfo.devices) {
            if (serial === device.deviceSerial) {
                device.position.x = x;
                device.position.y = y;
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
    updateDevice(obj) {
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
    updateCapture(obj) {
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
    registerObserver(elem) {
        this.observers.push(elem);
        elem.onNotify(this.simulationInfo);
    }
    removeObserver(elem) {
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
                alert("Failed to Connect to netsim");
                return;
            }
            else {
                request = 0;
                start = new Date().getTime();
            }
        }
    }
}
subscribe();
//# sourceMappingURL=device-observer.js.map