import{i as t,_ as e,s as a,y as r,e as s}from"./48895b41.js";import{e as d}from"./270e41ec.js";import{s as i}from"./1596eb10.js";var n;let o=n=class extends a{constructor(){super(),this.action="move",this.addEventListener("dragstart",this.handleDragStart),this.addEventListener("dragend",this.handleDragEnd),this.addEventListener("click",this.handleSelect)}connectedCallback(){this.draggable=!0}handleDragStart(t){this.style.opacity="0.4",t.dataTransfer&&t.target&&(n.dragged=t.target,t.dataTransfer.effectAllowed="move"===this.action?"move":"copy")}handleDragEnd(){this.style.opacity="1",n.dragged=null}handleSelect(t){this.style.opacity="1",t.target&&i.updateSelected(t.target.id)}render(){return r` <slot></slot> `}};o.styles=t`
    :host {
      cursor: move;
    }
  `,e([d({type:String,attribute:"action"})],o.prototype,"action",void 0),o=n=e([s("ns-device-dragzone")],o);export{o as D};
