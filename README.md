# devolve-ui.rs: React-like TUI and GUI library
devolve-ui is a *fast, platform-agnostic, reactive* TUI and GUI library in Rust.

devolve-ui applications can be written in Rust or JavaScript/TypeScript using the exposed WebAssembly bindings. The API is based on React but there are notable differences. The fundamental concepts are:

- **Views:** *Stateless* graphics and layout. Y layout boxes, text, images, rendered content, and filters are all views. You can create **compound views** which are simply a function that returns a tree of views. You can also create your own renderer and then define custom primitive views.
- **Components:** Graphics and UI elements with *state*, *effects*, and *event listeners*. You use **hooks** to define these, "effects" run at points in the component's lifecycle (e.g. on create, when input changes), "events" include time and input.
  - Components are defined by a function which takes a **component context** and some arbitrary input, and returns a tree of views and other components.
  - The body of the function may registers hooks and compute intermediate values from the input and state, but should not run "side effects"; instead put the side-effecting computations in the body of effect hooks. Though what you define as "side-effects" may be arbitrary, be aware that code in the component's body may be run at arbitrary times and possibly redundantly.
  - Every component has a **key** which identifies it among siblings in updates. This is like React, except you must always specify the key, even when the component is the only child. Components of different types must have different keys, and you cannot convert a component of one type into another by re-using it's key.
- **Context:** A context parameter (typically named `c`) is passed to component functions and closures in hooks. This context is needed to register hooks, access most types of state, and create child components.
- **Renderer:** The root of devolve-ui UI. The renderer stores a **render engine** which understands how to actually render the UI and receive input. It also stores the root component. The renderer manages global internals, including the **nonlocal update queue** and **view cache**.
- **Observable reference / ObsRef:** A value which tracks when it gets modified or its fields get modified - the specific modified field path is recorded so modifying a deeply-nested field doesn't update everything. When the ObsRef is modified, the components which last accessed that part of the ObsRef are updated.

## Design of your devolve-ui app
devolve-ui apps typically use a modification of the **MVP Design Pattern**. The "model" is the app's data, the "views" are just devolve-ui views, and the "presenters" are the devolve-ui components. Unlike in React, component-state is only for UI-specific state, and your app's state can be passed to the root component and modified in event handlers (also passed into the root) outside of the components.

However devolve-ui is very flexible and doesn't enforce design choices when it doesn't need to. If you want you can store the app state in the root component or other components, like in React.

## devolve-ui re-renders
React uses **diffing** to determine which components it need to re-render when props or state change. devolve-ui does not use diffing, and as such re-renders much more frequently. However, devolve-ui renders are also a lot faster, since it doesn't manage a DOM.

devolve-ui is intended to be a cross between immediate-mode and retained-mode. It doesn't have to re-compute rasters for views which are clearly not modified (determined by local states and ObsRef). But computing re-renders should still be fast as re-rendering is done liberally and in some cases, a full update and re-render may occur every frame.

## Rust API
Like Rust itself, devolve-ui's Rust API exposes some implementation details for implementation simplicity and performance, at the cost of verboseness and user-facing complexity. Which is to say: there are a lot of ways to do the same thing, some which are more performant but restricted and some which trigger updates in a slightly different order.

### Multi-threading and concurrency
devolve-ui is *not* multi-threaded: the renderer isn't thread-safe. However, you can interact with the renderer and devolve-ui component state on other threads. For example, `Renderer::resume_blocking_with_escape` runs the renderer completely blocking its thread, but provides a `NotifyFlag` which can be `set` on another thread to stop it. `AtomicRefState` can also be passed and set on other threads, and closures in hooks can still spawn other threads, so e.g. you can create an effect hook which loads an image in the background, and then after it's loaded displays the image on the component.

devolve-ui *is* concurrent. It uses tokio for its event loop, `Renderer::resume` is an `async fn` and, while it still blocks the current async block, allows parallel async blocks to run. If you don't want async that is completely fine as `Renderer::resume_blocking` works in a synchronous context, and you can even just call `Renderer::rerender` and/or `Renderer::poll` manually and run the event loop yourself.

