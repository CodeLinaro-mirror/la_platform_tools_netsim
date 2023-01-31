import{i as e,_ as i,s as t,y as r,e as s}from"./48895b41.js";import{e as a}from"./270e41ec.js";import{s as n}from"./a9eb0d1c.js";let o=class extends t{constructor(){super(...arguments),this.deviceData=[]}connectedCallback(){super.connectedCallback(),n.registerObserver(this)}disconnectedCallback(){super.disconnectedCallback(),n.removeObserver(this)}onNotify(e){this.deviceData=e.devices,this.requestUpdate()}render(){const e=["red","orange","yellow","green","blue","indigo","purple"];return r`
      ${this.deviceData.map(((i,t)=>r`
          <li>
            <center>
              ${!0===i.visible?r`<ns-cube-sprite
                      id=${i.deviceSerial}
                      color=${e[t%e.length]}
                      size="30px"
                      style="opacity:0.5;"
                    ></ns-cube-sprite
                    >${i.name} `:r`<ns-device-dragzone action="move">
                      <ns-cube-sprite
                        id=${i.deviceSerial}
                        color=${e[t%e.length]}
                        size="30px"
                      ></ns-cube-sprite> </ns-device-dragzone
                    >${i.name}`}
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
    `}};o.styles=e`
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
  `,i([a()],o.prototype,"deviceData",void 0),o=i([s("ns-device-list")],o);
