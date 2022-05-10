# Model in UI

How do you represent state in UI? It's an age-old question.

In devolve-ui, there are 2 types of state:

- **App state:* Application state AKA model
- **UI state:** UI-specific state (e.g. is a button pressed? Is there an animation in progress?)

And 4 ways to create and pass state:

- Through `props` (app state and UI state; explicitly passed from parent to child)
- Through `useState` (UI state only, passed from parent to child's `props`)
- **Props contexts**: `createPropsContext, propsContext.useProvide, propsContext.useConsume` (app state and UI state; implicitly passed from parent to its children)
- **State contexts**: `createStateContext, stateContext.useProvide, stateContext.useConsume, stateContext.useConsumeRoot` (UI state only; implicitly passed from parent to its children)

## Why 2 types of state?

UI is literally "user interface": it's a way for users to view and mutate your app's model[1]. It's kind of a stretch, but I believe UI is essentially a glorified editor or ["inspector"](https://docs.unity3d.com/Manual/UsingTheInspector.html) for the model, which is especially easy to use and prevents you from breaking the model's invariants (e.g. in an FPS, those invariants would be "you can't break the rules and just fly around everywhere, or add random stuff to the world, or crash the 3D renderer with bad frame data"). Those invariants are maintained by only giving the UI capabilities to mutate the model which it needs, like [Redux actions](https://redux.js.org/basics/actions) or [Elm messages](https://guide.elm-lang.org/architecture/).

However, the UI actually needs to store state which we wouldn't consider part of the app's "model" at all. For instance, buttons being pressed, intermediate text input, animations which are in progress. This info is specific to the UI and view and IMO should only be exposed to the UI components[2]. This is what "state" is [in React](https://reactjs.org/docs/state-and-lifecycle.html) and also devolve-ui.

So there are 2 "types" of UI state: the model's state (**app state**), and the view's internal state (**UI state**).

## Why 4 ways to pass state?

But there is also the question of how to pass this state from parents to children. If you pass the state explicitly, you end up with a lot of redundancy (e.g. passing the "currently focused" variable to every UI element). Therefore, devolve-ui allows you to pass state implicitly through **contexts**, which function similarly to [React contexts](https://reactjs.org/docs/context.html).

devolve-ui has separate contexts for props and state because of the way state changes are implemented. When a state context is changed, all components which use that context (including the one which provides it) must be updated. This isn't necessary for props contexts which store state from `props`, or state explicitly passed to a child's `props`, because modifying `props` already causes the component and its ~children[3] to update.

---

[1] In a tabletop game the model is the game board, current turn, etc., in JIRA the model is all of your JIRA tickets, etc., in a TODO list the model is the TODOs.

[2] This is one of my main issues with [Elm](https://elm-lang.org/). In Elm, basic HTML components like buttons, text fields, etc. store internal state, but you can't create custom components without unnecessarily exposing the state (see [this package](https://package.elm-lang.org/packages/thebritican/elm-autocomplete/latest/) for an example). I think Elm has some really amazing ideas, but is hurt by design decisions like this (and also manually parsing JSON)

[3] Actually only causes children which are created in the `VComponent`'s body, not any which are passed as `children` via `props` to the `VComponent`. But `props` is already being modified so this isn't a problem.
