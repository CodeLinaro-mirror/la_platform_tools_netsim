import{i as t,_ as e,s as d,y as a,e as i}from"./48895b41.js";import{e as o}from"./270e41ec.js";import{s as r}from"./a9eb0d1c.js";let l=class extends d{constructor(){super(...arguments),this.deviceData=[]}connectedCallback(){super.connectedCallback(),r.registerObserver(this)}disconnectedCallback(){r.removeObserver(this),super.disconnectedCallback()}onNotify(t){this.deviceData=t.devices,this.requestUpdate()}handleCapture(t){const e=t.target;r.updateCapture({deviceSerial:e.id,capture:e.checked}),this.requestUpdate()}render(){return a`
      <div class="panel">
        <div class="title">Packet Info</div>
        ${this.deviceData.map((t=>a`
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
                ${t.chips.map((t=>{var e,d,i,o,r,l,n,s;return t.bt?a`
                      <tr>
                        <td>BLE</td>
                        <td>N/A</td>
                        <td>N/A</td>
                        <td>${null!==(e=t.bt.lowEnergy.rxCount)&&void 0!==e?e:0}</td>
                        <td>${null!==(d=t.bt.lowEnergy.txCount)&&void 0!==d?d:0}</td>
                        <td>N/A</td>
                        <td>N/A</td>
                      </tr>
                      <tr>
                        <td>Bluetooth Classic</td>
                        <td>N/A</td>
                        <td>N/A</td>
                        <td>${null!==(i=t.bt.classic.rxCount)&&void 0!==i?i:0}</td>
                        <td>${null!==(o=t.bt.classic.txCount)&&void 0!==o?o:0}</td>
                        <td>N/A</td>
                        <td>N/A</td>
                      </tr>
                    `:t.uwb?a`
                      <tr>
                        <td>UWB</td>
                        <td>N/A</td>
                        <td>N/A</td>
                        <td>${null!==(r=t.uwb.rxCount)&&void 0!==r?r:0}</td>
                        <td>${null!==(l=t.uwb.txCount)&&void 0!==l?l:0}</td>
                        <td>N/A</td>
                        <td>N/A</td>
                      </tr>
                    `:t.wifi?a`
                      <tr>
                        <td>WIFI</td>
                        <td>N/A</td>
                        <td>N/A</td>
                        <td>${null!==(n=t.wifi.rxCount)&&void 0!==n?n:0}</td>
                        <td>${null!==(s=t.wifi.txCount)&&void 0!==s?s:0}</td>
                        <td>N/A</td>
                        <td>N/A</td>
                      </tr>
                    `:a``}))}
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
          ${this.deviceData.map((t=>a`
                <tr>
                  <td>${t.name}</td>
                  <td>${t.deviceSerial}</td>
                  <td>
                    ${t.chips.map((e=>e.bt?a`<input
                          id=${t.deviceSerial}
                          type="checkbox"
                          class="switch_1"
                          .checked=${"ON"===e.capture}
                          @click=${this.handleCapture}
                        />`:a``))}
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
    `}};l.styles=t`
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
  `,e([o()],l.prototype,"deviceData",void 0),l=e([i("ns-packet-info")],l);
