# ✨ rizzler: stop crying over Git merge conflicts and let AI handle the drama ✨

💀 **Ugh, merge conflicts.** That sinking feeling when Git screams at you? We've all been there. Manually fixing those tangled messes? It's giving... tedious. It's giving... waste of my precious time. 😩

![rizzler](./assets/rizzler.png)

🚀 **Enter rizzler:** Your new AI bestie that actually *gets* Git. This ain't your grandpa's merge tool. rizzler slides into your Git workflow and uses ✨ AI magic ✨ (think OpenAI, Claude, Gemini, Bedrock - the whole squad) to automatically resolve those annoying merge conflicts. Less time untangling, more time coding (or scrolling). You're welcome. 😉

Basically, it turns this:

```diff
<<<<<<< HEAD
const message = "Hello from main branch!";
=======
const message = "Waddup from feature branch!";
>>>>>>> feature-branch
```


Into *actual*, usable code, letting you get back to the important stuff. ✨

## 🚀 Get Rizzin': Installation

Ready to ditch the conflict drama? Let's get you set up. Choose your install method:

1.  **Build from Source:**
    *   Since this is a Rust project, you'll need the Rust toolchain installed.
    *   Clone this repo and run:
        ```bash
        cargo build --release
        # The binary will be in target/release/rizzler
        ```
    *   Make sure this binary is somewhere in your system's `PATH`.

2.  **Install with Nix (Recommended):**
    *   **Declarative (Profile):** Install directly into your user profile:
        ```bash
        nix profile install github:ghuntley/rizzler
        ```
    *   **Temporary (Run):** Try it out without installing permanently:
        ```bash
        nix run github:ghuntley/rizzler -- --help # Or any other rizzler command
        ```
    *   **NixOS/Home Manager:** Add `rizzler` as an input to your `flake.nix` and include it in your `environment.systemPackages` or `home.packages`.

