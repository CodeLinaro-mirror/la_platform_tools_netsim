import{__decorate as t}from"../node_modules/tslib/tslib.es6.js";import{css as e,LitElement as i,html as o}from"https://cdn.jsdelivr.net/gh/lit/dist@2/core/lit-core.min.js";import{property as a,customElement as n}from"https://cdn.skypack.dev/pin/lit@v2.5.0-jYRq0AKQogjUdUh7SCAE/mode=imports/optimized/lit/decorators.js";import{simulationState as d}from"./device-observer.js";import{State as l}from"./model.js";let r=class extends i{constructor(){super(...arguments),this.deviceData=[]}connectedCallback(){super.connectedCallback(),d.registerObserver(this)}disconnectedCallback(){d.removeObserver(this),super.disconnectedCallback()}onNotify(t){this.deviceData=t.devices,this.requestUpdate()}handleGetChips(t){var e,i,a,n,d,l,r,s;let c=o``,p=o``,b=o``;if("chips"in t&&t.chips)for(const h of t.chips){if("bt"in h&&h.bt){let t=o``,d=o``;"lowEnergy"in h.bt&&h.bt.lowEnergy&&(t=o`
              <tr>
                <td>BLE</td>
                <td>${null!==(e=h.bt.lowEnergy.rxCount)&&void 0!==e?e:0}</td>
                <td>${null!==(i=h.bt.lowEnergy.txCount)&&void 0!==i?i:0}</td>
              </tr>
            `),"classic"in h.bt&&h.bt.classic&&(d=o`
              <tr>
                <td>Bluetooth Classic</td>
                <td>${null!==(a=h.bt.classic.rxCount)&&void 0!==a?a:0}</td>
                <td>${null!==(n=h.bt.classic.txCount)&&void 0!==n?n:0}</td>
              </tr>
            `),c=o`${t} ${d}`}"uwb"in h&&h.uwb&&(p=o`
            <tr>
              <td>UWB</td>
              <td>${null!==(d=h.uwb.rxCount)&&void 0!==d?d:0}</td>
              <td>${null!==(l=h.uwb.txCount)&&void 0!==l?l:0}</td>
            </tr>
          `),"wifi"in h&&h.wifi&&(b=o`
            <tr>
              <td>WIFI</td>
              <td>${null!==(r=h.wifi.rxCount)&&void 0!==r?r:0}</td>
              <td>${null!==(s=h.wifi.txCount)&&void 0!==s?s:0}</td>
            </tr>
          `)}return o`
      ${c}
      ${p}
      ${b}
    `}handleGetCapture(t){let e=o``;if("chips"in t&&t.chips)for(const i of t.chips)e=o`
          ${e}
          <tr>
            <td>${t.name}</td>
            <td>
              ${i.bt?"Bluetooth":i.uwb?"UWB":i.wifi?"WIFI":"Unknown"}
            </td>
            <td>
              <input
                type="checkbox"
                class="switch_1"
                .checked=${i.capture===l.ON}
                @click=${()=>{t.toggleCapture(t,i)}}
              />
            </td>
            <td>
              <a
                href="http://localhost:7681/pcap/${t.name}"
                target="_blank"
                type="application/vnd.tcpdump.pcap"
                >Download PCAP</a
              >
            </td>
          </tr>
        `;return e}render(){return o`
      <div class="panel">
        <div class="title">Packet Info</div>
        ${this.deviceData.map((t=>o`
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
            <th>Name</th>
            <th>Chip Type</th>
            <th>Capture ON/OFF</th>
            <th>Packet Trace</th>
          </tr>
          ${this.deviceData.map((t=>this.handleGetCapture(t)))}
        </table>
      </div>
    `}};r.styles=e`
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
  `,t([a()],r.prototype,"deviceData",void 0),r=t([n("ns-packet-info")],r);export{r as PacketInformation};
