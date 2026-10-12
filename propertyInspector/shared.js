/* Shared OpenDeck Property Inspector binding for focused DesignCraft actions. */
const DesignDeck = (() => {
  let socket, context, ids, defaults;
  const el = (id) => document.getElementById(id);
  const value = (node) => node.type === "checkbox" ? node.checked : node.type === "number" ? Number(node.value) || 0 : node.value.trim();
  const settings = () => Object.fromEntries(ids.map((id) => [id, value(el(id))]));
  const save = () => socket && context && socket.send(JSON.stringify({event:"setSettings",context,payload:settings()}));
  function bind(fields, initial) { ids = fields; defaults = initial; }
  function connectElgatoStreamDeckSocket(port, propertyInspectorUUID, registerEvent, info, actionInfo) {
    const action = JSON.parse(actionInfo); context = action.context; const saved = (action.payload && action.payload.settings) || {};
    ids.forEach((id) => { const node = el(id); const v = saved[id] ?? defaults[id] ?? (node.type === "checkbox" ? false : ""); if (node.type === "checkbox") node.checked = Boolean(v); else node.value = v; node.addEventListener(node.type === "checkbox" ? "change" : "input", save); });
    socket = new WebSocket(`ws://localhost:${port}`); socket.onopen = () => socket.send(JSON.stringify({event:registerEvent,uuid:propertyInspectorUUID}));
  }
  return {bind, connectElgatoStreamDeckSocket};
})();
window.connectElgatoStreamDeckSocket = DesignDeck.connectElgatoStreamDeckSocket;
