# Design decisions
devolve-ui is a very new and unstable library so it's API and design decisions are subject to change. Please be aware of this if you intend to use devolve-ui in production-grade software.

## Simple and extensible
devolve-ui is designed to be simple and extensible. We generally don't provide functionality if it introduces unnecessary complexity. Nearly all abstractions are leaky, so not only is the API simple, so is the implementation.

When adding a feature, either a) add it so that it doesn't interfere with existing features and code, or b) make a small change to the core library making it more extensible *ideally not affecting the API*, and then do a.

Furthermore, if any one feature becomes too large, especially if it significantly impacts size or performance, move it into a separate package.

## Encourage good design and discourage confusing bugs
Make it easy to add new small components. Also automatically useDynamicFn in hooks to avoid the [React Stale Closure Problem](https://stackoverflow.com/questions/62806541/how-to-solve-the-react-hook-closure-issue) as much as possible.

## Isomorphic, easy to use
Configuring node.js packages and avoid confusing JavaScript transpile errors is honestly very difficult. devolve-ui is isomorphic, is an es module, and should be as easy to use as possible. We provide examples so you can literally clone the repository, remove the example code, and already you have all the boilerplate ready and can start building your site or CLI app.
