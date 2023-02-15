import { __decorate } from "tslib";
import { LitElement, html, css } from 'lit';
import { customElement, property } from 'lit/decorators.js';
import { styleMap } from 'lit/directives/style-map.js';
import { simulationState, } from './device-observer.js';
let DeviceMap = class DeviceMap extends LitElement {
    constructor() {
        super(...arguments);
        /**
         * List of devices currently on the netsim.
         */
        this.deviceData = [];
        /**
         * Index of the background image displayed.
         */
        this.imageIdx = 0;
        /**
         * Number of images available for the background.
         */
        this.numImages = 3;
        this.isometric = false;
        this.onChangeMap = () => {
            this.imageIdx = (this.imageIdx + 1) % this.numImages;
        };
        this.handleIsometricView = () => {
            this.isometric = !this.isometric;
        };
    }
    connectedCallback() {
        super.connectedCallback(); // eslint-disable-line
        simulationState.registerObserver(this);
        window.addEventListener('map-button-clicked', this.onChangeMap);
        window.addEventListener('isometric-button-clicked', this.handleIsometricView);
    }
    disconnectedCallback() {
        window.removeEventListener('isometric-button-clicked', this.handleIsometricView);
        window.removeEventListener('map-button-clicked', this.onChangeMap);
        simulationState.removeObserver(this);
        super.disconnectedCallback(); // eslint-disable-line
    }
    onNotify(data) {
        this.deviceData = data.devices;
        this.requestUpdate();
    }
    render() {
        const rainbow = [
            'red',
            'orange',
            'yellow',
            'green',
            'blue',
            'indigo',
            'purple',
        ];
        const viewStyle = this.isometric
            ? `perspective(200rem) rotateX(60deg) rotateY(0deg) rotateZ(0deg) scale3d(0.8,0.8,0.8); top: 250px`
            : 'none; top: 0px;';
        return html `
      <ns-device-dropzone>
        <div id="dropzone" class="box pattern${this.imageIdx}">
          ${this.deviceData.map((device, idx) => {
            var _a, _b, _c, _d, _e, _f;
            return html `
              ${device.visible === true
                ? html `
                    <ns-device-dragzone
                      .action=${'move'}
                      style=${styleMap({
                    position: 'absolute',
                    left: `${((_a = device.position.x) !== null && _a !== void 0 ? _a : 0) * 100}px`,
                    top: `${((_b = device.position.y) !== null && _b !== void 0 ? _b : 0) * 100}px`,
                })}
                    >
                      <ns-cube-sprite
                        id=${device.deviceSerial}
                        .color=${rainbow[idx % rainbow.length]}
                        .size=${'30px'}
                        .controls=${true}
                        yaw=${(_c = device.orientation.yaw) !== null && _c !== void 0 ? _c : 0}
                        pitch=${(_d = device.orientation.pitch) !== null && _d !== void 0 ? _d : 0}
                        roll=${(_e = device.orientation.roll) !== null && _e !== void 0 ? _e : 0}
                        posZ=${((_f = device.position.z) !== null && _f !== void 0 ? _f : 0) * 100}
                      ></ns-cube-sprite>
                    </ns-device-dragzone>
                  `
                : html ``}
            `;
        })}
        </div>
        <style>
          #dropzone {
            transform: ${viewStyle};
          }
        </style>
      </ns-device-dropzone>
    `;
    }
};
DeviceMap.styles = css `
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
  `;
__decorate([
    property()
], DeviceMap.prototype, "deviceData", void 0);
__decorate([
    property()
], DeviceMap.prototype, "imageIdx", void 0);
__decorate([
    property()
], DeviceMap.prototype, "numImages", void 0);
__decorate([
    property({ type: Boolean, reflect: true })
], DeviceMap.prototype, "isometric", void 0);
DeviceMap = __decorate([
    customElement('ns-device-map')
], DeviceMap);
export { DeviceMap };
//# sourceMappingURL=device-map.js.map