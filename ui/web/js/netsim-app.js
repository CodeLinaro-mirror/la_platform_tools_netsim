import { __decorate } from "tslib";
import { LitElement, html, css } from 'lit';
import { customElement, property } from 'lit/decorators.js';
let NetsimApp = class NetsimApp extends LitElement {
    constructor() {
        super(...arguments);
        this.title = '#betosim';
    }
    render() {
        return html `
      <main>
        <div class="logo"></div>
        <h1>${this.title}</h1>

        <p>edit <code>src/NetsimApp.ts</code> and save to reload.</p>

        <a
          class="app-link"
          href="../playground.html"
          target="_blank"
          rel="noopener noreferrer"
          >netsim Web UI</a
        >
      </main>

      <p class="app-footer">
        🚽 Made with love by
        <a
          target="_blank"
          rel="noopener noreferrer"
          href="https://github.com/open-wc"
          >open-wc</a
        >.
      </p>
    `;
    }
};
NetsimApp.styles = css `
    :host {
      min-height: 100vh;
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: flex-start;
      font-size: calc(10px + 2vmin);
      color: #1a2b42;
      max-width: 960px;
      margin: 0 auto;
      text-align: center;
      background-color: var(--netsim-app-background-color);
    }

    main {
      flex-grow: 1;
    }

    .logo {
      margin-top: 36px;
      animation: app-logo-two infinite 10s;
      background-repeat: no-repeat;
      margin-left: 25%;
      height: 349px;
    }

    @keyframes app-logo-two {
      0%,
      50% {
        background-image: url(../web/assets/netsim-logo.svg);
      }
      55%,
      60% {
        background-image: url(../web/assets/netsim-logo-b.svg);
      }
      65%,
      100% {
        background-image: url(../web/assets/netsim-logo.svg);
      }
    }

    @keyframes app-logo-spin {
      from {
        transform: rotate(0deg);
      }
      to {
        transform: rotate(360deg);
      }
    }

    .app-footer {
      font-size: calc(12px + 0.5vmin);
      align-items: center;
    }

    .app-footer a {
      margin-left: 5px;
    }
  `;
__decorate([
    property({ type: String })
], NetsimApp.prototype, "title", void 0);
NetsimApp = __decorate([
    customElement('netsim-app')
], NetsimApp);
export { NetsimApp };
//# sourceMappingURL=netsim-app.js.map