/**
 * Normalized user-visible UI model. Provider engine consumes this, not DOM internals.
 */

export function emptyPage() {
  return {
    url: "",
    title: "",
    authenticated: false,
    navigation: [],
    conversations: [],
    currentConversation: null,
    model: { current: null, available: [] },
    messages: [],
    composer: { input: null, send: null, stop: null, value: "" },
    generating: false,
    visibleText: "",
  };
}

export function snapshotFromTeaching(state, extras = {}) {
  return { ...emptyPage(), ...state, ...extras };
}
