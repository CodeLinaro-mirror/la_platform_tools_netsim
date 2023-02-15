import { __decorate } from "tslib";
import { LitElement, html, css } from 'lit';
import { customElement, property } from 'lit/decorators.js';
import { simulationState, } from './device-observer.js';
let CubeSprite = class CubeSprite extends LitElement {
    constructor() {
        super(...arguments);
        /**
         * the yaw value in orientation for ns-cube-sprite
         * unit: deg
         */
        this.yaw = -15;
        /**
         * the pitch value in orientation for ns-cube-sprite
         * unit: deg
         */
        this.pitch = -15;
        /**
         * the roll value in orientation for ns-cube-sprite
         * unit: deg
         */
        this.roll = 0;
        /**
         * the z value in position for ns-cube-sprite
         * unit: cm
         */
        this.posZ = 0;
        /**
         * the css value for color
         */
        this.color = css `red`;
        /**
         * the css value for size
         */
        this.size = css `30px`;
        /**
         * A Boolean property; if set true, the user would
         * be able to control the cube's pitch, yaw, and roll
         * with the info panel.
         */
        this.controls = false;
        /**
         * A Boolean property; if set true, the box is selected
         * therefore the outline gets dotted.
         */
        this.highlighted = false;
        this.handleOrientationEvent = (e) => {
            const { detail } = e;
            if (detail.deviceSerial === this.id && this.controls) {
                if (detail.type === 'yaw') {
                    this.yaw = detail.value;
                }
                else if (detail.type === 'pitch') {
                    this.pitch = detail.value;
                }
                else {
                    this.roll = detail.value;
                }
            }
        };
    }
    connectedCallback() {
        super.connectedCallback(); // eslint-disable-line
        simulationState.registerObserver(this);
        window.addEventListener('orientationEvent', this.handleOrientationEvent);
    }
    disconnectedCallback() {
        window.removeEventListener('orientationEvent', this.handleOrientationEvent);
        simulationState.removeObserver(this);
        super.disconnectedCallback(); // eslint-disable-line
    }
    onNotify(data) {
        var _a;
        this.highlighted = data.selectedSerial === this.id;
        for (const device of data.devices) {
            if (device.deviceSerial === this.id) {
                this.posZ = ((_a = device.position.z) !== null && _a !== void 0 ? _a : 0) * 100;
                return;
            }
        }
    }
    render() {
        // TODO(b/255635486): Make cube easily scalable with user input size
        return html `
      <style>
        :host {
          font-size: ${this.size};
          --color: ${this.color};
          --yaw: ${this.yaw};
          --pitch: ${this.pitch};
          --roll: ${this.roll};
          --posZ: ${this.controls ? this.posZ : 0};
        }
        .cube > div {
          outline: ${this.highlighted && this.controls ? css `dashed` : css ``};
        }
      </style>
      <div class="cube">
        <div></div>
        <div></div>
        <div></div>
        <div></div>
        <div></div>
        <div></div>
      </div>
      ${this.controls
            ? html `
            <div class="line"></div>
            <div class="base"></div>
          `
            : html ``}
    `;
    }
};
CubeSprite.styles = css `
    :host {
      /** all sizes are relative to font-size **/
      display: block;
      min-height: 1.5em;
      min-width: 1.5em;
      width: 1em;
      /*  overflow: hidden; */
      transform-origin: center;
      transform-style: preserve-3d;
      transform: translateZ(calc(var(--posZ) * 1px));
    }

    .cube {
      transform-style: preserve-3d;
      transform: rotateX(calc(var(--yaw) * 1deg))
        rotateY(calc(var(--pitch) * 1deg)) rotateZ(calc(var(--roll) * 1deg));
      position: absolute;
      left: 0.25em;
      bottom: 0.25em;
      width: 1em;
      height: 1em;
    }
    .cube > div {
      position: absolute;
      background-color: var(--color);
      width: 100%;
      height: 100%;
      box-shadow: 0 0 0.25em #000 inset;
    }
    .cube > div:nth-child(1) {
      transform: translateZ(0.5em);
    }
    .cube > div:nth-child(2) {
      transform: rotateY(180deg) translateZ(0.5em);
    }
    .cube > div:nth-child(3) {
      right: 0;
      width: 1em;
      transform: rotateY(90deg) translateZ(0.5em);
    }
    .cube > div:nth-child(4) {
      width: 1em;
      transform: rotateY(270deg) translateZ(0.5em);
    }
    .cube > div:nth-child(5) {
      bottom: -0.5em;
      height: 1em;
      transform: rotateX(90deg);
      box-shadow: 0 0 0.25em #000 inset, 0 0 0.25em #000;
    }
    .cube div:nth-child(6) {
      height: 1em;
      transform: translateY(-0.5em) rotateX(90deg);
      overflow: hidden;
    }

    .line {
      position: absolute;
      border-bottom: 5px dashed;
      width: calc(var(--posZ) * 1px);
      top: 50%;
      left: 50%;
      transform: rotateY(90deg) rotateX(90deg);
      transform-origin: left;
    }

    .base {
      position: absolute;
      border: 5px solid;
      border-radius: 50%;
      background-color: black;
      height: 5px;
      width: 5px;
      top: 50%;
      left: 50%;
      transform: translate3d(-50%, -50%, calc(var(--posZ) * -1px));
    }
  `;
__decorate([
    property({ type: Number })
], CubeSprite.prototype, "yaw", void 0);
__decorate([
    property({ type: Number })
], CubeSprite.prototype, "pitch", void 0);
__decorate([
    property({ type: Number })
], CubeSprite.prototype, "roll", void 0);
__decorate([
    property({ type: Number })
], CubeSprite.prototype, "posZ", void 0);
__decorate([
    property({ type: css, attribute: 'color' })
], CubeSprite.prototype, "color", void 0);
__decorate([
    property({ type: css, attribute: 'size' })
], CubeSprite.prototype, "size", void 0);
__decorate([
    property({ type: Boolean })
], CubeSprite.prototype, "controls", void 0);
__decorate([
    property({ type: Boolean })
], CubeSprite.prototype, "highlighted", void 0);
CubeSprite = __decorate([
    customElement('ns-cube-sprite')
], CubeSprite);
export { CubeSprite };
//# sourceMappingURL=cube-sprite.js.map