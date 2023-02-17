import{__decorate as e}from"../node_modules/tslib/tslib.es6.js";import{css as n,LitElement as t,html as i}from"https://cdn.jsdelivr.net/gh/lit/dist@2/core/lit-core.min.js";import{property as s,customElement as o}from"https://cdn.skypack.dev/pin/lit@v2.5.0-jYRq0AKQogjUdUh7SCAE/mode=imports/optimized/lit/decorators.js";let d=class extends t{constructor(){super(...arguments),this.viewMode="main",this.handleChangeModeEvent=e=>{const{detail:n}=e;this.viewMode=n.mode}}connectedCallback(){super.connectedCallback(),window.addEventListener("changeModeEvent",this.handleChangeModeEvent)}disconnectedCallback(){window.removeEventListener("changeModeEvent",this.handleChangeModeEvent),super.disconnectedCallback()}render(){let e=i``;return"main"===this.viewMode?e=i`
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
      `:"trace"===this.viewMode?e=i`
        <ns-packet-info></ns-packet-info>
      `:"oslib"===this.viewMode&&(e=i`
        <ns-license-info></ns-license-info>
      `),i`
      <ns-navigation-bar></ns-navigation-bar>
      ${e}
    `}};d.styles=n`
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
  `,e([s()],d.prototype,"viewMode",void 0),d=e([o("netsim-app")],d);export{d as NetsimApp};
