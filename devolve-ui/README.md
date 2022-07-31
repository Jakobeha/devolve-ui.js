# devolve-ui: data-binding graphics and UI engine

**Notice:** Currently this is a **proof-of-concept** and not yet implemented.

devolve-ui is a way to create 2D and 3D graphics ranging from UIs to HUDs to 3D game objects and scenes.

Similar to [slint](https://slint-ui.com/releases/0.2.5/docs/rust/slint/), the actual graphics themselves are not written in Rust, but a separate "graphics" file format. devolve-ui provides a 2D and 3D editors for each of these target (UI-focused, 2D game focused, and 3D focused) to edit these graphics. These files are essentially functions which take input data, and return your UI/graphics render as well as any output events.

Using these graphics, you construct [**prompts**](https://jakobeha.github.io/devolve-ui/prompt-based-gui). These are async functions with a specific graphics file to use (you can include multiple scenes in one file or reference other files, so you only need one graphics file per prompt. But we might make it a specific element in a file if this leads to too many). During execution they may also alter or animate the input passed to the graphics file or wait for events, which are done in async functions. You combine these functions sequentially and concurrently.

**Example:** display a prompt with a text field, "OK", and "cancel". Animate the prompt in while ensuring that the text is editable, then wait for the user to click "OK" or "cancel" while letting them enter text. Ensure that "OK" is grayed out while the textbox is empty. Once the user makes their selection, the prompt is animated out. If the user clicked "OK" the text is returned from this prompt, else `None` is returned.

TODO prompt.dui (will probably be an embedded image or video)

```rust
// In<T> = can write T to .dui dynamically
// Out<T> = can read T from .dui dynamically
// InOut<T> = can read and write T from .dui dynamically. Cannot borrow, a workaround is to have a struct with InOut leaf fields
// InOutLock<T> = can read and write T from .dui. Can borrow but it will freeze the UI so use with caution
// InSignal<T> = In<T> but each write triggers an event
// OutSignal<T> = Out<T> but each read returns a future which awaits on the next event
// InOutSignal<T> = InOut<T> but each write triggers an event, and each read returns a future which awaits on the next event. If you concurrently read and write you will receive your own event.
// T = value is static for the entire prompt unless it contains nested In/Out types.
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