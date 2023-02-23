import{__decorate as t}from"../node_modules/tslib/tslib.es6.js";import{css as e,LitElement as d,html as i}from"https://cdn.jsdelivr.net/gh/lit/dist@2/core/lit-core.min.js";import{property as a,customElement as o}from"https://cdn.skypack.dev/pin/lit@v2.5.0-jYRq0AKQogjUdUh7SCAE/mode=imports/optimized/lit/decorators.js";import{simulationState as r}from"./device-observer.js";let l=class extends d{constructor(){super(...arguments),this.deviceData=[]}connectedCallback(){super.connectedCallback(),r.registerObserver(this)}disconnectedCallback(){r.removeObserver(this),super.disconnectedCallback()}onNotify(t){this.deviceData=t.devices,this.requestUpdate()}handleCapture(t){const e=t.target;r.updateCapture({deviceSerial:e.id,capture:e.checked}),this.requestUpdate()}handleGetChips(t){var e,d,a,o,r,l,n,s;let c=i``,p=i``,h=i``;if("chips"in t&&t.chips)for(const b of t.chips){if("bt"in b&&b.bt){let t=i``,r=i``;"lowEnergy"in b.bt&&b.bt.lowEnergy&&(t=i`
              <tr>
                <td>BLE</td>
                <td>N/A</td>
                <td>N/A</td>
                <td>${null!==(e=b.bt.lowEnergy.rxCount)&&void 0!==e?e:0}</td>
                <td>${null!==(d=b.bt.lowEnergy.txCount)&&void 0!==d?d:0}</td>
                <td>N/A</td>
                <td>N/A</td>
              </tr>
            `),"classic"in b.bt&&b.bt.classic&&(r=i`
              <tr>
                <td>Bluetooth Classic</td>
                <td>N/A</td>
                <td>N/A</td>
                <td>${null!==(a=b.bt.classic.rxCount)&&void 0!==a?a:0}</td>
                <td>${null!==(o=b.bt.classic.txCount)&&void 0!==o?o:0}</td>
                <td>N/A</td>
                <td>N/A</td>
              </tr>
            `),c=i`${t} ${r}`}"uwb"in b&&b.uwb&&(p=i`
            <tr>
              <td>UWB</td>
              <td>N/A</td>
              <td>N/A</td>
              <td>${null!==(r=b.uwb.rxCount)&&void 0!==r?r:0}</td>
              <td>${null!==(l=b.uwb.txCount)&&void 0!==l?l:0}</td>
              <td>N/A</td>
              <td>N/A</td>
            </tr>
          `),"wifi"in b&&b.wifi&&(h=i`
            <tr>
              <td>WIFI</td>
              <td>N/A</td>
              <td>N/A</td>
              <td>${null!==(n=b.wifi.rxCount)&&void 0!==n?n:0}</td>
              <td>${null!==(s=b.wifi.txCount)&&void 0!==s?s:0}</td>
              <td>N/A</td>
              <td>N/A</td>
            </tr>
          `)}return i`
      ${c}
      ${p}
      ${h}
    `}render(){return i`
      <div class="panel">
        <div class="title">Packet Info</div>
        ${this.deviceData.map((t=>i`
              <div class="label">${t.name} | ${t.deviceSerial}</div>
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
                ${this.handleGetChips(t)}
              </table>
            `))}
        <div class="title">Packet Capture</div>
        <table class="styled-table">
          <tr>
            <th>Name</th>
            <th>Serial</th>
            <th>Capture ON/OFF</th>
            <th>Packet Trace</th>
          </tr>
          ${this.deviceData.map((t=>i`
                <tr>
                  <td>${t.name}</td>
                  <td>${t.deviceSerial}</td>
                  <td>
                    ${"chips"in t&&t.chips?t.chips.map((e=>e.bt?i`<input
                          id=${t.deviceSerial}
                          type="checkbox"
                          class="switch_1"
                          .checked=${"ON"===e.capture}
                          @click=${this.handleCapture}
                        />`:i``)):i``}
                  </td>
                  <td>
                    <a
                      href="http://localhost:3000/${t.deviceSerial}-hci.pcap"
                      target="_blank"
                      type="application/vnd.tcpdump.pcap"
                      >Download PCAP</a
                    >
                  </td>
                </tr>
              `))}
        </table>
      </div>
    `}};l.styles=e`
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
  `,t([a()],l.prototype,"deviceData",void 0),l=t([o("ns-packet-info")],l);export{l as PacketInformation};
