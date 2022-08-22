# devolve-ui: composable, modular, rapidly-iterable UI

**Notice:** this is a proof-of-concept, subject to change and many parts are unimplemented.

## What?

devolve-ui is a UI library which integrates with existing UI libraries such as [egui](https://docs.rs/egui/latest) and [druid](https://docs.rs/druid/latest). devolve-ui provides a way to compose simple widgets implemented in these libraries into full applications, in a way that is modular and supports near-instant reloads and previews.

devolve-ui has the following goals:

- **Composable:** UI elements are encapsulated. UI elements can contain other UI elements. This includes temporal composition (e.g. a multi-step form)
- **Modular:** UI elements are encapsulated. You can swap out data sources in the UI, swap out UI backends, swap out nested UI elements, view UI elements individually, and reuse them in different contexts without unnecessary code duplication.
- **Rapidly iterable:** You can preview UI elements individually. You can preview UI elements with a dummy data source. You can preview UI elements in an alternate backend (e.g. show bounding boxes, view layouts only). You can generate regression tests from your UI previews. You can modify the UI and reload the preview without rebuilding your project, as devolve-ui uses a DSL scripting language.

devolve-ui UI elements are written in a DSL scripting language ([details](#dui-language)), like [slint-ui](https://docs.rs/slint-ui/latest) (the scripting languages themselves are different). This is because the scripting language:

- Can be reloaded without building your project
- Can be edited by a dedicated IDE, with blueprints-style visual programming, and possibly in the future WYSIWYG.
- Doesn't have to deal with Rust's verbose syntax / borrow checker (lang types are all copy-able or references to Rust types)
- Encourages encapsulation and UI responsiveness by making it harder to mix UI code and other code

## How?

In devolve-ui there are 2 kinds of UI elements:

- **Snapshots:** Some UI elements are represented as a function `Inputs -> (Render, Outputs)`. This is how immediate-mode UI and also in systems like [React](https://reactjs.org/) work. By feeding part of `Output` back into `Input` you can make this a pure function. The function gets repeatedly called for each "tick" (e.g. once every 60fps), but it memoizes / caches complex render calculations so it is fast.
- **Prompts:** Other UI elements (prompts) are represented as an async function (e.g. `async login() -> Credentials`). If the prompt has multiple stages (e,g. multi-step form), the function "sets" the immediate UI (an `Inputs -> (Render, Outputs)` function) during each stage. The prompt/function can call other prompt/functions and return values (e.g. `login` returns the user's credentials after they successfully login, typical prompts like alerts return the user's choice) which are used by other callers.

How snapshots are embedded in prompts: the prompt/function sets a snapshot/function at different points in its async execution. At any point in time, the prompt/function has a current snapshot/function set, which is how it is rendered

How prompts are embedded in snapshots: One of the `Input`s and `Output`s (part of `Inputs` and `Outputs`) is the prompt's state.
  - Initially `Input` is None and `Output` is None.
  - The prompt is spawned from an event (e.g. the user clicks a button which opens an alert box). If `Input` is None and the event is in another `Input`, `Output` will be the new spawned state.
  - From then on, `Input` will be the prompt's state, `Output` will be the prompt's state after other events (e.g. ticks) received from other `Input`s, and `Render` will contain the prompt's render (remember: render from the prompt's currently-set snapshot).
  - Eventually the prompt will complete and `Output` will be `None` again.

### DUI Language

The `.dui` scripting language is for creating **snapshots**: prompts are still declared in Rust code, as they will have more control. Each `.dui` file is a directed-acyclic [node-graph](https://github.com/jchanvfx/NodeGraphQt) with an `Inputs` node, `Outputs` node, and intermediate nodes. This ultimately forms the `Inputs -> (Render, Outputs)` function: `Render` is implicitly derived from the view nodes, which draw to the screen as a side-effect. Besides view nodes and the `Input` and `Output` nodes, there are also computation nodes, which just take inputs and outputs and encode functions including your own computations.

TODO images of the graphs created by .dui scripts, possibly better node-graph link or just use another image

An interesting note: a snapshot `Inputs -> (Render, Outputs)` is itself a node, and you can embed one `.dui` file as a node in another.

The `.dui` scripting language also lets you define basic Rust `struct` and `enum` types, for even faster iteration without rebuilding your app. The custom `struct` and `enum` shapes are checked with those in Rust, so as long as they have the same shape as a `C`-repr rust struct, they can be used interchangeably.

## Why?

I'm not satisfied with existing UI, and specifically want to create a UI with the [above principles](#what) 

Rust has a lot of UI frameworks, however many of them have various drawbacks. The main issues are that many require you to write the UI in Rust, which is slow to rebuild, and don't enforce or encourage a truly modular and composable structure. devolve-ui is not just another framework, it uses those already-existing frameworks, and exposes them in a way which enables rapid iteration, and encourages modularity and composablity.

## Example

Display a prompt with a text field, "OK", and "cancel".

- Animate the prompt in while ensuring that the text is editable, then wait for the user to click "OK" or "cancel" while letting them enter text.
- Ensure that "OK" is grayed out while the textbox is empty.
- Once the user makes their selection, the prompt is animated out.
- Typing "Enter" is considered clicking "OK"
- If the user clicked "OK" the text is returned from this prompt, else `None` is returned.
- Listeners can receive events when the text is modified

**text_input.dui**

```dui
Inputs
  placeholder: &str
  text: String
  -- events
  enter_key: Option<()>
  ===
    
"Ok Button" = Button
  text = "OK"
  is_enabled = Inputs.ok_enabled
  ===
  click = Outputs.click_ok
  
"Cancel Button" = Button
  text = "Cancel"
  ===
  click = Outputs.click_cancel
  
"Text Input" = TextField
  text = Inputs.text
  placeholder = Inputs.placeholder
  ===
  text = Outputs.text
  text_modified = Outputs.text_modified
  enter_key = Outputs.enter_key
  
Outputs
  ===
  text: String
  -- events
  text_modified: Option<(Range<usize>, String)>,
  enter_key: Option<()>,
  click_ok: Option<()>,
  click_cancel: Option<()>
```

TODO embedded image of this in the editor once it's created

**text_input.rs**

```rust
#[prompt]
async fn text_input(
    mut c: PromptContext<'_>,
    placeholder: In<str>,
    text: InOut<String>,
    text_modified: OutSend<(Range<usize>, String)>,
    ok_enabled: In<bool>
) -> String {
    let (enter_key_send, enter_key) = out_channel();
    let (click_ok_send, click_ok) = out_channel();
    let (click_cancel_send, click_cancel) = out_channel();
    // Note: `c.set_view` requires a mutable reference to `c`
    c.set_view("text_input.dui", labeled![
        placeholder,
        text,
        text_modified,
        enter_key: enter_key_send,
        click_ok: click_ok_send,
        click_cancel: click_cancel_send,
        ok_enabled
    ]);
    concurrent_race([
        || {
            // ok_enabled being set to false when text is empty may be possible in the .dui directly in the future
            // But we can always do it in Rust
            loop {
                text_modified.await;
                *ok_enabled = text.get().len() > 0;
            }
        },
        || {
            // Listen for ok or enter
            concurrent_race([click_ok, enter_key.recv()]).await;
            // We can clone
            // Some(text.get().clone())
            // ...or we can consume the value from text. This will *panic at runtime* if we let the UI run after consuming an output, so be careful doing this and make sure it's on the last frame (there are no awaits afterwards)
            Some(text.consume())
        },
        || {
            // Listen for cancel
            click_cancel.await;
            None
        }
    ]).await;
}
```

## Setup

For editing/building source, make sure rust is installed

```sh
git clone git@github.com:Jakobeha/devolve-ui.rs --recurse-submodules --depth=1
cd devolve-ui.rs
```# devolve: composable, rapidly-iterable editing

**Notice:** this is a primarily a *proof-of-concept*, subject to change and many parts are unimplemented.

## What?

devolve is a collection of Rust libraries:

- **devolve-core:** a framework to create and run rapidly-iterable  WSIWYG visual editors
- **devolve-bevy:** prototype of a potential [visual editor] for [bevy]

## Why?

Code is great for abstract content: servers, compilers, simulators, equations.

Code is *not* great for concrete content: user interfaces, websites, 3d models, scenes.

But, you may ask, we *have* code which creates UI, websites,

## What is DVO?

DVO is a framework to create editors for visual content.

devolve-ui is a framework to create DSLs for visual content. These may range from mobile UIs to 3D game objects and scenes to full-scale applications. devolve-ui's goal is *functional, composable, rapidly-iterable, visual editing*:

- **Composable:** parts can be embedded into other parts. Not much trouble scaling from simple controls or objects to full-scale UI or scenes.
- **Rapidly iterable:** change data without rebuilding and re-running, insert mock data, time-travel debugging
- **Visual editing:** The preview is right next to your code and updates in real-time

- What is a visual editor? See [Unity](), [Unreal Engine](), [Interface Builder](), etc.

There are 2 types of views: **immediate** views and **temporal** views. Immediate views are a function `inputs -> (render, outputs)`, temporal views are an async function

```
Immediate

 [ view ] <--> [ inputs, outputs ]
    
```

The key concepts of devolve-ui are:

- **Views:** UI, graphics, assets, scenes. These are written in a DSL which may be edited both in code (as in [slint]) or a visual editor (as in [Unity]).
    - Views may embed other views, and encode computations and even scripts. Your entire application may be a view. The key is that views are *rapidly-iterable*: they can be live-reloaded and edited while your application is running. In contrast, Rust code is much

- **Views:** The actual graphics and "content". Written in a DSL which may be edited in code (as in [slint]) or a visual editor (as in [Unity]).
    - Views may embed other views, and encode computations and even scripts. The key is that views are *rapidly-iterable*, they can be live-reloaded and edited without an instance of your application running.
- **Interface:** a data structure containing the view's inputs and outputs, which is how the view and your program communicate.
    - Inputs and outputs can be real-time or events. Inputs may be position, theme, and outputs are user input and also calculations such as collision detection which may be done directly in the view. A value can be both an input and output (e.g. text field content, player health)
    - Views are run on a separate thread from your main program, and the interface contains shared memory
    - Views have a set of inputs and outputs known as an **interface**, which is how they communicate with your main program. Inputs and outputs can be real-time (shared memory) or events (channels). Inputs may be position, color, etc. outputs are user input and calculations such as collision detection which are best done directly in the view
    - Abstractly, a view is a function `input -> (render, output)`
-
- **Prompts:**

Similar to [slint](https://slint-ui.com/releases/0.2.5/docs/rust/slint/) the actual graphics themselves are not written in Rust, but a separate "graphics" file format known as a **view**. Views may be edited in a visual WSYWIG editor similar to a game engine or Interface Builder, though they are also code.

The key

Views may link to other views and encode computations and even scripts. The point is that they are *fast-iterable* and *visually editable*, whereas code is

complete with prefabs (smaller .dui files). The .dui encodes visual layout and basic computations. They are composable, one .dui can refernece other .duis

The high level buzzword goal is to take "visual" WSYWIG-style editing and make it modular and composable using functional paradigms. `.dui` files are essentially functions of the form `inputs -> (render, outputs)` and prompts are async functions which present UI to the user as their "computation" and return the user's actions as their result.

This format is arbitrary, and devolve-ui actually provides multiple editors for 2D and 3D graphics devolve-ui provides 2D and 3D editors for each of these targets (UI-focused, 2D game focused, and 3D focused) to edit these graphics. These files are essentially functions which take input data, and return your UI/graphics render as well as any output events.

Using these graphics, you construct [**prompts**](https://jakobeha.github.io/devolve-ui/prompt-based-gui). These are async functions with a specific graphics file to use (you can include multiple scenes in one file or reference other files, so you only need one graphics file per prompt. But we might make it a specific element in a file if this leads to too many). During execution they may also alter or animate the input passed to the graphics file or wait for events, which are done in async functions. You combine these functions sequentially and concurrently.

**Example:** display a prompt with a text field, "OK", and "cancel". Animate the prompt in while ensuring that the text is editable, then wait for the user to click "OK" or "cancel" while letting them enter text. Ensure that "OK" is grayed out while the textbox is empty. Once the user makes their selection, the prompt is animated out. If the user clicked "OK" the text is returned from this prompt, else `None` is returned.

TODO prompt.dui (will probably be an embedded image or video)

```rust
// T = value is constant for the entire prompt unless it contains nested In/Out types.
// In<T> = can write T to .dui dynamically
// Out<T> = can read T from .dui dynamically
// InOut<T> = can read and write T from .dui dynamically. Cannot borrow, a workaround is to have a struct with InOut leaf fields
// InOutLock<T> = can read and write T from .dui. Can borrow but it will freeze the UI so use with caution
// InSignal<T> = In<T> but each write triggers an event
// OutSignal<T> = Out<T> but each read returns a future which awaits on the next event
// InOutSignal<T> = InOut<T> but each write triggers an event, and each read returns a future which awaits on the next event. If you concurrently read and write you will receive your own event.
// The type inside In/Out must derive DuiData, so you cannot nest In/Out types in other In/Out types.
// This type itself must derive DuiInterface
// In<T>, InOut<T>, and InOutLock<T> must be constructed with initial values, and InSignal<T> and InOutSignal<T> can be constructed with initial values, all others are created with ::new().
#[derive(DuiInterface)]
struct TextInput {
    placeholder: &str,
    text: Out<String>,
    text_modified: OutSignal<()>,
    ok_enabled: In<bool>,
    click_ok: OutSignal<()>,
    click_cancel: OutSignal<()>,
}

// file = ... is actually optional here since this has the same name
// data = ... is optional if your struct derives Default. Ours can't because we have placeholder.
#[prompt(
    file = "text_input.dui",
    init = "TextInput {
        placeholder,
        text: Out::new(),
        text_modified: OutSignal::new(),
        ok_enabled: In::new(false),
        click_ok: OutSignal::new(),
        click_cancel: OutSignal::new()
    }"
)]
async fn text_input(placeholder: &str) -> Option<String> {
    // __input is a stub which will be replaced to return a reference to the actual input.
    // If not explicitly replaced else it will panic at runtime.
    // It is here instead of function input so that the signature is identical after proc macro expansion,
    // so IDE the analyzer should not have to expand the proc macros.
    let TextInput { text, ok_enabled, click_ok, click_cancel, text_modified, .. } = __input();
    concurrent_race(|| {
        // ok_enabled being set to false when text is empty may be possible in the .dui directly in the future
        // But we can always do it in Rust
        loop {
            text_modified.await;
            ok_enabled.set(text.get().len() > 0);
        }
    }, || {
        // Listen for ok
        click_ok.await;
        // We can clone
        // Some(text.get().clone())
        // ...or we can consume the value from text. This will *panic at runtime* if we let the UI run after consuming an output, so be careful doing this and make sure it's on the last frame (there are no awaits afterwards)
        Some(text.consume())
    }, || {
        // Listen for cancel
        click_cancel.await;
        None
    }).await;
}
```

## Setup

```sh
git clone git@github.com:Jakobeha/devolve-ui.rs --recurse-submodules --depth=1
cd devolve-ui.rs
```

Install rust if you haven't already

[Download IntelliJ](https://www.jetbrains.com/idea/download/download-thanks.html) or another code editor if you want, make sure rust is installed, etc.# devolve-ui: data-binding graphics and UI engine

**Notice:** this is primarily a **proof-of-concept** and not all parts are implemented.

devolve-ui is a UI and game engine to create 2D and 3D graphics ranging from UIs to HUDs to 3D game objects and scenes. devolve-ui's goal is a *composable visual (WSYWIG) editor*.

The key concepts of devolve-ui are:

- **Views:** The actual graphics and "content". Written in a DSL which may be edited in code (as in [slint]) or a visual editor (as in [Unity]).
	- Views may embed other views, and encode computations and even scripts. The key is that views are *rapidly-iterable*, they can be live-reloaded and edited without an instance of your application running.
- **Interface:** a data structure containing the view's inputs and outputs, which is how the view and your program communicate.
	- Inputs and outputs can be real-time or events. Inputs may be position, theme, and outputs are user input and also calculations such as collision detection which may be done directly in the view. A value can be both an input and output (e.g. text field content, player health)
	- Views are run on a separate thread from your main program, and the interface contains shared memory 
	- Views have a set of inputs and outputs known as an **interface**, which is how they communicate with your main program. Inputs and outputs can be real-time (shared memory) or events (channels). Inputs may be position, color, etc. outputs are user input and calculations such as collision detection which are best done directly in the view
	- Abstractly, a view is a function `input -> (render, output)`
- 
- **Prompts:**

Similar to [slint](https://slint-ui.com/releases/0.2.5/docs/rust/slint/) the actual graphics themselves are not written in Rust, but a separate "graphics" file format known as a **view**. Views may be edited in a visual WSYWIG editor similar to a game engine or Interface Builder, though they are also code.

The key 

Views may link to other views and encode computations and even scripts. The point is that they are *fast-iterable* and *visually editable*, whereas code is

complete with prefabs (smaller .dui files). The .dui encodes visual layout and basic computations. They are composable, one .dui can refernece other .duis

The high level buzzword goal is to take "visual" WSYWIG-style editing and make it modular and composable using functional paradigms. `.dui` files are essentially functions of the form `inputs -> (render, outputs)` and prompts are async functions which present UI to the user as their "computation" and return the user's actions as their result.

This format is arbitrary, and devolve-ui actually provides multiple editors for 2D and 3D graphics devolve-ui provides 2D and 3D editors for each of these targets (UI-focused, 2D game focused, and 3D focused) to edit these graphics. These files are essentially functions which take input data, and return your UI/graphics render as well as any output events.

Using these graphics, you construct [**prompts**](https://jakobeha.github.io/devolve-ui/prompt-based-gui). These are async functions with a specific graphics file to use (you can include multiple scenes in one file or reference other files, so you only need one graphics file per prompt. But we might make it a specific element in a file if this leads to too many). During execution they may also alter or animate the input passed to the graphics file or wait for events, which are done in async functions. You combine these functions sequentially and concurrently.

**Example:** display a prompt with a text field, "OK", and "cancel". Animate the prompt in while ensuring that the text is editable, then wait for the user to click "OK" or "cancel" while letting them enter text. Ensure that "OK" is grayed out while the textbox is empty. Once the user makes their selection, the prompt is animated out. If the user clicked "OK" the text is returned from this prompt, else `None` is returned.

TODO prompt.dui (will probably be an embedded image or video)

```rust
// T = value is constant for the entire prompt unless it contains nested In/Out types.
// In<T> = can write T to .dui dynamically
// Out<T> = can read T from .dui dynamically
// InOut<T> = can read and write T from .dui dynamically. Cannot borrow, a workaround is to have a struct with InOut leaf fields
// InOutLock<T> = can read and write T from .dui. Can borrow but it will freeze the UI so use with caution
// InSignal<T> = In<T> but each write triggers an event
// OutSignal<T> = Out<T> but each read returns a future which awaits on the next event
// InOutSignal<T> = InOut<T> but each write triggers an event, and each read returns a future which awaits on the next event. If you concurrently read and write you will receive your own event.
// The type inside In/Out must derive DuiData, so you cannot nest In/Out types in other In/Out types.
// This type itself must derive DuiInterface
// In<T>, InOut<T>, and InOutLock<T> must be constructed with initial values, and InSignal<T> and InOutSignal<T> can be constructed with initial values, all others are created with ::new().
#[derive(DuiInterface)]
struct TextInput {
    placeholder: &str,
    text: Out<String>,
    text_modified: OutSignal<()>,
    ok_enabled: In<bool>,
    click_ok: OutSignal<()>,
    click_cancel: OutSignal<()>,
}

// file = ... is actually optional here since this has the same name
// data = ... is optional if your struct derives Default. Ours can't because we have placeholder.
#[prompt(
    file = "text_input.dui",
    init = "TextInput {
        placeholder,
        text: Out::new(),
        text_modified: OutSignal::new(),
        ok_enabled: In::new(false),
        click_ok: OutSignal::new(),
        click_cancel: OutSignal::new()
    }"
)]
async fn text_input(placeholder: &str) -> Option<String> {
    // __input is a stub which will be replaced to return a reference to the actual input.
    // If not explicitly replaced else it will panic at runtime.
    // It is here instead of function input so that the signature is identical after proc macro expansion,
    // so IDE the analyzer should not have to expand the proc macros.
    let TextInput { text, ok_enabled, click_ok, click_cancel, text_modified, .. } = __input();
    concurrent_race(|| {
        // ok_enabled being set to false when text is empty may be possible in the .dui directly in the future
        // But we can always do it in Rust
        loop {
            text_modified.await;
            ok_enabled.set(text.get().len() > 0);
        }
    }, || {
        // Listen for ok
        click_ok.await;
        // We can clone
        // Some(text.get().clone())
        // ...or we can consume the value from text. This will *panic at runtime* if we let the UI run after consuming an output, so be careful doing this and make sure it's on the last frame (there are no awaits afterwards)
        Some(text.consume())
    }, || {
        // Listen for cancel
        click_cancel.await;
        None
    }).await;
}
```