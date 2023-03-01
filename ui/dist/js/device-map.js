import{__decorate as e}from"../node_modules/tslib/tslib.es6.js";import{css as t,LitElement as i,html as o}from"https://cdn.jsdelivr.net/gh/lit/dist@2/core/lit-core.min.js";import{property as n,customElement as r}from"https://cdn.skypack.dev/pin/lit@v2.5.0-jYRq0AKQogjUdUh7SCAE/mode=imports/optimized/lit/decorators.js";import{styleMap as s}from"https://cdn.jsdelivr.net/gh/lit/dist@2/all/lit-all.min.js";import{simulationState as d}from"./device-observer.js";let a=class extends i{constructor(){super(...arguments),this.deviceData=[],this.imageIdx=0,this.numImages=3,this.isometric=!1,this.onChangeMap=()=>{this.imageIdx=(this.imageIdx+1)%this.numImages},this.handleIsometricView=()=>{this.isometric=!this.isometric}}connectedCallback(){super.connectedCallback(),d.registerObserver(this),window.addEventListener("map-button-clicked",this.onChangeMap),window.addEventListener("isometric-button-clicked",this.handleIsometricView)}disconnectedCallback(){window.removeEventListener("isometric-button-clicked",this.handleIsometricView),window.removeEventListener("map-button-clicked",this.onChangeMap),d.removeObserver(this),super.disconnectedCallback()}onNotify(e){this.deviceData=e.devices,this.requestUpdate()}render(){const e=["red","orange","yellow","green","blue","indigo","purple"],t=this.isometric?"perspective(200rem) rotateX(60deg) rotateY(0deg) rotateZ(0deg) scale3d(0.8,0.8,0.8); top: 250px":"none; top: 0px;";return o`
      <ns-device-dropzone>
        <div id="dropzone" class="box pattern${this.imageIdx}">
          ${this.deviceData.map(((t,i)=>{var n,r,d,a,l,c;return o`
              ${!0===t.visible?o`
                    <ns-device-dragzone
                      .action=${"move"}
                      style=${s({position:"absolute",left:100*(null!==(n=t.position.x)&&void 0!==n?n:0)+"px",top:100*(null!==(r=t.position.y)&&void 0!==r?r:0)+"px"})}
                    >
                      <ns-cube-sprite
                        id=${t.deviceSerial}
                        .color=${e[i%e.length]}
                        .size=${"30px"}
                        .controls=${!0}
                        yaw=${null!==(d=t.orientation.yaw)&&void 0!==d?d:0}
                        pitch=${null!==(a=t.orientation.pitch)&&void 0!==a?a:0}
                        roll=${null!==(l=t.orientation.roll)&&void 0!==l?l:0}
                        posZ=${100*(null!==(c=t.position.z)&&void 0!==c?c:0)}
                      ></ns-cube-sprite>
                    </ns-device-dragzone>
                  `:o``}
            `}))}
        </div>
        <style>
          #dropzone {
            transform: ${t};
          }
        </style>
      </ns-device-dropzone>
    `}};a.styles=t`
    #dropzone {
      margin-left: 200px;
      margin-right: 200px;
      transition: transform 2s, top 2s;
      transform-style: preserve-3d;
    }

    .box {
      position: relative;
      width: 1000px; //40vw;
      height: 1000px; //40vh;
      border: solid 1px rgb(198, 210, 255);
      margin: 2.5em auto;
    }

    .pattern0 {
      background-image: url(./assets/grid-background.svg);
    }

    .pattern1 {
      background-image: url(./assets/polar-background.svg);
      background-size: 1150px 1150px;
      background-position: center;
    }

    .pattern2 {
      background-image: url(./assets/hexagonal-background.png);
      background-size: 1175px 1175px;
      background-position: center;
    }

    .container {
      display: flex;
      width: 100%;
    }

    .contentA {
      flex: 2;
    }

    .contentB {
      flex: 2;
    }

    ns-device-dragzone {
      transform-style: inherit;
    }
  `,e([n()],a.prototype,"deviceData",void 0),e([n()],a.prototype,"imageIdx",void 0),e([n()],a.prototype,"numImages",void 0),e([n({type:Boolean,reflect:!0})],a.prototype,"isometric",void 0),a=e([r("ns-device-map")],a);export{a as DeviceMap};
