# devolve-ui: composable, rapidly-iterable, visual editors for your datatypes

**Notice:** this is a primarily a *proof-of-concept*, subject to change and many parts are unimplemented.

## What?

devolve-ui is a UI system based on the following principles:

- A UI element is just a visual editor (inspector) for data
- A UI scene is just a collection of UI elements + layout
- Writing UI in a DSL or even WYSIWYG is easier than writing UI natively
- Some UI can be represented as a function `(Input, InEvents) -> (Render, Output, OutEvents)`. This is easier than writing a UI which synchronizes data but fast enough (immediate-mode UI)
- Other UI (prompts) can be represented as an async function (e.g. `login() -> Credentials`)
- UI in apps and 2D/3D scenes in games have many similarities

devolve-ui strives to be

- **Composable:** UI elements encapsulate themselves. You can swap out data sources in the UI and swap out UI for your data.
- **Rapidly-iterable:** You can change the look and feel of the UI without rebuilding and re-running your app
- **Scalable:** Whether you want just a single inspector for your Rust data while debugging your app, to a full-blown application which you show to users

The key concepts of devolve-ui are:

- **View:** a function of type `(Input, InEvents) -> (Render, Output, OutEvents)`
  - **AtomView:** a simple graphic or native UI control. Written in native Rust, but these are small enough they shouldn't change frequently.
  - **CompoundView:** a node-graph of inputs, outputs, computations, and child views like so: ![there is a node with the view's inputs, another with the view's outputs, a node for each child view, nodes for value transformers, and nodes which let you embed lists or maps of other nodes and extract data for each item]. It's how you compose views. Written in a DSL which can be edited in the DUI editor (TODO). It can be live-reloaded without recompiling your app, and even while your app is running.
- **Interface:** Provides volatile (changes at any time) / mutable data and events to views over a period of time. A datastructure with the following types of fields:
  - `In<T>`: takes a reference to `T` and provides it as a volatile input. Passed to the view-function as `Input`
  - `Out<T>`: derefs to `T`, is an output / updated from the view-function's `Output`
  - `InOut<T>`: owns a `T`, is both an `Input` and `Output`
  - `InRecv<T>`: Corresponding `InSend` lets you send events of type `T` to this which get forwarded to the view. Events are passed to the view-function as `Some`s in `InEvents` (`None` for when there was no event that frame).
  - `OutSend<T>`: Corresponding `OutRecv` lets you receive events of type `T` from this which are forwarded from the view. When the view-function returns a `Some` in `OutEvents`, this will emit the event.
  - (In the future we may have `ConstIn<T>` for non-volatile inputs, but for now just use `In<T>`)
- **Prompt:** Asynchronous function / generator which takes an `Interface`, yields `View`s, and returns an arbitrary value (usually unit but e.g. the login credentials for a login form). The yielded components may have inputs and outputs from the prompt's interface, as well as local inputs and outputs (which is how you embed state into components).

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
) -> Output<String> {
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