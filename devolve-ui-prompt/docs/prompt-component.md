# Prompt components
## Prompt components in devolve-ui

Prompt components are created with `make_prompt_component`, with the same syntax as `make_component`. While `make_component` wraps a function which returns a `VNode`, `make_prompt_component` wraps an async function which returns `Future<T>`: instead of returning the node, `make_prompt_component` asynchronously yields nodes via `c.yield`.

`c.yield(|resolve| node).await` causes the prompt component render as `node` until `resolve` is called. `resolve` takes one argument; which is what `c.yield` ultimately returns.

`make_prompt_component` defines a macro which can be used just like the macro defined by regular components. The prompt component will render as the currently yielded node, until it returns, in which case it disappears. The macro has an additional optional argument `resolve`, which is also called when the prompt component returns. The prompt component will restart when the parent component is destroyed and then re-created.

`make_prompt_component`'s macro can also be called with square brackets, which should be done in prompt components'. This will return a future which runs the prompt component to completion and returns it's result. `foo![c, { ... }, ...].await` is semantically equivalent to `c.yield(|c, resolve| foo!(c, { ... }, ...)).await`

You can run prompt components or yield nodes concurrently with `concurrent![component1![c, ...], component2[c, ...], c.yield(|c, resolve| ...)]].await`. `concurrent![a, b, c]` macro-expands to `concurrent(vec![|mut c| a, |mut c| b, |mut c| c])`: `c` can be borrowed mutably in each function because the execution is still on a single thread, so only one function runs at a time.

You cannot use regular component hooks in prompt components. Instead, you must use prompt components, or yield a regular component with regular hooks. This is because regular hooks trigger rerenders which re-run the component function, but we can't just re-run a prompt component because it's asynchronous.

I suggest you wrap side-effects in prop components within `run_effect`. However this function is merely denotation and just calls it's argument, you don't have to wrap the side-effects.

### Rx (reactive value)

Because you can't re-render a prompt component, the props and state are wrapped in `Rx`s (reactive values), and can't be accessed directly. Instead, you
get a `RxContext` within a closure, either `IRx::run` or a component context, and call `rx.get(c)`; the closure is re-run when the `Rx` value changes.

`MRx` and `IRx` aren't just part of prompt components, they also integrate with regular components and can be used standalone. They allow you to write solid.js-like code, where updating an `Rx` only updates the components which use it.

## Motivation
Prompt-based UI is a paradigm where you write UI as a set of prompts, which are asynchronous functions. For example, a social media site could be written like this (pseudocode):

```
async fn site({account}) {
  concurrent[
    header(),
    if account == null {
      concurrent[
        login()
        signup()
      ]
    } else {
      logout()
    },
    sequence {
      post = concurrent[
        viewPosts(),
        if state.account != null {
          createPost()
        }
      ]
      viewPost(post)
    },
    footer(),
  ]
}

async fn signup() {
  sequence {
    accountDetails = signupForm()
    moreDetails = completeSignup(accountDetails)
    run { createAccount(accountDetails, moreDetails) }
  }
}

async fun login({account}) {
  sequence {
    credentials = loginForm()
    run { try { account = login(credentials) }}
  }
}

async fn viewPost(post, {account}) {
  concurrent[
    viewBody(post.body),
    viewComments(post.comments)
    if account != null {
      addComment()
    }
  ]
}
```

In this language, code in `concurrent` runs concurrently, code in `sequential` runs sequentially. While a function is running, its corresponding UI element is visible: e.g. when `loginForm` is called, the login form appears and asynchronously blocks until the user submits it, then returns with the inputted credentials. Side-effects are enclosed in `run` blocks for clarity, although this isn't necessary.

Prompt-based UI isn't ideal for every situation, but **arguably** is better than traditional UI in some cases: mainly, multi-state prompts. In traditional UI you must manually keep track of the component's temporal state; in prompt-based UI this is automatic, the state is the current execution point in the asynchronous function.
