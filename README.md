[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)

# devolve-ui: super simple reactive graphics for browser *and* terminal

devolve-ui is a super simple graphics library for games, canvas-based websites, TUIs, and JavaScript applications, which can deploy to both browser *and* terminal.

**Live demos: standalone @ [demos/index.html](demos/index.html), with CGE @ [https://cge.raycenity.net/demos/](https://cge.raycenity.net/demos/)**

```tsx
// demos/readme.tsx
import { DevolveUI, useState, useTimeout } from '@raycenity/devolve-ui'

const App = ({ name }) => {
  const [counter, setCounter] = useState(0)
  useTimeout(() => {
    setCounter(counter() + 1)
  }, 1000)

  return (
    <vbox sublayout={{ padding: 1, gap: 1 }}>
      <hbox sublayout={{ paddingX: 1, align: 'justify' }}>
        <text>Hello {name}</text>
        <text>{counter} seconds</text>
      </hbox>
      <graphic src='dog.png' bounds={Bounds({ width: '100%', height: 'auto' })} />
    </vbox>
  )
}

new DevolveUI(<App name='devolve-ui' />).start()

// Works in node or browser (with additional pixi.js script)
```

## Cross-platform

devolve-ui is *cross-platform* (isomorphic): a devolve-ui application may run in both web browsers and terminals (via node.js). When the application is run in the terminal, graphics are much simpler and certain effects and animations are removed, hence the name "devolve"-ui.

When a devolve-ui application is run in the web browser, it uses pixi.js for rendering.

## Super simple

devolve-ui uses JSX and React-style **components**: you write your UI declaratively and use hooks (useState, useEffect, useLazy, useInput) for local state and side-effects. Your UI is literally a function which takes the global state, and returns a render your application.

devolve-ui components return **nodes**, which make up the "virtual DOM" or "HTML" of your scene. Unlike real HTML there are 3 kinds of nodes: box, text, and graphic. Boxes contain children and define your layout, text contains styled (e.g. bold, colored) text, and graphics are solid backgrounds, gradients, images, videos, and custom pixi elements.

Every devolve-ui node has **bounds**, which define its position, size, and z-position (nodes with higher z-positions are rendered on top of nodes with lower z-positions). You create bounds using the function `Bounds`, e.g. `Bounds({ left: '32em', centerY: '50%', width: '250px' })`. The bounds system is very flexibld, so you can define custom layouts (see the section in [Implementation](#Bounds)).

### Implementation

devolve-ui has minimal dependencies and is lightweight relative to React. It is open source so you can [read the code  m yourself](https://github.com/Jakobeha/devolve-ui/tree/master/src)

#### Rendering

A component is essentially a function which takes the component's props and children and returns a node.

When the scene re-renders, devolve-ui calls each component function to reconstruct the nodes, reusing child components by matching them via their keys and function names, and preserving each component's state through hooks (which  are bound to the component).

Next, devolve-ui calculates each node's absolute bounding box by calling its `bounds`, using the parent node or scene's bounding box and sublayout. devolve-ui uses the position and x-position to determine the order it renders the nodes, and uses the size to affect how the node itself renders (wrapping text, scaling graphics).

Finally, devolve-ui draws each node onto the scene: in Terminal devolve-ui clears the display buffer and prints each node, in pixi.js it removes all DisplayObjects from the scene and re-adds them.

#### Bounds

Internally, every `bounds` value is actually by a function which takes the parent node's bounding
box and sublayout, and returns the node's absolute bounding box. This means that nodes can have absolute positions or z-positions regardless of their parents,  offsets and sizes which are percentages of the parents' size, margins, padding, gaps, and even completely custom layouts. In practice, you always create bounds using the `Bounds` function.

## Installing

devolve-ui can be installed using [npm](https://www.npmjs.com/) or [pnpm](https://pnpm.io/).

```shell
pnpm install @raycenity/devolve-ui
```

Alternatively you can just download the built code directly [here](https://github.com/Jakobeha/devolve-ui/releases/latest). The code is an unminified ES module (learn about ES modules [here](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Guide/Modules))

## Repository info (e.g. for contributing)

devolve-ui is built using [esbuild](https://esbuild.org/). The package manager used is [pnpm](https://pnpm.io/). Linting is done by [standard](https://standardjs.com/), however we use a *slightly* modified version removing some warnings which is run through `pnpm run lint` (specifically `node ts-standardx.mjs`).
