import{__decorate as e}from"../node_modules/tslib/tslib.es6.js";import{css as i,LitElement as t,html as r}from"https://cdn.jsdelivr.net/gh/lit/dist@2/core/lit-core.min.js";import{property as s,customElement as o}from"https://cdn.skypack.dev/pin/lit@v2.5.0-jYRq0AKQogjUdUh7SCAE/mode=imports/optimized/lit/decorators.js";import{simulationState as n}from"./device-observer.js";let l=class extends t{constructor(){super(...arguments),this.deviceData=[]}connectedCallback(){super.connectedCallback(),n.registerObserver(this)}disconnectedCallback(){super.disconnectedCallback(),n.removeObserver(this)}onNotify(e){this.deviceData=e.devices,this.requestUpdate()}render(){const e=["red","orange","yellow","green","blue","indigo","purple"];return r`
      ${this.deviceData.map(((i,t)=>r`
          <li>
            <center>
              ${"visible"in i&&!0!==i.visible?r`<ns-device-dragzone action="move">
                      <ns-cube-sprite
                        id=${i.deviceSerial}
                        color=${e[t%e.length]}
                        size="30px"
                      ></ns-cube-sprite> </ns-device-dragzone
                    >${i.name}`:r`<ns-cube-sprite
                      id=${i.deviceSerial}
                      color=${e[t%e.length]}
                      size="30px"
                      style="opacity:0.5;"
                    ></ns-cube-sprite
                    >${i.name} `}
            </center>
          </li>
        `))}
      <li>
        <center>
          <ns-pyramid-sprite
            id="1234"
            color=${e[this.deviceData.length%e.length]}
            size="30px"
            style="opacity:0.5;"
          ></ns-pyramid-sprite
          >beacon
        </center>
      </li>
      <li>
        <center>
          <ns-pyramid-sprite
            id="5678"
            color=${e[(this.deviceData.length+1)%e.length]}
            size="30px"
            style="opacity:0.5;"
          ></ns-pyramid-sprite
          >anchor
        </center>
      </li>
    `}};l.styles=i`
    :host {
      justify-content: center;
      display: flex;
      flex-wrap: wrap;
      gap: 1rem;
      margin: 0;
      padding: 0;
      list-style: none;
    }

    li {
      border-style: solid;
      border-color: lightgray;
      flex-grow: 0;
      flex-shrink: 0;
      flex-basis: 125px;
    }

    li center {
      font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
      margin: 8px;
    }

    .box {
      position: relative;
      width: 80vw;
      height: 60vh;
      border: solid 1px rgb(198, 210, 255);
      margin: 2.5em auto;
    }
  `,e([s()],l.prototype,"deviceData",void 0),l=e([o("ns-device-list")],l);export{l as DeviceList};
