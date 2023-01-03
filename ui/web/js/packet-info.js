import { __decorate } from "tslib";
import { css, html, LitElement } from 'lit';
import { customElement, property } from 'lit/decorators.js';
import { simulationState, } from './device-observer.js';
let PacketInformation = class PacketInformation extends LitElement {
    constructor() {
        super(...arguments);
        /**
         * List of devices currently on the netsim.
         */
        this.deviceData = [];
    }
    connectedCallback() {
        super.connectedCallback(); // eslint-disable-line
        simulationState.registerObserver(this);
    }
    disconnectedCallback() {
        simulationState.removeObserver(this);
        super.disconnectedCallback(); // eslint-disable-line
    }
    onNotify(data) {
        this.deviceData = data.devices;
        this.requestUpdate();
    }
    handleCapture(ev) {
        const target = ev.target;
        simulationState.updateCapture({
            deviceSerial: target.id,
            capture: target.checked,
        });
        this.requestUpdate();
    }
    render() {
        return html `
      <div class="panel">
        <div class="title">Packet Info</div>
        ${this.deviceData.map(device => html `
              <div class="label">${device.name} | ${device.deviceSerial}</div>
              <table class="styled-table">
                <tr>
                  <th>Radio</th>
                  <th>Start-Time</th>
                  <th>End-Time</th>
                  <th>RX Count</th>
                  <th>TX Count</th>
                  <th>RX Bytes</th>
                  <th>TX Bytes</th>
                </tr>
                ${device.chips.map(chip => {
            var _a, _b, _c, _d, _e, _f, _g, _h;
            if (chip.bt) {
                return html `
                      <tr>
                        <td>BLE</td>
                        <td>N/A</td>
                        <td>N/A</td>
                        <td>${(_a = chip.bt.lowEnergy.rxCount) !== null && _a !== void 0 ? _a : 0}</td>
                        <td>${(_b = chip.bt.lowEnergy.txCount) !== null && _b !== void 0 ? _b : 0}</td>
                        <td>N/A</td>
                        <td>N/A</td>
                      </tr>
                      <tr>
                        <td>Bluetooth Classic</td>
                        <td>N/A</td>
                        <td>N/A</td>
                        <td>${(_c = chip.bt.classic.rxCount) !== null && _c !== void 0 ? _c : 0}</td>
                        <td>${(_d = chip.bt.classic.txCount) !== null && _d !== void 0 ? _d : 0}</td>
                        <td>N/A</td>
                        <td>N/A</td>
                      </tr>
                    `;
            }
            if (chip.uwb) {
                return html `
                      <tr>
                        <td>UWB</td>
                        <td>N/A</td>
                        <td>N/A</td>
                        <td>${(_e = chip.uwb.rxCount) !== null && _e !== void 0 ? _e : 0}</td>
                        <td>${(_f = chip.uwb.txCount) !== null && _f !== void 0 ? _f : 0}</td>
                        <td>N/A</td>
                        <td>N/A</td>
                      </tr>
                    `;
            }
            if (chip.wifi) {
                return html `
                      <tr>
                        <td>WIFI</td>
                        <td>N/A</td>
                        <td>N/A</td>
                        <td>${(_g = chip.wifi.rxCount) !== null && _g !== void 0 ? _g : 0}</td>
                        <td>${(_h = chip.wifi.txCount) !== null && _h !== void 0 ? _h : 0}</td>
                        <td>N/A</td>
                        <td>N/A</td>
                      </tr>
                    `;
            }
            return html ``;
        })}
              </table>
            `)}
        <div class="title">Packet Capture</div>
        <table class="styled-table">
          <tr>
            <th>Name</th>
            <th>Serial</th>
            <th>Capture ON/OFF</th>
            <th>Packet Trace</th>
          </tr>
          ${this.deviceData.map(device => html `
                <tr>
                  <td>${device.name}</td>
                  <td>${device.deviceSerial}</td>
                  <td>
                    ${device.chips.map(chip => {
            if (chip.bt) {
                return html `<input
                          id=${device.deviceSerial}
                          type="checkbox"
                          class="switch_1"
                          .checked=${chip.capture === 'ON'}
                          @click=${this.handleCapture}
                        />`;
            }
            return html ``;
        })}
                  </td>
                  <td>
                    <a
                      href="http://localhost:3000/${device.deviceSerial}-hci.pcap"
                      target="_blank"
                      type="application/vnd.tcpdump.pcap"
                      >Download PCAP</a
                    >
                  </td>
                </tr>
              `)}
        </table>
      </div>
    `;
    }
};
PacketInformation.styles = css `
    .panel {
      cursor: pointer;
      display: grid;
      place-content: center;
      color: black;
      font-size: 25px;
      font-family: 'Lato', sans-serif;
      border: 5px solid black;
      border-radius: 12px;
      padding: 10px;
      background-color: #ffffff;
      max-width: max-content;
      float: left;
    }

    .title {
      font-weight: bold;
      text-transform: uppercase;
      text-align: center;
      margin-bottom: 10px;
    }

    .label {
      text-align: left;
    }

    .styled-table {
      border-collapse: collapse;
      margin: 25px 0;
      font-size: 20px;
      font-family: sans-serif;
      width: 100%;
      box-shadow: 0 0 20px rgba(0, 0, 0, 0.15);
    }

    .styled-table thead tr {
      background-color: #009879;
      color: #ffffff;
      text-align: left;
    }

    .styled-table th,
    .styled-table td {
      padding: 12px 15px;
      text-align: left;
    }

    .styled-table tbody tr {
      border-bottom: 1px solid #dddddd;
    }

    .styled-table tbody tr:nth-of-type(even) {
      background-color: #cac0c0;
    }

    input[type='button'] {
      height: 2rem;
      font-size: inherit;
    }

    input[type='checkbox'].switch_1 {
      font-size: 30px;
      -webkit-appearance: none;
      -moz-appearance: none;
      appearance: none;
      width: 3.5em;
      height: 1.5em;
      background: #ddd;
      border-radius: 3em;
      position: relative;
      cursor: pointer;
      outline: none;
      -webkit-transition: all 0.2s ease-in-out;
      transition: all 0.2s ease-in-out;
    }

    input[type='checkbox'].switch_1:checked {
      background: #0ebeff;
    }

    input[type='checkbox'].switch_1:after {
      position: absolute;
      content: '';
      width: 1.5em;
      height: 1.5em;
      border-radius: 50%;
      background: #fff;
      -webkit-box-shadow: 0 0 0.25em rgba(0, 0, 0, 0.3);
      box-shadow: 0 0 0.25em rgba(0, 0, 0, 0.3);
      -webkit-transform: scale(0.7);
      transform: scale(0.7);
      left: 0;
      -webkit-transition: all 0.2s ease-in-out;
      transition: all 0.2s ease-in-out;
    }

    input[type='checkbox'].switch_1:checked:after {
      left: calc(100% - 1.5em);
    }
  `;
__decorate([
    property()
], PacketInformation.prototype, "deviceData", void 0);
PacketInformation = __decorate([
    customElement('ns-packet-info')
], PacketInformation);
export { PacketInformation };
//# sourceMappingURL=packet-info.js.map