3.  **Download from GitHub Releases:**
    *   Go to the [latest releases page](https://github.com/ghuntley/rizzler/releases/latest).
    *   Find the archive (`.tar.gz` or `.zip`) for your operating system (Linux, macOS, Windows) and architecture (e.g., `x86_64`, `aarch64`).
    *   Download and extract the archive.
    *   Copy the `rizzler` binary to a directory in your system's `PATH` (e.g., `/usr/local/bin` or `~/.local/bin`).

4.  **Hook it up with Git (for automatic resolution):** *After installing `rizzler`*, tell Git to use it *automatically* during merges/pulls for specific file types. You can do this for just one project (`--local`) or for all your projects (`--global`).
    ```bash
    # Example: Configure for the current repo only
    rizzler setup --local --extensions js ts py rs go java

    # Example: Configure globally for your user
    rizzler setup --global --extensions js ts py rs go java md json yaml
    ```
    This command tweaks your `.gitconfig` and sets up a `.gitattributes` file. **This step is for enabling automatic conflict resolution.**

## 🔌 How it Hooks Up with Git (The Nerdy Deets)

Okay, so how does `rizzler` actually get triggered by Git? It's not *actual* magic, just some clever Git config.

1.  **Git Config (`.gitconfig`):** The `rizzler setup` command adds a custom merge driver definition to your Git configuration. It looks something like this:
    ```ini
    [merge "rizzler"]
        name = rizzler
        driver = rizzler %O %A %B %P
        trustExitCode = true
    ```
    This tells Git: "Hey, there's a merge tool called `rizzler`. When you need it, run the `rizzler` command with these file paths (`%O`, `%A`, `%B`, `%P` are placeholders Git fills in).

2.  **Git Attributes (`.gitattributes`):** How does Git know *when* to use the `rizzler` driver? That's where `.gitattributes` comes in (either in your repo or globally). The `setup` command adds lines like this:
    ```
    *.js merge=rizzler
    *.py merge=rizzler
    # etc...
    ```
    This tells Git: "For any file ending in `.js` (or `.py`, etc.), if there's a merge conflict, use the `rizzler` merge driver we defined earlier."

3.  **The Hand-off:** When you run `git merge` (or pull, rebase, etc.) and Git hits a conflict in a file matching one of the patterns in `.gitattributes`, it automatically runs the `rizzler` command specified in your `.gitconfig`. `rizzler` does its AI thing, hopefully fixes the file, and then exits.

4.  **Exit Code Matters:** `rizzler` tells Git if it succeeded by its exit code. `0` means "All good, conflicts resolved!" and Git continues. Any other number means "Nah, couldn't fix it, you handle it," and Git leaves the conflict markers for you.

So yeah, that's the behind-the-scenes tea on how `rizzler` becomes your automated merge conflict wingman.

## 🚨 Heads Up: Safety & The Real Tea 🚨

Okay, fam, let's keep it 💯. `rizzler` is cool, but AI ain't magic (yet!).

*   **No Cap, It's Not Actually Reading Code:** Neither `rizzler` nor the big brain AIs (GPT, Claude, etc.) *truly* understand your code like a compiler or interpreter does. They don't get the deep logic, the Abstract Syntax Tree (AST), or the semantic meaning. They're basically super-powered pattern matchers, guessing based on tons of text they've seen. Sometimes this guess is fire, sometimes it's... not. 🔥 vs 🗑️

*   **The Ghost of `goto fail;`:** Remember that time Apple had a *massive* security flaw in their SSL/TLS code back in 2014? It was literally because of a duplicated `goto fail;` line. Like this (oversimplified):
    ```c
    if ((err = SSLHashSHA1.update(&hashCtx, &serverRandom)) != 0)
        goto fail;
    if ((err = SSLHashSHA1.update(&hashCtx, &signedParams)) != 0)
        goto fail;
        goto fail; // <--- 💀 THIS EXTRA LINE SKIPPED THE VERIFICATION!!!
    if ((err = SSLHashSHA1.final(&hashCtx, &hashOut)) != 0)
        goto fail;
    // ... crucial signature verification code ...
    fail:
        // ... error handling ...
    ```
    A tiny, almost invisible change (maybe even something an auto-formatter *might* do weirdly, or an AI *might* hallucinate in a merge) completely broke critical security checks. This is the kind of subtle-but-deadly bug that purely text-based merging (like what AI does *now*) can accidentally introduce because it doesn't understand the *consequences* of the code structure.

*   **Recommendation Station:** Because of this, we strongly advise **NOT** running `rizzler` blindly on your main production branches (`main`, `master`, `trunk`, whatever your vibe is). It's giving... risky. 😬 Instead, use `rizzler` for the lower-stakes game: resolving conflicts when **rebasing** your feature branch *from* the main branch. Get your local branch up-to-date, let `rizzler` handle the rebase conflicts, **review the changes carefully**, and *then* merge your clean feature branch.

*   **The Glow-Up Goal:** We *want* `rizzler` to be smarter! The dream is proper **semantic merging** – understanding the code structure (AST) to make safer merges. AI could then be a fallback or assistant for the really tricky bits. Wanna help build this? Check out [CONTRIBUTING.md](CONTRIBUTING.md)! We'd love the help. ✨

## ⚙️ Dial in the Settings: Configuration

`rizzler` is pretty chill out of the box, but you can customize its vibe. Configs are layered, kinda like your fave fit:

1.  **Environment Variables (Highest Priority):** Set these in your shell. They override everything else.
2.  **Git Config:** Use `git config` or the `rizzler config` command. Can be local or global.
3.  **`.rizzler` file:** A `TOML` file in your project root for repo-specific settings (lower priority than env/git config).
4.  **Defaults:** Sensible defaults if nothing else is set.

**Key Environment Variables:**

*   `RIZZLER_PROVIDER_DEFAULT`: Which AI fam to use? (`openai`, `anthropic`, `gemini`, `bedrock`).
*   `RIZZLER_MODEL`: Specific model name (e.g., `gpt-4-turbo`, `claude-3-opus-20240229`, `gemini-pro`).
*   `RIZZLER_TIMEOUT`: How long to wait for the AI (seconds, default: 30).
*   `RIZZLER_SYSTEM_PROMPT`: Give the AI custom instructions (optional).
*   `RIZZLER_LOG_LEVEL`: How chatty should the logs be? (`error`, `warn`, `info`, `debug`, `trace`). Default: `info`.
*   `RIZZLER_LOG_FILE`: Path to write logs to (optional).
*   **API Keys (Mandatory for most providers):**
    *   `RIZZLER_OPENAI_API_KEY`
    *   `RIZZLER_CLAUDE_API_KEY`
    *   `RIZZLER_GEMINI_API_KEY`
    *   (For Bedrock, it uses standard AWS credential chain)

**Using `rizzler config`:**

```bash
# See current settings
rizzler config list

# Get a specific setting
rizzler config get ai_provider.default_model

# Set a setting (locally by default)
rizzler config set ai_provider.default_model gpt-4-turbo

# Set a setting globally
rizzler config set --global ai_provider.default_provider openai
```

## 🤖 AI & Strategies: The Brains of the Operation

So how does `rizzler` actually *decide* how to fix conflicts? And which AI overlord does it consult? Let's dive in.

**Resolution Strategies:**

`rizzler` can use different methods (strategies) to resolve conflicts. Think of them as different playbooks for tackling a merge mess.

*   **`ai` (Default & The Brainiac):** This is the main event. It sends the conflicting code snippets (plus maybe some surrounding context) to the configured Large Language Model (LLM) like GPT-4, Claude, etc. The AI analyzes the changes and attempts to generate a semantically correct merged version. This is best for complex logical conflicts.
*   **`whitespace-only` (The Neat Freak):** This is a simple, fast, rule-based strategy. If the *only* difference between the conflicting parts is whitespace (spaces, tabs, line endings), this strategy resolves the conflict by just picking one side (specifically, the `HEAD`/`--ours` version) and ignoring the whitespace changes. It's great for avoiding AI calls on purely stylistic/formatting differences.
*   *(Other potential strategies like `fallback`, `simple`, `ai-windowing`, etc., might exist depending on the specific build/configuration, often related to different ways of interacting with the AI or handling specific conflict types.)*

**How Strategies are Chosen:**

1.  **File Extension First:** If you've configured a specific strategy for a file extension (e.g., make `.md` files use `simple`), `rizzler` tries that first.fallback
2.  **Default Strategy:** If no extension-specific rule applies, it uses the default strategy (usually `ai`).
3.  **Engine Fallback:** If the chosen strategy fails or can't handle the conflict, the engine might try other available strategies (like `whitespace-only` if the `ai` strategy was chosen but the conflict was just whitespace).

**Setting Strategies:**

You can control which strategy gets used:

*   **Default Strategy:** Set globally or per-repo using `rizzler config set resolution.default_strategy <strategy_name>` or the `RIZZLER_DEFAULT_STRATEGY` environment variable.
*   **Per-Extension Strategies:** Map specific file extensions to strategies for fine-grained control:
    *   **Env Var:** `RIZZLER_EXTENSION_STRATEGY_<ext>=<strategy_name>` (e.g., `RIZZLER_EXTENSION_STRATEGY_MD=whitespace-only`)
    *   **Git Config:** `git config merge-ai-resolver.extension_strategy.<ext> <strategy_name>` (e.g., `git config merge-ai-resolver.extension_strategy.md whitespace-only`)
    *   **`.rizzler` file:** Add mappings under the `[resolution.extension_strategies]` table in your repo's `.rizzler` TOML file.

**AI Providers: Choose Your Fighter**

`rizzler` isn't locked into one AI. You've got options! Choose your fave provider via `RIZZLER_PROVIDER_DEFAULT` or the `ai_provider.default_provider` config key.

*   **OpenAI:** (`openai`)
    *   Requires `RIZZLER_OPENAI_API_KEY`.
    *   Supports models like `gpt-4`, `gpt-4-turbo`, `gpt-3.5-turbo`.
    *   Can use custom base URLs (`RIZZLER_OPENAI_BASE_URL`) for Azure, local models, etc.
*   **Anthropic (Claude):** (`anthropic`)
    *   Requires `RIZZLER_CLAUDE_API_KEY`.
    *   Supports models like `claude-3-opus-20240229`, `claude-3-sonnet-20240229`.
*   **Google (Gemini):** (`gemini`)
    *   Requires `RIZZLER_GEMINI_API_KEY`.
    *   Supports models like `gemini-pro`.
*   **AWS Bedrock:** (`bedrock`)
    *   Uses your standard AWS credentials chain (env vars, config files, IAM roles).
    *   Needs your AWS region configured.
    *   Supports various models available on Bedrock (including Claude).

Don't forget to set the specific model you want with `RIZZLER_MODEL` or `ai_provider.default_model`!

**Bonus: The `ai-fallback` Strategy 🛡️**

What if your chosen AI is down or being flaky? `rizzler` has your back with the `ai-fallback` strategy. If you set your strategy to `ai-fallback` (e.g., `rizzler config set resolution.default_strategy ai-fallback`), it will try multiple AI providers *in sequence* until one successfully resolves the conflict.

*   **How it works:** It attempts resolution with the first provider in its list. If that fails (API error, timeout, etc.), it automatically tries the next one, and so on.
*   **Default Order:** `openai,claude,gemini,bedrock`
*   **Custom Order:** You can change the sequence and which providers are included using the `RIZZLER_FALLBACK_ORDER` environment variable. Separate provider names (lowercase) with commas.
    ```bash
    # Example: Try Claude first, then OpenAI if Claude fails
    export RIZZLER_FALLBACK_ORDER="claude,openai"
    ```
*   **Availability:** Only providers that are configured correctly (e.g., have API keys set) will be included in the fallback chain.

This makes `rizzler` more resilient – if one service is having a moment, it can just pivot to the next one.

**Customizing the AI Prompt:**

Wanna give the AI some specific instructions or context? You can override the default system prompt.

*   **Env Var:** `RIZZLER_SYSTEM_PROMPT="Your custom instructions here..."`
*   **Git Config:** `rizzler config set ai_provider.system_prompt "Your custom instructions here..."`
*   **`.rizzler` file:** Set `system_prompt` under `[ai_provider]`.

This lets you fine-tune how the AI approaches the merge resolution.

## 💾 Cache = Less $$$, More Speed 💅

Aight, listen up! `rizzler` is smart with your API keys and your time. It uses a **disk cache** to remember the answers it gets from the AI. Think of it like your phone remembering Wi-Fi passwords, but for merge conflicts.

**Why You Should Care:**

*   **Saves $$:** If `rizzler` sees the *exact same conflict* it already solved, it grabs the answer from the cache instead of pinging the AI again. Cha-ching! 💸 Saves API costs.
*   **Speeds Things Up:** Cache hits are *way* faster than waiting for the AI. Gets you back to coding quicker. ⚡
*   **It Remembers!:** Since it's on disk, the cache sticks around even after you close your terminal. If you run `rizzler` later and hit the same conflict, it'll remember the fix. Persistent W.

**The Lowdown:**

*   **Where it Lives:** By default, cache files get stashed in a folder called `rizzler-cache` in your system's temp directory (like `/tmp/rizzler-cache`). You can change this spot by setting the `RIZZLER_CACHE_DIR` environment variable to your preferred path.
*   **How it Works:** `rizzler` hashes the conflict details (or file path + content) to make a key, then saves the AI's answer as a `.json` file using that key.
*   **Expiration:** Cache files have a "best before" date (TTL). After the TTL (default: **24 hours**), they get automatically deleted to keep things fresh. Old cache = irrelevant cache.
*   **Max Capacity:** It won't hoard files forever. There's a limit (default: **1000 files** per type). If it hits the cap, the oldest files get yeeted.

**Tune the Cache Vibe (Env Vars):**

*   `RIZZLER_USE_CACHE`: Turn it on (`true`, default) or off (`false`).
*   `RIZZLER_CACHE_DIR`: Pick where the cache files live.
*   `RIZZLER_CACHE_TTL_HOURS`: Set the expiry time in hours (default: `24`).
*   `RIZZLER_CACHE_MAX_ENTRIES`: Cap the number of saved files (default: `1000`).
*   `RIZZLER_CACHE_AUTO_CLEANUP`: Let `rizzler` automatically clean up old files (`true`, default) or not (`false`).

Basically, the disk cache helps `rizzler` work smarter, not harder. ✨

## 🎮 How to Play

**Automatic Mode (The Chill Way):**

Mostly, you just... don't. After running `rizzler setup` with your desired extensions, `rizzler` automatically jumps in when Git finds a merge conflict in a configured file type during `git merge`, `git pull`, etc. It does its AI thing, fixes the file, and lets Git continue. Easy peasy.

**Manual Mode (On-Demand Rizz):**

Forgot to add an extension during setup? Or just wanna run `rizzler` on a specific file *right now*? No prob!

You can use `rizzler` manually on *any* file with Git conflict markers (`<<<<<`, `=====`, `>>>>>`), regardless of whether its extension was included in the `setup` command.

```bash
# This works even if .css wasn't in your 'setup --extensions'
rizzler resolve path/to/your/conflicted_file.css
```

## 🛠️ Commands Lowdown

*   `rizzler setup`: Configures Git integration (automatic mode).
*   `rizzler config`: View or change settings.
*   `rizzler resolve <file>`: Manually resolve a specific file.
*   `rizzler doctor`: Checks if everything is set up correctly.
*   `rizzler version`: Shows the version.

## 📜 License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details. Basically, do what you want, just give credit and don't sue us. 🤙

## 🙌 Contributing

Wanna help make `rizzler` even more based? Sick! Contributions are welcome.

*   **Bugs:** Found a glitch? Report it in the Issues section.
*   **Features:** Got a killer idea? Open an issue to discuss it first.
*   **Code:** Wanna submit a Pull Request? Go for it! Make sure your code vibes with the project style and includes tests.

Check out the [CONTRIBUTING.md](CONTRIBUTING.md) (if it exists) for more detailed guidelines.

---

Stop letting merge conflicts ruin your day. Let `rizzler` handle the drama. ✨ 
