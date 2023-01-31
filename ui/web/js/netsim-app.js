import { __decorate } from "tslib";
import { LitElement, html, css } from 'lit';
import { customElement, property } from 'lit/decorators.js';
let NetsimApp = class NetsimApp extends LitElement {
    constructor() {
        super(...arguments);
        /**
         * The view of the netsim app: main (map view), trace (packet trace view)
         */
        this.viewMode = 'main';
        this.handleChangeModeEvent = (e) => {
            const { detail } = e;
            this.viewMode = detail.mode;
        };
    }
    connectedCallback() {
        super.connectedCallback();
        window.addEventListener('changeModeEvent', this.handleChangeModeEvent);
    }
    disconnectedCallback() {
        window.removeEventListener('changeModeEvent', this.handleChangeModeEvent);
        super.disconnectedCallback();
    }
    render() {
        let page = html ``;
        if (this.viewMode === 'main') {
            page = html `
        <ns-customize-button eventName="map-button-clicked" class="primary">Change Background</ns-customize-button>
        <ns-customize-button eventName="isometric-button-clicked" class="primary">Toggle View</ns-customize-button>
        <div class="container">
          <div class="contentA">
            <ns-device-map></ns-device-map>
            <ns-device-list></ns-device-list>
          </div>
          <div class="contentB">
            <ns-device-info></ns-device-info>
          </div>
        </div>
      `;
        }
        else if (this.viewMode === 'trace') {
            page = html `
        <ns-packet-info></ns-packet-info>
      `;
        }
        return html `
      <ns-navigation-bar></ns-navigation-bar>
      ${page}
    `;
    }
};
NetsimApp.styles = css `
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
  `;
__decorate([
    property()
], NetsimApp.prototype, "viewMode", void 0);
NetsimApp = __decorate([
    customElement('netsim-app')
], NetsimApp);
export { NetsimApp };
//# sourceMappingURL=netsim-app.js.map