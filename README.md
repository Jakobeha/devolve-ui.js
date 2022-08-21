# devolve-ui: composable, modular, rapidly-iterable UI

**Notice:** this is a proof-of-concept, subject to change and many parts are unimplemented.

## What?

devolve-ui is a UI library which integrates with existing UI libraries such as [egui](https://docs.rs/egui/latest) and [druid](https://docs.rs/druid/latest). devolve-ui provides a way to compose simple components implemented in these libraries into full applications, in a way that is modular and supports near-instant reloads and previews.

devolve-ui has the following goals:

- **Composable:** UI elements are encapsulated. UI elements can contain other UI elements. This includes temporal composition (e.g. prompt into another element, multi-step form)
- **Modular:** UI elements are encapsulated. You can swap out data sources in the UI, swap out UI backends, swap out UI components, view components individually, and reuse them in different contexts without unnecessary code duplication.
- **Rapidly iterable:** You can preview UI elements individually. You can preview UI elements with a dummy data source. You can generate regression tests from your UI previews. You can modify the preview without rebuilding your project, as devolve-ui uses a DSL scripting language.

devolve-ui UI elements are written in a DSL scripting language ([details](#dui-language)), just like [slint-ui](https://docs.rs/slint-ui/latest) (the scripting languages themselves are different). This is because the scripting language:

- Can be reloaded without building your project
- Can be edited by a dedicated IDE with graphical representation and possibly in the future a live-preview.
- Doesn't have to deal with Rust's verbose syntax / borrow checker (lang types are all copy-able or references to Rust types)
- Encourages encapsulation by making it harder to mix UI code and other code

## How?

In devolve-ui there are 2 kinds of UI elements:

- **Snapshots:** Some UI elements are represented as a function `Inputs -> (Render, Outputs)`. This is how immediate-mode UI and also in systems like [React](https://reactjs.org/) work. By feeding part of `Output` back into `Input` you can represent this UI as a pure function. The function gets repeatedly called for each "tick" (e.g. once every 60fps), though it can cache UI so it is fast.
- **Prompts:** Other UI elements (prompts) are represented as an async function (e.g. `async login() -> Credentials`). If the prompt has multiple stages (e,g. multi-step form), the function "sets" the immediate UI (an `Inputs -> (Render, Outputs)` function) during each stage. The prompt/function can call other prompt/functions and return values (e.g. user's response to the prompt) which are used by other callers.

How snapshots are embedded in prompts: the prompt/function sets a snapshot/function at different points in its async execution. At any point in time, the prompt/function has a current snapshot/function set, which is how it is rendered

How prompts are embedded in snapshots: One of the `Input`s and `Output`s (part of `Inputs` and `Outputs`) is the prompt's state.
  - Initially `Input` is None and `Output` is None.
  - The prompt is spawned from an event (e.g. the user clicks a button which opens an alert box). If `Input` is None and the event is in another `Input`, `Output` will be the new spawned state.
  - From then on, `Input` will be the prompt's state, `Output` will be the prompt's state after other events (e.g. ticks) received from other `Input`s, and `Render` will contain the prompt's render (remember: render from the prompt's currently-set snapshot).
  - Eventually the prompt will complete and `Output` will be `None` again.

### DUI Language

The `.dui` scripting language is for creating **snapshots**: prompts are still declared in Rust code, as they will have more control. Each `.dui` file is a directed-acyclic [node-graph](https://github.com/jchanvfx/NodeGraphQt) with an `Inputs` node, `Outputs` node, and intermediate nodes. This ultimately forms the `Inputs -> (Render, Outputs)` function: `Render` is implicitly derived from the view nodes, which draw to the screen as a side-effect. Besides view nodes and the `Input` and `Output` nodes, there are also computation nodes, which just take inputs and outputs and encode functions including your own computations.

TODO images of the graphs created by .dui scripts, possibly better node-graph link or just use another image

An interesting fact is that a snapshot `Inputs -> (Render, Outputs)` is itself a node, and you can embed one `.dui` file as a node in another.

The `.dui` scripting language also lets you define basic Rust `struct` and `enum` types, for even faster iteration without rebuilding your app. The custom `struct` and `enum` shapes are checked with those in Rust, so as long as they have the same shape as a `C`-repr rust struct, they can be used interchangeably.

## Why?

I'm not satisfied with existing UI, and specifically want to create a UI with the [above principles](#what) 

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
```