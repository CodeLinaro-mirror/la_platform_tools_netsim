var DeviceInformation_1;
import { __decorate } from "tslib";
import { css, html, LitElement } from 'lit';
import { customElement, property } from 'lit/decorators.js';
import { live } from 'lit/directives/live.js';
import { styleMap } from 'lit/directives/style-map.js';
import { simulationState, } from './device-observer.js';
let DeviceInformation = DeviceInformation_1 = class DeviceInformation extends LitElement {
    constructor() {
        super(...arguments);
        /**
         * the yaw value in orientation for ns-cube-sprite
         * unit: deg
         */
        this.yaw = 0;
        /**
         * the pitch value in orientation for ns-cube-sprite
         * unit: deg
         */
        this.pitch = 0;
        /**
         * the roll value in orientation for ns-cube-sprite
         * unit: deg
         */
        this.roll = 0;
        /**
         * The state of device info. True if edit mode.
         */
        this.editMode = false;
        this.posX = 0;
        this.posY = 0;
        this.posZ = 0;
    }
    connectedCallback() {
        super.connectedCallback(); // eslint-disable-line
        simulationState.registerObserver(this);
    }
    disconnectedCallback() {
        simulationState.removeObserver(this);
        super.disconnectedCallback(); // eslint-disable-line
    }
    onNotify(data) {
        var _a, _b, _c, _d, _e, _f;
        this.editMode = false;
        if (data.selectedSerial) {
            for (const device of data.devices) {
                if (device.deviceSerial === data.selectedSerial) {
                    this.selectedDevice = device;
                    this.yaw = (_a = device.orientation.yaw) !== null && _a !== void 0 ? _a : 0;
                    this.pitch = (_b = device.orientation.pitch) !== null && _b !== void 0 ? _b : 0;
                    this.roll = (_c = device.orientation.roll) !== null && _c !== void 0 ? _c : 0;
                    this.posX = Math.round(((_d = device.position.x) !== null && _d !== void 0 ? _d : 0) * 100);
                    this.posY = Math.round(((_e = device.position.y) !== null && _e !== void 0 ? _e : 0) * 100);
                    this.posZ = Math.round(((_f = device.position.z) !== null && _f !== void 0 ? _f : 0) * 100);
                    break;
                }
            }
        }
    }
    changeRange(ev) {
        var _a;
        console.assert(this.selectedDevice !== null); // eslint-disable-line
        const range = ev.target;
        const event = new CustomEvent('orientationEvent', {
            detail: {
                deviceSerial: (_a = this.selectedDevice) === null || _a === void 0 ? void 0 : _a.deviceSerial,
                type: range.id,
                value: range.value,
            },
        });
        window.dispatchEvent(event);
        if (range.id === 'yaw') {
            this.yaw = Number(range.value);
        }
        else if (range.id === 'pitch') {
            this.pitch = Number(range.value);
        }
        else {
            this.roll = Number(range.value);
        }
    }
    updateOrientation() {
        console.assert(this.selectedDevice !== undefined); // eslint-disable-line
        if (this.selectedDevice === undefined)
            return;
        simulationState.updateDevice({
            device: {
                deviceSerial: this.selectedDevice.deviceSerial,
                orientation: { yaw: this.yaw, pitch: this.pitch, roll: this.roll },
            },
        });
    }
    updateRadio() {
        console.assert(this.selectedDevice !== undefined); // eslint-disable-line
        if (this.selectedDevice === undefined)
            return;
        simulationState.updateDevice({
            device: {
                deviceSerial: this.selectedDevice.deviceSerial,
                chips: this.selectedDevice.chips,
            },
        });
    }
    handleEditForm() {
        this.editMode = !this.editMode;
    }
    static checkPositionBound(value) {
        return value > 10 ? 10 : value < 0 ? 0 : value; // eslint-disable-line
    }
    static checkOrientationBound(value) {
        return value > 90 ? 90 : value < -90 ? -90 : value; // eslint-disable-line
    }
    handleSave() {
        console.assert(this.selectedDevice !== undefined); // eslint-disable-line
        if (this.selectedDevice === undefined)
            return;
        const elements = this.renderRoot.querySelectorAll(`[id^="edit"]`);
        const obj = {
            deviceSerial: this.selectedDevice.deviceSerial,
            name: this.selectedDevice.name,
            position: this.selectedDevice.position,
            orientation: this.selectedDevice.orientation,
        };
        elements.forEach(element => {
            const inputElement = element;
            if (inputElement.id === 'editName') {
                obj.name = inputElement.value;
            }
            else if (inputElement.id.startsWith('editPos')) {
                if (!Number.isNaN(Number(inputElement.value))) {
                    obj.position[inputElement.id.slice(7).toLowerCase()] =
                        DeviceInformation_1.checkPositionBound(Number(inputElement.value) / 100);
                }
            }
            else if (inputElement.id.startsWith('editOri')) {
                if (!Number.isNaN(Number(inputElement.value))) {
                    obj.orientation[inputElement.id.slice(7).toLowerCase()] =
                        DeviceInformation_1.checkOrientationBound(Number(inputElement.value));
                }
            }
        });
        simulationState.updateDevice({
            device: obj,
        });
        this.handleEditForm();
    }
    render() {
        return html `${this.selectedDevice
            ? html `
          <div class="title">Device Info</div>
          <div class="setting">
            <div class="name">Name</div>
            <div class="info">
              ${this.editMode
                ? html `<input
                    type="text"
                    id="editName"
                    .value=${this.selectedDevice.name}
                  />`
                : html `${this.selectedDevice.name}`}
            </div>
          </div>
          <div class="setting">
            <div class="name">Serial</div>
            <div class="info">${this.selectedDevice.deviceSerial}</div>
          </div>
          <div class="setting">
            <div class="name">Position</div>
            <div class="label">X</div>
            <div class="info" style=${styleMap({ color: 'red' })}>
              ${this.editMode
                ? html `<input
                    type="text"
                    id="editPosX"
                    .value=${this.posX.toString()}
                  />`
                : html `${this.posX}`}
            </div>
            <div class="label">Y</div>
            <div class="info" style=${styleMap({ color: 'green' })}>
              ${this.editMode
                ? html `<input
                    type="text"
                    id="editPosY"
                    .value=${this.posY.toString()}
                  />`
                : html `${this.posY}`}
            </div>
            <div class="label">Z</div>
            <div class="info" style=${styleMap({ color: 'blue' })}>
              ${this.editMode
                ? html `<input
                    type="text"
                    id="editPosZ"
                    .value=${this.posZ.toString()}
                  />`
                : html `${this.posZ}`}
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
              ${this.editMode
                ? html `<input
                    type="text"
                    id="editOriYaw"
                    class="orientation"
                    .value=${this.yaw.toString()}
                  />`
                : html `<div class="text">(${this.yaw})</div>`}
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
              ${this.editMode
                ? html `<input
                    type="text"
                    id="editOriPitch"
                    class="orientation"
                    .value=${this.pitch.toString()}
                  />`
                : html `<div class="text">(${this.pitch})</div>`}
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
              ${this.editMode
                ? html `<input
                    type="text"
                    id="editOriRoll"
                    class="orientation"
                    .value=${this.roll.toString()}
                  />`
                : html `<div class="text">(${this.roll})</div>`}
            </div>
          </div>
          <div class="setting">
            ${this.editMode
                ? html `
                  <input type="button" value="Save" @click=${this.handleSave} />
                  <input
                    type="button"
                    value="Cancel"
                    @click=${this.handleEditForm}
                  />
                `
                : html `<input
                  type="button"
                  value="Edit"
                  @click=${this.handleEditForm}
                />`}
          </div>
          <div class="setting">
            <div class="name">Radio States</div>
            ${this.selectedDevice.chips.map(chip => chip.bt
                ? html `
                    <div class="label">BLE</div>
                    <div class="info">
                      <label class="switch">
                        <input
                          id="lowEnergy"
                          type="checkbox"
                          .checked=${live(chip.bt.lowEnergy.state === 'ON')}
                          @click=${() => {
                    // eslint-disable-next-line
                    chip.bt.lowEnergy.state =
                        chip.bt.lowEnergy.state === 'ON' ? 'OFF' : 'ON';
                    this.updateRadio();
                }}
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
                          .checked=${live(chip.bt.classic.state === 'ON')}
                          @click=${() => {
                    // eslint-disable-next-line
                    chip.bt.classic.state =
                        chip.bt.classic.state === 'ON' ? 'OFF' : 'ON';
                    this.updateRadio();
                }}
                        />
                        <span class="slider round"></span>
                      </label>
                    </div>
                  `
                : html ``)}
            <!--Hard coded and disabled Radio States-->
            <div class="label" style=${styleMap({ opacity: '0.7' })}>WIFI</div>
            <div class="info">
              <label class="switch">
                <input type="checkbox" disabled />
                <span
                  class="slider round"
                  style=${styleMap({ opacity: '0.7' })}
                ></span>
              </label>
            </div>
            <div class="label" style=${styleMap({ opacity: '0.7' })}>UWB</div>
            <div class="info">
              <label class="switch">
                <input type="checkbox" disabled />
                <span
                  class="slider round"
                  style=${styleMap({ opacity: '0.7' })}
                ></span>
              </label>
            </div>
            <div class="label" style=${styleMap({ opacity: '0.7' })}>
              WIFI_RTT
            </div>
            <div class="info">
              <label class="switch">
                <input type="checkbox" disabled />
                <span
                  class="slider round"
                  style=${styleMap({ opacity: '0.7' })}
                ></span>
              </label>
            </div>
          </div>
        `
            : html `<div class="title">Device Info</div>`}`;
    }
};
DeviceInformation.styles = css `
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
  `;
__decorate([
    property()
], DeviceInformation.prototype, "selectedDevice", void 0);
__decorate([
    property({ type: Number })
], DeviceInformation.prototype, "yaw", void 0);
__decorate([
    property({ type: Number })
], DeviceInformation.prototype, "pitch", void 0);
__decorate([
    property({ type: Number })
], DeviceInformation.prototype, "roll", void 0);
__decorate([
    property({ type: Boolean })
], DeviceInformation.prototype, "editMode", void 0);
DeviceInformation = DeviceInformation_1 = __decorate([
    customElement('ns-device-info')
], DeviceInformation);
export { DeviceInformation };
//# sourceMappingURL=device-info.js.map