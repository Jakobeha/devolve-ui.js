[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)

# devolve-ui: fast JSX-based UI for browser (pixi.js) and terminal

**Live demos: standalone @ WIP, with CGE @ [https://cge.raycenity.net/examples/](https://cge.raycenity.net/examples/)**

Devolve-ui is a minimal UI and graphics engine for writing UI that works both in browser and on a terminal as a TUI. You can extend the browser UI with the full power of pixi.js to add effects that aren't rendered in the terminal. The terminal enables you to do fast debugging and testing, and also provides another build target.

Projects built with devolve-ui automatically run both on browser and node, with the platform detected at runtime and any platform-specific imports done dynamically. If you need to use platform-specific functionality, do so through one of the devolve-ui wrappers and provide a fallback for the other platform.

The goal of devolve-ui is to make writing UI much easier and faster to debug. The idea is that the core functionality of UI is very simple, and we separate that from the extra effects, animations, etc. devolve-ui also encourages loose coupling since effects are modular instead of being "mixed in" with the actual core UI.

Another use case of devolve-ui is that its applications can be run almost anywhere. Most devices have either a web-browser and a terminal. Futhermore the terminal UI might be a lot faster because it does not have to render extra effects.

Devolve-ui allows you to write graphics using JSX and react-like hooks. It also provides a simple API for creating browser-specific effects and animations (WIP).

## Installing

devolve-ui can be installed using [npm](https://www.npmjs.com/) or [pnpm](https://pnpm.io/).

```shell
pnpm install @raycenity/devolve-ui
```

Alternatively you can just download the built code directly [here](https://github.com/Jakobeha/devolve-ui/releases/latest). The code is an unminified ES module (learn about ES modules [here](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Guide/Modules))

## Repository info (e.g. for contributing)

devolve-ui is built using [esbuild](https://esbuild.org/). The package manager used is [pnpm](https://pnpm.io/). Linting is done by [standard](https://standardjs.com/), however we use a *slightly* modified version removing some warnings which is run through `pnpm run lint` (specifically `node ts-standardx.mjs`).
