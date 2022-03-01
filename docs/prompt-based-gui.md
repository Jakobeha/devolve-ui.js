# Prompt-based GUI: how to define a GUI as a function

In GUI-based applications users are often presented with **prompts**, like dialog boxes. The user must click "ok" or "cancel" in order to continue.

However, "prompt" can be generalized into more UI elements. A form is a prompt, where a user must fill out the fields and click "submit". A menu is a prompt, where a user must click one of the menu items. In fact, *any* GUI element can be represented a prompt, and any application can be considered a concurrent set of prompts, when "prompt" is defined as "*a series of interactions a user must complete in order for something to occur*".

But why is this important? Because a "prompt" can be represented as an asynchronous function. When you call the function, it presents the prompt to the user; when the user answers, the function returns. If you're a fan of functional programming this will be important to you, because it means *we can define entire GUI-based applications as functions*, and leverage the power of functional programming when writing GUIs.

### GUI implemented as a function

Here is a simple social media site represented as a function in Haskell, a functional programming language, using prompt-based GUI.

```haskell
TODO example
```

### When to use prompt-based GUI

In practice, you shouldn't necessarily implement *every* GUI as a prompt-function. But GUI-as-prompts does have some benefits over "traditional" (e.g. React-Redux) GUI. Encoding some GUI operations as prompt functions can make them very easy to read to read and implement.

#### Automation

GUI prompts are *especially* better if you want your application to be easily automated. Automating a function-of-prompts is as simple as replacing the prompt function calls with function calls to your automator, be it a neural network or remote client or sequence of GUI actions recorded into a text file. Automating a React-based application, especially if it listeners for complex mouse or touch gestures, is harder.

#### GUI as a state machine

GUI prompts are also better at handling state transitions. If you model your application as a state or activity diagram, it's much easier and more straightforward to convert that diagram into a prompt-function than a React component. In the React component, you need a property to represent the current location in the diagram, in a prompt-function, the location is automatic because it's simply the location inside the function's code.