### 4 types of state
This is one of those implantation details exposed for performance: devolve-ui exposes 4 types of state hooks: **local state, provided state, atomic-red state, and ObsRef state**

- **Local state:** State which can only be modified by this component (passing the state to child components doesn't work). Internally it's as an index into the component's local state vector.
- **Provided state:** State which can be modified by this component or child components, but not parent components or without a context. Internally it's an id into the component's provided state map, which gets passed to children
	- **Implicit provided state:** As the provided state id is a global constant, you can access and modify provided state from parents without the parents needing to pass it explicitly. This is like "contexts" in React, we use different naming because we already have different contexts.
	- **Explicit provided state:** If you don't want to make the state implicit, or if you have nested components of the same type (otherwise the top-most one will override), you can use explicit-provided state. Internally this is done using a local state which generates a unique provided state id, and then you pass that id to child components.
- **Atomic-ref state:** State which can be accessed and modified outside of the component without a context, and even on other threads. Internally this is a local state with an `Arc`, with wrappers so that the state is updated when a mutable dereference gets dropped. If done on another thread or outside of the component's update, this will put the component on the non-local update queue.
- **Tree-ref state:** State which can be accessed and modified outside the component without a context, but needs a context when accessed in a component to track which components to update. This is useful for large structures whose parts are going to be modified by child components, so you don't want to update the parent every time. There are atomic and non-atomic versions of this. You can also have a tree-ref at the root of your `Renderer` which can be accessed through the renderer directly (this is where you store your app's model if following MVP).

### Macros
devolve-ui in Rust is very verbose, so we have macros to help. You don't have to use any macros, and we try to provide clean alternatives, but they remove a lot of boilerplate.

`make_component`, `make_view`, and `make_hook` define helpers and boilerplate for your custom components, views, and hooks. `make_component` and `make_view` define structures for the arguments, functions, and macros, while `make_hook` defines an orphan trait so that your hook is a method on component contexts. Note that the actual function implementing your component, view, or hook is defined outside of the macro, so IDEs like IntelliJ still provide smart completion.

Views and components take optional arguments and required arguments. The syntax for view macros is `view_name!({ optional_arg: "foobar" }, required_arg1, required_arg2)` and component macros is `component_name!(c, key, { optional_arg: "foobar" }, required_arg1, required_arg2)` (you must still provide `{}` if you have no optional arguments).

### Shared mutable state
Everything in devolve-ui is stored under the renderer. The renderer is an `Rc` and there are references to it in each component, almost everything else including components are in unique memory. A component reference consists of a reference to the renderer and the component's **path**, a list of component-keys which tells you how to get to the component from the root. Component references are always weak and may resolve to null if the renderer was freed or the component at the specific path no longer exists, meaning it was deleted in an update.

### Component updates and the update queue
We update a component when it gets created, when it gets put in the non-local update queue and then the renderer gets polled, and when it gets a pending update (e.g. state change) during already being updated. Polling is done every frame, so basically a component gets updated when created and if it changes in the last frame. Except, one update may trigger cascading updates, so we must "update" the component multiple times during each real update.

Cascading updates are also called **local** updates, and the rest are **non-local**. Note that a local state may trigger a non-local update (e.g. in an event handler), and vice versa (e.g. setting an atomic-ref state in a component's body).

## TypeScript API (WIP)
devolve-ui's implementation is written in Rust as it's more performant, but it exposes a TypeScript API as TypeScript is less verbose and simpler. The TypeScript API is very similar to the Rust API, except instead of many implementation-specific options for performance, we expose only the least-performant but most-flexible option.

In devolve-ui for TypeScript there are no explicit contexts: when you register a hook, access a state, or create a component, devolve-ui uses the implicit context available only during a component function or effect closure call. You are responsible for not creating components or registering hooks in the wrong places, or using stale values.

You cannot currently define render engines for devolve-ui in TypeScript.