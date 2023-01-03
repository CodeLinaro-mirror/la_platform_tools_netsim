import{x as e,b as i,i as t,_ as s,s as a,y as o,e as d}from"./48895b41.js";import{e as l}from"./270e41ec.js";import{e as n,a as c,t as r,i as p}from"./b6ffebd9.js";import{s as v}from"./1596eb10.js";const h={},u=n(class extends c{constructor(e){if(super(e),e.type!==r.PROPERTY&&e.type!==r.ATTRIBUTE&&e.type!==r.BOOLEAN_ATTRIBUTE)throw Error("The `live` directive is not allowed on child or event bindings");if(void 0!==e.strings)throw Error("`live` bindings can only contain a single expression")}render(e){return e}update(t,[s]){if(s===e||s===i)return s;const a=t.element,o=t.name;if(t.type===r.PROPERTY){if(s===a[o])return e}else if(t.type===r.BOOLEAN_ATTRIBUTE){if(!!s===a.hasAttribute(o))return e}else if(t.type===r.ATTRIBUTE&&a.getAttribute(o)===s+"")return e;return((e,i=h)=>{e._$AH=i})(t),s}});var b;let y=b=class extends a{constructor(){super(...arguments),this.yaw=0,this.pitch=0,this.roll=0,this.editMode=!1,this.posX=0,this.posY=0,this.posZ=0}connectedCallback(){super.connectedCallback(),v.registerObserver(this)}disconnectedCallback(){v.removeObserver(this),super.disconnectedCallback()}onNotify(e){var i,t,s,a,o,d;if(this.editMode=!1,e.selectedSerial)for(const l of e.devices)if(l.deviceSerial===e.selectedSerial){this.selectedDevice=l,this.yaw=null!==(i=l.orientation.yaw)&&void 0!==i?i:0,this.pitch=null!==(t=l.orientation.pitch)&&void 0!==t?t:0,this.roll=null!==(s=l.orientation.roll)&&void 0!==s?s:0,this.posX=Math.round(100*(null!==(a=l.position.x)&&void 0!==a?a:0)),this.posY=Math.round(100*(null!==(o=l.position.y)&&void 0!==o?o:0)),this.posZ=Math.round(100*(null!==(d=l.position.z)&&void 0!==d?d:0));break}}changeRange(e){var i;console.assert(null!==this.selectedDevice);const t=e.target,s=new CustomEvent("orientationEvent",{detail:{deviceSerial:null===(i=this.selectedDevice)||void 0===i?void 0:i.deviceSerial,type:t.id,value:t.value}});window.dispatchEvent(s),"yaw"===t.id?this.yaw=Number(t.value):"pitch"===t.id?this.pitch=Number(t.value):this.roll=Number(t.value)}updateOrientation(){console.assert(void 0!==this.selectedDevice),void 0!==this.selectedDevice&&v.updateDevice({device:{deviceSerial:this.selectedDevice.deviceSerial,orientation:{yaw:this.yaw,pitch:this.pitch,roll:this.roll}}})}updateRadio(){console.assert(void 0!==this.selectedDevice),void 0!==this.selectedDevice&&v.updateDevice({device:{deviceSerial:this.selectedDevice.deviceSerial,chips:this.selectedDevice.chips}})}handleEditForm(){this.editMode=!this.editMode}static checkPositionBound(e){return e>10?10:e<0?0:e}static checkOrientationBound(e){return e>90?90:e<-90?-90:e}handleSave(){if(console.assert(void 0!==this.selectedDevice),void 0===this.selectedDevice)return;const e=this.renderRoot.querySelectorAll('[id^="edit"]'),i={deviceSerial:this.selectedDevice.deviceSerial,name:this.selectedDevice.name,position:this.selectedDevice.position,orientation:this.selectedDevice.orientation};e.forEach((e=>{const t=e;"editName"===t.id?i.name=t.value:t.id.startsWith("editPos")?Number.isNaN(Number(t.value))||(i.position[t.id.slice(7).toLowerCase()]=b.checkPositionBound(Number(t.value)/100)):t.id.startsWith("editOri")&&(Number.isNaN(Number(t.value))||(i.orientation[t.id.slice(7).toLowerCase()]=b.checkOrientationBound(Number(t.value))))})),v.updateDevice({device:i}),this.handleEditForm()}render(){return o`${this.selectedDevice?o`
          <div class="title">Device Info</div>
          <div class="setting">
            <div class="name">Name</div>
            <div class="info">
              ${this.editMode?o`<input
                    type="text"
                    id="editName"
                    .value=${this.selectedDevice.name}
                  />`:o`${this.selectedDevice.name}`}
            </div>
          </div>
          <div class="setting">
            <div class="name">Serial</div>
            <div class="info">${this.selectedDevice.deviceSerial}</div>
          </div>
          <div class="setting">
            <div class="name">Position</div>
            <div class="label">X</div>
            <div class="info" style=${p({color:"red"})}>
              ${this.editMode?o`<input
                    type="text"
                    id="editPosX"
                    .value=${this.posX.toString()}
                  />`:o`${this.posX}`}
            </div>
            <div class="label">Y</div>
            <div class="info" style=${p({color:"green"})}>
              ${this.editMode?o`<input
                    type="text"
                    id="editPosY"
                    .value=${this.posY.toString()}
                  />`:o`${this.posY}`}
            </div>
            <div class="label">Z</div>
            <div class="info" style=${p({color:"blue"})}>
              ${this.editMode?o`<input
                    type="text"
                    id="editPosZ"
                    .value=${this.posZ.toString()}
                  />`:o`${this.posZ}`}
            </div>
          </div>
          <div class="setting">
            <div class="name">Orientation</div>
            <div class="label">Yaw</div>
            <div class="info">
              <input
                id="yaw"
                type="range"
                min="-90"
                max="90"
                .value=${this.yaw.toString()}
                .disabled=${this.editMode}
                @input=${this.changeRange}
                @change=${this.updateOrientation}
              />
              ${this.editMode?o`<input
                    type="text"
                    id="editOriYaw"
                    class="orientation"
                    .value=${this.yaw.toString()}
                  />`:o`<div class="text">(${this.yaw})</div>`}
            </div>
            <div class="label">Pitch</div>
            <div class="info">
              <input
                id="pitch"
                type="range"
                min="-90"
                max="90"
                .value=${this.pitch.toString()}
                .disabled=${this.editMode}
                @input=${this.changeRange}
                @change=${this.updateOrientation}
              />
              ${this.editMode?o`<input
                    type="text"
                    id="editOriPitch"
                    class="orientation"
                    .value=${this.pitch.toString()}
                  />`:o`<div class="text">(${this.pitch})</div>`}
            </div>
            <div class="label">Roll</div>
            <div class="info">
              <input
                id="roll"
                type="range"
                min="-90"
                max="90"
                .value=${this.roll.toString()}
                .disabled=${this.editMode}
                @input=${this.changeRange}
                @change=${this.updateOrientation}
              />
              ${this.editMode?o`<input
                    type="text"
                    id="editOriRoll"
                    class="orientation"
                    .value=${this.roll.toString()}
                  />`:o`<div class="text">(${this.roll})</div>`}
            </div>
          </div>
          <div class="setting">
            ${this.editMode?o`
                  <input type="button" value="Save" @click=${this.handleSave} />
                  <input
                    type="button"
                    value="Cancel"
                    @click=${this.handleEditForm}
                  />
                `:o`<input
                  type="button"
                  value="Edit"
                  @click=${this.handleEditForm}
                />`}
          </div>
          <div class="setting">
            <div class="name">Radio States</div>
            ${this.selectedDevice.chips.map((e=>e.bt?o`
                    <div class="label">BLE</div>
                    <div class="info">
                      <label class="switch">
                        <input
                          id="lowEnergy"
                          type="checkbox"
                          .checked=${u("ON"===e.bt.lowEnergy.state)}
                          @click=${()=>{e.bt.lowEnergy.state="ON"===e.bt.lowEnergy.state?"OFF":"ON",this.updateRadio()}}
                        />
                        <span class="slider round"></span>
                      </label>
                    </div>
                    <div class="label">Bluetooth Classic</div>
                    <div class="info">
                      <label class="switch">
                        <input
                          id="classic"
                          type="checkbox"
                          .checked=${u("ON"===e.bt.classic.state)}
                          @click=${()=>{e.bt.classic.state="ON"===e.bt.classic.state?"OFF":"ON",this.updateRadio()}}
                        />
                        <span class="slider round"></span>
                      </label>
                    </div>
                  `:o``))}
            <!--Hard coded and disabled Radio States-->
            <div class="label" style=${p({opacity:"0.7"})}>WIFI</div>
            <div class="info">
              <label class="switch">
                <input type="checkbox" disabled />
                <span
                  class="slider round"
                  style=${p({opacity:"0.7"})}
                ></span>
              </label>
            </div>
            <div class="label" style=${p({opacity:"0.7"})}>UWB</div>
            <div class="info">
              <label class="switch">
                <input type="checkbox" disabled />
                <span
                  class="slider round"
                  style=${p({opacity:"0.7"})}
                ></span>
              </label>
            </div>
            <div class="label" style=${p({opacity:"0.7"})}>
              WIFI_RTT
            </div>
            <div class="info">
              <label class="switch">
                <input type="checkbox" disabled />
                <span
                  class="slider round"
                  style=${p({opacity:"0.7"})}
                ></span>
              </label>
            </div>
          </div>
        `:o`<div class="title">Device Info</div>`}`}};y.styles=t`
    :host {
      cursor: pointer;
      display: grid;
      place-content: center;
      color: white;
      font-size: 25px;
      font-family: 'Lato', sans-serif;
      border: 5px solid black;
      border-radius: 12px;
      padding: 10px;
      background-color: #9199a5;
      max-width: 600px;
    }

    .title {
      font-weight: bold;
      text-transform: uppercase;
      text-align: center;
      margin-bottom: 10px;
    }

    .setting {
      display: grid;
      grid-template-columns: auto auto;
      margin-top: 0px;
      margin-bottom: 30px;
      //border: 3px solid black;
      padding: 10px;
    }

    .setting .name {
      grid-column: 1 / span 2;
      text-transform: uppercase;
      text-align: left;
      margin-bottom: 10px;
      font-weight: bold;
    }

    .label {
      grid-column: 1;
      text-align: left;
    }

    .info {
      grid-column: 2;
      text-align: right;
      margin-bottom: 10px;
    }

    .switch {
      position: relative;
      float: right;
      width: 60px;
      height: 34px;
    }

    .switch input {
      opacity: 0;
      width: 0;
      height: 0;
    }

    .slider {
      position: absolute;
      cursor: pointer;
      top: 0;
      left: 0;
      right: 0;
      bottom: 0;
      background-color: #ccc;
      -webkit-transition: 0.4s;
      transition: 0.4s;
    }

    .slider:before {
      position: absolute;
      content: '';
      height: 26px;
      width: 26px;
      left: 4px;
      bottom: 4px;
      background-color: white;
      -webkit-transition: 0.4s;
      transition: 0.4s;
    }

    input:checked + .slider {
      background-color: #2196f3;
    }

    input:focus + .slider {
      box-shadow: 0 0 1px #2196f3;
    }

    input:checked + .slider:before {
      -webkit-transform: translateX(26px);
      -ms-transform: translateX(26px);
      transform: translateX(26px);
    }

    /* Rounded sliders */
    .slider.round {
      border-radius: 34px;
    }

    .slider.round:before {
      border-radius: 50%;
    }

    .text {
      display: inline-block;
      position: relative;
      width: 50px;
    }

    input[type='range'] {
      width: 400px;
    }

    input[type='text'] {
      width: 50%;
      font-size: inherit;
      text-align: right;
      max-height: 25px;
    }

    input[type='text'].orientation {
      max-width: 50px;
    }

    input[type='button'] {
      display: inline;
      font-size: inherit;
      max-width: 200px;
    }
  `,s([l()],y.prototype,"selectedDevice",void 0),s([l({type:Number})],y.prototype,"yaw",void 0),s([l({type:Number})],y.prototype,"pitch",void 0),s([l({type:Number})],y.prototype,"roll",void 0),s([l({type:Boolean})],y.prototype,"editMode",void 0),y=b=s([d("ns-device-info")],y);
