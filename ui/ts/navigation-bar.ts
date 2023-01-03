import { LitElement, html, css } from 'lit';
import { customElement } from 'lit/decorators.js';

@customElement('ns-navigation-bar')
export class NavigationBar extends LitElement {
  static styles = css`
    :host {
      --border-color: rgb(255, 255, 255, 0.1);
      --background-color: #747474;
    }

    .logo {
      animation: app-logo-two infinite 10s;
      background-repeat: no-repeat;
      margin-left: 25%;
      width: 50px;
      height: 50px;
    }

    @keyframes app-logo-two {
      0%,
      50% {
        background-image: url(./assets/netsim-logo.svg);
      }
      55%,
      60% {
        background-image: url(./assets/netsim-logo-b.svg);
      }
      65%,
      100% {
        background-image: url(./assets/netsim-logo.svg);
      }
    }

    nav {
      display: flex;
      width: 100%;
      border-bottom: 1px solid var(--border-color);
      background-color: var(--background-color);
      margin-bottom: 10px;
    }

    nav > .nav-section {
      padding: 3rem 2rem;
      display: flex;
      gap: 1rem;
      border-left: 1px solid var(--border-color);
      align-items: center;
      justify-content: center;
    }

    #nav-logo-section {
      justify-content: flex-start;
      flex-basis: calc(100% / 3);
    }

    #nav-link-section {
      flex-basis: calc(100% / 3);
      gap: 6rem;
    }

    #nav-contact-section {
      flex-grow: 1;
    }

    a {
      text-decoration: none;
    }

    h1,
    h2,
    h3,
    a,
    p,
    span {
      font-family: 'Lato';
      font-weight: bold;
      color: white;
      font-size: 20px;
    }
  `;

  connectedCallback() {
    super.connectedCallback(); // eslint-disable-line
  }

  disconnectedCallback() {
    super.disconnectedCallback(); // eslint-disable-line
  }

  render() {
    return html`
      <nav>
        <div id="nav-logo-section" class="nav-section">
          <a href=".">
            <div class="logo"></div>
          </a>
          <p>#betosim</p>
        </div>
        <div id="nav-link-section" class="nav-section">
          <a href="http://go/betosim" target="_blank" rel="noopener noreferrer"
            >ABOUT</a
          >
          <a href="./packet.html" target="_blank" rel="noopener noreferrer"
            >PACKET TRACE</a
          >
        </div>
        <div id="nav-contact-section" class="nav-section">
          <a
            href="https://team.git.corp.google.com/betosim/web"
            target="_blank"
            rel="noopener noreferrer"
            >DOCUMENTATION</a
          >
        </div>
      </nav>
    `;
  }
}
