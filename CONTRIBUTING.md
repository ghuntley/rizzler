# 🙌 Wanna Help Make `rizzler` Even More Based? 🙌

Yo! Thanks for being interested in contributing to `rizzler`. You're awesome! ✨

Whether you're fixing a bug, proposing a dope new feature, or just tidying things up, your help is super appreciated. Let's make Git conflicts even less of a headache together.

## 🌊 How to Contribute: The Flow

1.  **Got an Idea or Bug? -> Open an Issue:**
    *   Check if an [issue](https://github.com/ghuntley/rizzler/issues) already exists for your idea/bug.
    *   If not, open a new one! Be descriptive. For bugs, tell us how to reproduce it. For features, explain the *why* and *what*.
    *   Let's chat about it in the issue first, especially for bigger changes.

2.  **Wanna Code? -> Fork & Pull Request:**
    *   Fork the repo to your own GitHub account.
    *   Create a new branch for your changes (e.g., `feat/add-cool-strategy` or `fix/resolve-bug-123`).
    *   Make your changes. Follow the code style (see below).
    *   **Add tests!** Super important, especially for new features or strategies.
    *   Commit your changes with clear messages.
    *   Push your branch to your fork.
    *   Open a Pull Request (PR) back to the main `rizzler` repo.
    *   Link the PR to the issue if there is one.
    *   We'll review it, maybe ask for changes, and then hopefully merge it! 🎉

## 🛠️ Dev Setup: Getting Ready

It's pretty straightforward since it's Rust:

1.  Clone your fork: `git clone https://github.com/YOUR_USERNAME/rizzler.git`
2.  Navigate into the directory: `cd rizzler`
3.  Build it: `cargo build`
4.  Run tests: `cargo test`

## ✨ Code Style: Keep it Clean

We follow standard Rust practices:

*   **Formatting:** Use `rustfmt`. Run `cargo fmt` before committing.
*   **Linting:** Use `clippy`. Run `cargo clippy` and fix any warnings.
*   **Comments:** Explain the *why*, not just the *what*, especially for complex logic.

## ✅ Testing: Prove it Works

Tests are crucial! We need to make sure `rizzler` reliably resolves conflicts (or knows when not to).

*   Add unit tests (`#[test]`) for new functions and logic.
*   For new strategies, add tests covering cases it *should* handle and cases it *shouldn't*.
*   Check out the existing tests in the codebase for examples.
*   Make sure `cargo test` passes before submitting your PR.

## 🔥 Where We REALLY Need Your Help: Resolution Strategies! 🔥

This is where `rizzler` truly shines, and where **you can make a huge impact!** We want `rizzler` to have a whole arsenal of strategies for different kinds of conflicts.

**What's a Strategy?**

It's basically a module that implements the `ResolutionStrategy` trait (check out `src/resolution_engine.rs` and existing strategies like `src/fallback.rs` or `src/ai_resolution.rs`). It needs to:

1.  Have a unique `name()`.
2.  Decide if it `can_handle()` a specific `ConflictRegion`.
3.  Implement the core `resolve_conflict()` logic.

**Ideas for New Strategies (Bring Your Own Too!):**

*   **Rule-Based:** Simple strategies for specific patterns (like the existing `whitespace-only`). Maybe one for comment-only conflicts? Renamed variables?
*   **AI Variations:** Different ways to prompt the AI? Fine-tuned models? Strategies that use smaller/cheaper models first?
*   **Hybrid Approaches:** Combine rules and AI.

**🚀 The BIG Idea: Tree-Sitter + AI 🤯**

Okay, here's a challenge if you're feeling ambitious: imagine a strategy that uses **tree-sitter**. Tree-sitter can parse code into an Abstract Syntax Tree (AST), meaning it *understands* the code structure, not just the text.

*   **Why?** Instead of just showing the AI messy text with `<<<<<`, `=====`, `>>>>>`, we could:
    *   Identify the *exact* conflicting code blocks/nodes in the AST.
    *   Give the AI much richer context about the *semantic* nature of the conflict (e.g., "these two functions were modified differently", "this variable definition conflicts").
    *   Potentially perform smarter, safer merges by manipulating the AST directly or guiding the AI with AST-level info.

*   **The Vision:** A strategy that uses tree-sitter to analyze the conflict semantically, then crafts a *way* better prompt for the AI, leading to more accurate resolutions, especially in complex code.

If you're interested in language parsing, ASTs, and pushing the boundaries of AI code merging, tackling a tree-sitter-based strategy would be absolutely legendary!

Even if you don't build the whole thing, contributing tree-sitter parsing logic for specific languages would be a massive help.

**How to Contribute a Strategy:**

1.  Open an issue to discuss your idea!
2.  Code it up, implementing the `ResolutionStrategy` trait.
3.  Add it to the `ResolutionEngine` (see how others are added in `src/resolution_engine.rs`).
4.  **Write comprehensive tests!**
5.  Submit a PR.

## 📜 License Reminder

By contributing, you agree that your submissions will be licensed under the project's [MIT License](LICENSE).

---

Thanks again for your interest! Let's get rizzin'! ✨
