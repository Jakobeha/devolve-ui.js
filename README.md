# devolve-ui.rs: Declarative "React" TUI and GUI in Rust

devolve-ui is a library for writing UI (TUI and GUI) using a React-like interface.

TODO code and demo

## Concepts

- **Views**: Primitive graphics and native controls.
- **Components**: User-defined UI elements. You define a component by writing a function which takes a **context** and props, and returns a tree of views and other components.

**Hooks**: Registered in components. These allow you to:
- define component state
- run side-effects during the components lifecycle (e.g. on creation)
- listen for events (time and input)

- `Renderer`: The root of devolve-ui

**Context**: A context parameter (often named `c`) is passed around most of the functions in devolve-ui. This is needed to retain Rust's lifetime invariants. You cannot pass contexts to effect closures, instead they get their own context as closure input. The state returned by hooks is just an index into the context, so it can be passed to effect closures - you use `state.get(c)` or `state.get_mut(c)` to get the real state. To pass contexts to other threads and time, you can get a "context ref" which is a path to the component, which you can retrieve later

The root of devolve-ui is a `Renderer`, and it's stored in an `Rc`. devolve-ui only runs on one thread but it can receive data from other threads, e.g. you can perform a background operation and then mutate a component's state with the result, which will show in the next re-render.

Components are stored in `Box`es. The root component is stored in the `Renderer`, and a component stores a map of its children. 