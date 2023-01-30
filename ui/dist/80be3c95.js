import{i as e,_ as n,s as t,y as i,e as s}from"./48895b41.js";import{e as o}from"./270e41ec.js";let a=class extends t{constructor(){super(...arguments),this.viewMode="main",this.handleChangeModeEvent=e=>{const{detail:n}=e;this.viewMode=n.mode}}connectedCallback(){super.connectedCallback(),window.addEventListener("changeModeEvent",this.handleChangeModeEvent)}disconnectedCallback(){window.removeEventListener("changeModeEvent",this.handleChangeModeEvent),super.disconnectedCallback()}render(){let e=i``;return"main"===this.viewMode?e=i`
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
    `}};a.styles=e`
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
  `,n([o()],a.prototype,"viewMode",void 0),a=n([s("netsim-app")],a);
