const api = globalThis.browser ?? globalThis.chrome;

api.action.onClicked.addListener((tab) => {
  Promise.resolve(api.tabs.sendMessage(tab.id, "toggle")).catch(() => {
    api.tabs.reload(tab.id);
  });
});
