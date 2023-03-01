import{__decorate as e}from"../node_modules/tslib/tslib.es6.js";import{css as t,LitElement as n,html as i}from"https://cdn.jsdelivr.net/gh/lit/dist@2/core/lit-core.min.js";import{property as s,customElement as o}from"https://cdn.skypack.dev/pin/lit@v2.5.0-jYRq0AKQogjUdUh7SCAE/mode=imports/optimized/lit/decorators.js";let d=class extends n{constructor(){super(...arguments),this.viewMode="main",this.handleChangeModeEvent=e=>{const{detail:t}=e;this.viewMode=t.mode}}connectedCallback(){super.connectedCallback(),window.addEventListener("changeModeEvent",this.handleChangeModeEvent)}disconnectedCallback(){window.removeEventListener("changeModeEvent",this.handleChangeModeEvent),super.disconnectedCallback()}render(){let e=i``;return"main"===this.viewMode?e=i`
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
      `:"trace"===this.viewMode&&(e=i`
        <ns-packet-info></ns-packet-info>
      `),i`
      <ns-navigation-bar></ns-navigation-bar>
      ${e}
    `}};d.styles=t`
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
