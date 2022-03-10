# Prompt-based GUI: how to define a GUI as a function

In GUI-based applications users are often presented with **prompts**, like dialog boxes. The user must click "ok" or "cancel" in order to continue.

However, "prompt" can be generalized into more UI elements. A form is a prompt, where a user must fill out the fields and click "submit". A menu is a prompt, where a user must click one of the menu items.

We define "prompt" as "*a series of interactions a user must complete in order for something to occur*". Going to the extreme, *any* GUI element can be represented a prompt, and any application can be considered a concurrent set of prompts.

But why is this important? Because a "prompt" can be represented as an asynchronous function. In turn, we can leverage the power of functional programming and general function composition when writing GUIs, and even *define entire GUI-based applications as functions*. If you're a fan of functional programming, this may be significant.

-- Henceforth we refer to a function which presents a "prompt", as defined above, as a **prompt-function**.

### GUI implemented as a function

TODO example

Here is a simple social media site represented as a function in Haskell, a functional programming language, using prompt-based GUI.

```haskell
TODO example
```

### When to use prompt-based GUI

In practice, you shouldn't necessarily implement *every* GUI as a prompt-function. But prompt-functions have some benefits over "traditional" (e.g. React-Redux) components. Ideally, you will implement "prompt-like" UI, like dialog boxes, as prompt-functions, and continuous UI, like menus and the main screen, as standard components.

#### Automation

Prompt-functions are *especially* better if you want your application to be easily automated. Automating a prompt-function is as simple as replacing the function implementation with a calls to your automator, be it a neural network or remote client or sequence of GUI actions recorded into a text file. Automating a component-based application, especially if it listens for complex mouse or touch gestures, is harder.

#### GUI as a state machine

Prompt-functions are also better at handling state transitions. If you model your application as a state or activity diagram, it's much easier and more straightforward to convert that diagram into a prompt-function than a component. In the component, you need a property to represent the current location in the diagram; in a prompt-function, the location is automatic because it's simply the location inside the function's code.
