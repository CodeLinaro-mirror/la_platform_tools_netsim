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

  private handleCapture(ev: InputEvent) {
    const target = ev.target as HTMLInputElement;
    simulationState.updateCapture({
      deviceSerial: target.id,
      capture: target.checked,
    });
    this.requestUpdate();
  }

  render() {
    return html`
      <div class="panel">
        <div class="title">Packet Info</div>
        ${this.deviceData.map(
          device =>
            html`
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
                  if (chip.bt) {
                    return html`
                      <tr>
                        <td>BLE</td>
                        <td>N/A</td>
                        <td>N/A</td>
                        <td>${chip.bt.lowEnergy.rxCount ?? 0}</td>
                        <td>${chip.bt.lowEnergy.txCount ?? 0}</td>
                        <td>N/A</td>
                        <td>N/A</td>
                      </tr>
                      <tr>
                        <td>Bluetooth Classic</td>
                        <td>N/A</td>
                        <td>N/A</td>
                        <td>${chip.bt.classic.rxCount ?? 0}</td>
                        <td>${chip.bt.classic.txCount ?? 0}</td>
                        <td>N/A</td>
                        <td>N/A</td>
                      </tr>
                    `;
                  }
                  if (chip.uwb) {
                    return html`
                      <tr>
                        <td>UWB</td>
                        <td>N/A</td>
                        <td>N/A</td>
                        <td>${chip.uwb.rxCount ?? 0}</td>
                        <td>${chip.uwb.txCount ?? 0}</td>
                        <td>N/A</td>
                        <td>N/A</td>
                      </tr>
                    `;
                  }
                  if (chip.wifi) {
                    return html`
                      <tr>
                        <td>WIFI</td>
                        <td>N/A</td>
                        <td>N/A</td>
                        <td>${chip.wifi.rxCount ?? 0}</td>
                        <td>${chip.wifi.txCount ?? 0}</td>
                        <td>N/A</td>
                        <td>N/A</td>
                      </tr>
                    `;
                  }
                  return html``;
                })}
              </table>
            `
        )}
        <div class="title">Packet Capture</div>
        <table class="styled-table">
          <tr>
            <th>Name</th>
            <th>Serial</th>
            <th>Capture ON/OFF</th>
            <th>Packet Trace</th>
          </tr>
          ${this.deviceData.map(
            device =>
              html`
                <tr>
                  <td>${device.name}</td>
                  <td>${device.deviceSerial}</td>
                  <td>
                    ${device.chips.map(chip => {
                      if (chip.bt) {
                        return html`<input
                          id=${device.deviceSerial}
                          type="checkbox"
                          class="switch_1"
                          .checked=${chip.capture === 'ON'}
                          @click=${this.handleCapture}
                        />`;
                      }
                      return html``;
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
              `
          )}
        </table>
      </div>
    `;
  }
}
