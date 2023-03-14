import { css, html, LitElement } from 'lit';
import { customElement, property } from 'lit/decorators.js';
import {
  Device,
  Notifiable,
  SimulationInfo,
  simulationState,
} from './device-observer.js';

@customElement('ns-packet-info')
export class PacketInformation extends LitElement implements Notifiable {
  /**
   * List of devices currently on the netsim.
   */
  @property() deviceData: Device[] = [];

  static styles = css`
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

  connectedCallback() {
    super.connectedCallback(); // eslint-disable-line
    simulationState.registerObserver(this);
  }

  disconnectedCallback() {
    simulationState.removeObserver(this);
    super.disconnectedCallback(); // eslint-disable-line
  }

  onNotify(data: SimulationInfo) {
    this.deviceData = data.devices;
    this.requestUpdate();
  }

  private handleGetChips(device: Device) {
    let btTable = html``;
    let uwbTable = html``;
    let wifiTable = html``;
    if ("chips" in device && device.chips) {
      for (const chip of device.chips) {
        if ("bt" in chip && chip.bt) {
          let bleTable = html``;
          let bclassicTable = html``;
          if ("lowEnergy" in chip.bt && chip.bt.lowEnergy) {
            bleTable = html`
              <tr>
                <td>BLE</td>
                <td>${chip.bt.lowEnergy.rxCount ?? 0}</td>
                <td>${chip.bt.lowEnergy.txCount ?? 0}</td>
              </tr>
            `;
          }
          if ("classic" in chip.bt && chip.bt.classic) {
            bclassicTable = html`
              <tr>
                <td>Bluetooth Classic</td>
                <td>${chip.bt.classic.rxCount ?? 0}</td>
                <td>${chip.bt.classic.txCount ?? 0}</td>
              </tr>
            `;
          }
          btTable = html`${bleTable} ${bclassicTable}`;
        }
        if ("uwb" in chip && chip.uwb) {
          uwbTable = html`
            <tr>
              <td>UWB</td>
              <td>${chip.uwb.rxCount ?? 0}</td>
              <td>${chip.uwb.txCount ?? 0}</td>
            </tr>
          `;
        }
        if ("wifi" in chip && chip.wifi) {
          wifiTable = html`
            <tr>
              <td>WIFI</td>
              <td>${chip.wifi.rxCount ?? 0}</td>
              <td>${chip.wifi.txCount ?? 0}</td>
            </tr>
          `;
        }
      }
    }
    return html`
      ${btTable}
      ${uwbTable}
      ${wifiTable}
    `;
  }

  private handleGetCapture(device: Device) {
    let resultCapture = html``;
    if ('chips' in device && device.chips) {
      for (const chip of device.chips) {
        resultCapture = html`
          ${resultCapture}
          <tr>
            <td>${device.name}</td>
            <td>
              ${chip.bt ? "Bluetooth" : chip.uwb ? "UWB" : chip.wifi ? "WIFI" : "Unknown"}
            </td>
            <td>
              <input
                type="checkbox"
                class="switch_1"
                .checked=${chip.capture === 'ON'}
                @click=${() => {device.toggleCapture(device, chip);}}
              />
            </td>
            <td>
              <a
                href="http://localhost:7681/pcap/${device.name}"
                target="_blank"
                type="application/vnd.tcpdump.pcap"
                >Download PCAP</a
              >
            </td>
          </tr>
        `;
      }
    }
    return resultCapture;
  }

  render() {
    return html`
      <div class="panel">
        <div class="title">Packet Info</div>
        ${this.deviceData.map(
          device =>
            html`
              <div class="label">${device.name}</div>
              <table class="styled-table">
                <tr>
                  <th>Radio</th>
                  <th>RX Count</th>
                  <th>TX Count</th>
                </tr>
                ${this.handleGetChips(device)}
              </table>
            `
        )}
        <div class="title">Packet Capture</div>
        <table class="styled-table">
          <tr>
            <th>Name</th>
            <th>Chip Type</th>
            <th>Capture ON/OFF</th>
            <th>Packet Trace</th>
          </tr>
          ${this.deviceData.map(
            device => this.handleGetCapture(device)
          )}
        </table>
      </div>
    `;
  }
}
