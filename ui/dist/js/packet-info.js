import{__decorate as t}from"../node_modules/tslib/tslib.es6.js";import{css as e,LitElement as i,html as a}from"https://cdn.jsdelivr.net/gh/lit/dist@2/core/lit-core.min.js";import{property as o,customElement as d}from"https://cdn.skypack.dev/pin/lit@v2.5.0-jYRq0AKQogjUdUh7SCAE/mode=imports/optimized/lit/decorators.js";import{simulationState as r}from"./device-observer.js";import{State as s}from"./model.js";let n=class extends i{constructor(){super(...arguments),this.captureData=[],this.deviceData=[]}connectedCallback(){super.connectedCallback(),r.registerObserver(this)}disconnectedCallback(){r.removeObserver(this),super.disconnectedCallback()}onNotify(t){this.captureData=t.captures,this.deviceData=t.devices,this.requestUpdate()}toggleCapture(t){let e=t.id.toString(),i=t.state===s.OFF?"1":"2";r.patchCapture(e,i)}handleGetChips(t){var e,i,o,d,r,s,n,l;let c=a``,p=a``,h=a``;if("chips"in t&&t.chips)for(const b of t.chips){if("bt"in b&&b.bt){let t=a``,r=a``;"lowEnergy"in b.bt&&b.bt.lowEnergy&&(t=a`
              <tr>
                <td>BLE</td>
                <td>${null!==(e=b.bt.lowEnergy.rxCount)&&void 0!==e?e:0}</td>
                <td>${null!==(i=b.bt.lowEnergy.txCount)&&void 0!==i?i:0}</td>
              </tr>
            `),"classic"in b.bt&&b.bt.classic&&(r=a`
              <tr>
                <td>Bluetooth Classic</td>
                <td>${null!==(o=b.bt.classic.rxCount)&&void 0!==o?o:0}</td>
                <td>${null!==(d=b.bt.classic.txCount)&&void 0!==d?d:0}</td>
              </tr>
            `),c=a`${t} ${r}`}"uwb"in b&&b.uwb&&(p=a`
            <tr>
              <td>UWB</td>
              <td>${null!==(r=b.uwb.rxCount)&&void 0!==r?r:0}</td>
              <td>${null!==(s=b.uwb.txCount)&&void 0!==s?s:0}</td>
            </tr>
          `),"wifi"in b&&b.wifi&&(h=a`
            <tr>
              <td>WIFI</td>
              <td>${null!==(n=b.wifi.rxCount)&&void 0!==n?n:0}</td>
              <td>${null!==(l=b.wifi.txCount)&&void 0!==l?l:0}</td>
            </tr>
          `)}return a`
      ${c}
      ${p}
      ${h}
    `}handleListCaptures(t){return a`
      <tr>
        <td>${t.deviceName}</td>
        <td>${t.chipKind}</td>
        <td>${t.size}</td>
        <td>${t.records}</td>
        <td>
        <input
                type="checkbox"
                class="switch_1"
                .checked=${t.state===s.ON}
                @click=${()=>{this.toggleCapture(t)}}
              />
        </td>
        <td>
          <a
            href="./v1/captures/${t.id}"
            target="_blank"
            type="application/vnd.tcpdump.pcap"
            >Download</a
          >
        </td>
      </tr>
    `}render(){return a`
      <div class="panel">
        <div class="title">Packet Info</div>
        ${this.deviceData.map((t=>a`
              <div class="label">${t.name}</div>
              <table class="styled-table">
                <tr>
                  <th>Radio</th>
                  <th>RX Count</th>
                  <th>TX Count</th>
                </tr>
                ${this.handleGetChips(t)}
              </table>
            `))}
        <div class="title">Packet Capture</div>
        <table class="styled-table">
          <tr>
            <th>Device Name</th>
            <th>Chip Kind</th>
            <th>Size(bytes)</th>
            <th>Records</th>
            <th>Capture State</th>
            <th>Download Pcap</th>
          </tr>
          ${this.captureData.map((t=>this.handleListCaptures(t)))}
        </table>
      </div>
    `}};n.styles=e`
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
  `,t([o()],n.prototype,"captureData",void 0),t([o()],n.prototype,"deviceData",void 0),n=t([d("ns-packet-info")],n);export{n as PacketInformation};
