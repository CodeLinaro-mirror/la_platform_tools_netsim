import{i as e,_ as t,s as i,y as o,e as n}from"./48895b41.js";import{e as s}from"./270e41ec.js";import{i as r}from"./b6ffebd9.js";import{s as a}from"./a9eb0d1c.js";let d=class extends i{constructor(){super(...arguments),this.deviceData=[],this.imageIdx=0,this.numImages=3,this.isometric=!1,this.onChangeMap=()=>{this.imageIdx=(this.imageIdx+1)%this.numImages},this.handleIsometricView=()=>{this.isometric=!this.isometric}}connectedCallback(){super.connectedCallback(),a.registerObserver(this),window.addEventListener("map-button-clicked",this.onChangeMap),window.addEventListener("isometric-button-clicked",this.handleIsometricView)}disconnectedCallback(){window.removeEventListener("isometric-button-clicked",this.handleIsometricView),window.removeEventListener("map-button-clicked",this.onChangeMap),a.removeObserver(this),super.disconnectedCallback()}onNotify(e){this.deviceData=e.devices,this.requestUpdate()}render(){const e=["red","orange","yellow","green","blue","indigo","purple"],t=this.isometric?"perspective(200rem) rotateX(60deg) rotateY(0deg) rotateZ(0deg) scale3d(0.8,0.8,0.8); top: 250px":"none; top: 0px;";return o`
      <ns-device-dropzone>
        <div id="dropzone" class="box pattern${this.imageIdx}">
          ${this.deviceData.map(((t,i)=>{var n,s,a,d,c,p;return o`
              ${!0===t.visible?o`
                    <ns-device-dragzone
                      .action=${"move"}
                      style=${r({position:"absolute",left:100*(null!==(n=t.position.x)&&void 0!==n?n:0)+"px",top:100*(null!==(s=t.position.y)&&void 0!==s?s:0)+"px"})}
                    >
                      <ns-cube-sprite
                        id=${t.deviceSerial}
                        .color=${e[i%e.length]}
                        .size=${"30px"}
                        .controls=${!0}
                        yaw=${null!==(a=t.orientation.yaw)&&void 0!==a?a:0}
                        pitch=${null!==(d=t.orientation.pitch)&&void 0!==d?d:0}
                        roll=${null!==(c=t.orientation.roll)&&void 0!==c?c:0}
                        posZ=${100*(null!==(p=t.position.z)&&void 0!==p?p:0)}
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
    `}};d.styles=e`
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
  `,t([s()],d.prototype,"deviceData",void 0),t([s()],d.prototype,"imageIdx",void 0),t([s()],d.prototype,"numImages",void 0),t([s({type:Boolean,reflect:!0})],d.prototype,"isometric",void 0),d=t([n("ns-device-map")],d);
