# devolve-ui: write one reactive UI for browser and terminal

Devolve-ui is a super minimal UI engine for writing a basic UI that works both in browser and on a terminal as a TUI. You can then extend the browser UI with the full suite of DOM tools to add extra animations, effects, etc.

The goal is to reduce complexity and simplify testing and debugging for UI. The idea is that the common UI will be very simple, but contain most of the ultimate functionality - the browser UI extras will be small and isolated. So devolve-ui encourages better organization and loose coupling.

The other goal is to make applications which can be run almost anywhere, since almost every system either supports a terminal or a web browser.

The UI is written using JSX and react-like hooks. Both the browser and terminal are powered by `solid-js`.
