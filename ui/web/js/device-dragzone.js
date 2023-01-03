var DeviceDragZone_1;
import { __decorate } from "tslib";
import { LitElement, html, css } from 'lit';
import { customElement, property } from 'lit/decorators.js';
import { simulationState } from './device-observer.js';
let DeviceDragZone = DeviceDragZone_1 = class DeviceDragZone extends LitElement {
    constructor() {
        super();
        this.action = 'move';
        this.addEventListener('dragstart', this.handleDragStart);
        this.addEventListener('dragend', this.handleDragEnd);
        this.addEventListener('click', this.handleSelect);
    }
    connectedCallback() {
        this.draggable = true;
    }
    handleDragStart(ev) {
        this.style.opacity = '0.4';
        if (ev.dataTransfer && ev.target) {
            DeviceDragZone_1.dragged = ev.target;
            // eslint-disable-next-line no-param-reassign
            ev.dataTransfer.effectAllowed = this.action === 'move' ? 'move' : 'copy';
        }
    }
    handleDragEnd() {
        this.style.opacity = '1';
        DeviceDragZone_1.dragged = null;
    }
    // Allow the information panel to figure what has been selected.
    handleSelect(ev) {
        this.style.opacity = '1';
        if (ev.target) {
            simulationState.updateSelected(ev.target.id);
            // We can add a feature for visually showing a selected object (i.e. bolded borders)
        }
    }
    render() {
        return html ` <slot></slot> `;
    }
};
DeviceDragZone.styles = css `
    :host {
      cursor: move;
    }
  `;
__decorate([
    property({ type: String, attribute: 'action' })
], DeviceDragZone.prototype, "action", void 0);
DeviceDragZone = DeviceDragZone_1 = __decorate([
    customElement('ns-device-dragzone')
], DeviceDragZone);
export { DeviceDragZone };
//# sourceMappingURL=device-dragzone.js.map