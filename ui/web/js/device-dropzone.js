var DeviceDropZone_1;
import { __decorate } from "tslib";
import { LitElement, html, css } from 'lit';
import { customElement, property } from 'lit/decorators.js';
import { DeviceDragZone } from './device-dragzone.js';
import { simulationState } from './device-observer.js';
let DeviceDropZone = DeviceDropZone_1 = class DeviceDropZone extends LitElement {
    constructor() {
        super();
        this.serial = '';
        this.type = '';
        this.addEventListener('drop', this.handleDrop);
        this.addEventListener('drag', this.handleDragOver);
        this.addEventListener('dragenter', DeviceDropZone_1.handleDragEnter);
        this.addEventListener('dragleave', DeviceDropZone_1.handleDragLeave);
        this.addEventListener('dragover', this.handleDragOver);
    }
    static handleDragEnter(ev) {
        ev.preventDefault();
    }
    static handleDragLeave(ev) {
        ev.preventDefault();
    }
    slottedDropZone() {
        var _a;
        // Returns the #dropzone div inside the slotted children, where devices are stored.
        // note: needs better checking when not the first element.
        const slot = (_a = this.shadowRoot) === null || _a === void 0 ? void 0 : _a.querySelector('slot');
        return slot === null || slot === void 0 ? void 0 : slot.assignedElements({ flatten: true })[0];
    }
    handleDrop(ev) {
        var _a, _b, _c, _d;
        ev.preventDefault();
        const dropzone = this.slottedDropZone();
        if (dropzone) {
            const draggedElement = DeviceDragZone.dragged;
            if (((_a = ev.dataTransfer) === null || _a === void 0 ? void 0 : _a.effectAllowed) === 'move') {
                (_b = draggedElement.parentNode) === null || _b === void 0 ? void 0 : _b.removeChild(draggedElement);
                draggedElement.style.opacity = '';
                dropzone.appendChild(draggedElement);
            }
            else {
                // copy
                dropzone.appendChild(draggedElement.cloneNode(true));
            }
            const dropped = dropzone.lastChild;
            if (dropped) {
                const rect = dropzone.getBoundingClientRect();
                dropped.setAttribute('action', 'move');
                dropped.style.position = 'absolute';
                dropped.style.left = `${ev.clientX - rect.left}px`;
                dropped.style.top = `${ev.clientY - rect.top}px`;
                dropped.style.opacity = `1.0`;
                // Update the position of a dropped element
                let serial = (_c = dropped
                    .getElementsByTagName('ns-cube-sprite')
                    .item(0)) === null || _c === void 0 ? void 0 : _c.getAttribute('id');
                if (serial === undefined) {
                    serial = (_d = dropped
                        .getElementsByTagName('ns-pyramid-sprite')
                        .item(0)) === null || _d === void 0 ? void 0 : _d.getAttribute('id');
                }
                if (serial === undefined || serial === null) {
                    serial = '';
                }
                simulationState.handleDrop(serial, (ev.clientX - rect.left) / 100, (ev.clientY - rect.top) / 100);
            }
        }
    }
    handleDragOver(ev) {
        ev.preventDefault();
        this.slottedDropZone();
    }
    render() {
        return html `<slot></slot>`;
    }
};
DeviceDropZone.styles = css `
    :host {
      cursor: move;
    }
  `;
__decorate([
    property({ type: String, attribute: 'serial' })
], DeviceDropZone.prototype, "serial", void 0);
__decorate([
    property({ type: String, attribute: 'type' })
], DeviceDropZone.prototype, "type", void 0);
DeviceDropZone = DeviceDropZone_1 = __decorate([
    customElement('ns-device-dropzone')
], DeviceDropZone);
export { DeviceDropZone };
//# sourceMappingURL=device-dropzone.js.map