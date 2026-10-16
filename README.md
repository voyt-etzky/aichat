# AIChat: All-in-one LLM CLI Tool

[![CI](https://github.com/sigoden/aichat/actions/workflows/ci.yaml/badge.svg)](https://github.com/sigoden/aichat/actions/workflows/ci.yaml)
[![Crates](https://img.shields.io/crates/v/aichat.svg)](https://crates.io/crates/aichat)
[![Discord](https://img.shields.io/discord/1226737085453701222?label=Discord)](https://discord.gg/mr3ZZUB9hG)

AIChat is an all-in-one LLM CLI tool featuring Shell Assistant, CMD & REPL Mode, RAG, AI Tools & Agents, and More. 

## Install

### Package Managers

- **Rust Developers:** `cargo install aichat`
- **Homebrew/Linuxbrew Users:** `brew install aichat`
- **Pacman Users**: `pacman -S aichat`
- **Windows Scoop Users:** `scoop install aichat`
- **Android Termux Users:** `pkg install aichat`

### Build from Source & Direct Setup

Kompilacja i instalacja ze źródeł zapewnia dostęp do najnowszych modeli (2026), obsługi `reasoning_effort` oraz predefiniowanego polecenia `aiq` (odpowiedzi w spartańskim stylu, rola `quick`).

#### Ubuntu / Debian (x86_64 & AArch64)

1. **Wymagania:**
   ```bash
   sudo apt update && sudo apt install -y build-essential pkg-config libssl-dev git curl
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
   source "$HOME/.cargo/env"
   ```

2. **Pobranie i instalacja:**
   ```bash
   git clone https://github.com/voyt-etzky/aichat.git
   cd aichat
   cargo install --path . --force --locked
   ```
   Polecenie `cargo install --path . --force --locked` kompiluje i instaluje **oba** narzędzia: `aichat` oraz `aiq` do katalogu `~/.cargo/bin/`. Zastępuje też starszą instalację.

3. **Aby działało od strzała z każdego terminala (`aichat` oraz `aiq`):**
   Upewnij się, że `~/.cargo/bin` jest w zmiennej `$PATH`:
   ```bash
   # Dodaj do ~/.bashrc jeśli jeszcze nie masz:
   echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc
   source ~/.bashrc

   # Opcjonalnie: instalacja globalna dla całego systemu:
   sudo cp ~/.cargo/bin/aichat ~/.cargo/bin/aiq /usr/local/bin/
   ```

#### Windows (x86_64)

1. **Wymagania:**
   - Zainstaluj Rust i narzędzia C++: pobierz i uruchom instalator ze strony [rustup.rs](https://rustup.rs) (wybierz domyślną instalację z *Visual Studio C++ Build Tools*).
   - Git for Windows: [git-scm.com](https://git-scm.com).

2. **Pobranie i instalacja (w PowerShell):**
   ```powershell
   git clone https://github.com/voyt-etzky/aichat.git
   cd aichat
   cargo install --path . --force --locked
   ```
   Kompilator wygeneruje i zainstaluje pliki `aichat.exe` oraz `aiq.exe` do katalogu `%USERPROFILE%\.cargo\bin`.

3. **Aby działało od strzała w PowerShell / CMD:**
   Instalator Rust zazwyczaj automatycznie konfiguruje zmienną `PATH`. Po zakończeniu instalacji zrestartuj terminal (PowerShell) i wpisz:
   ```powershell
   aichat --version
   aiq "Podaj stolicę Francji"
   ```
   *Jeśli katalog nie został automatycznie wykryty, dodaj go jednorazowo w PowerShell:*
   ```powershell
   [Environment]::SetEnvironmentVariable("Path", $env:Path + ";$env:USERPROFILE\.cargo\bin", "User")
   ```

#### Aktualizacja istniejącej instalacji (Windows i Linux)

Jeśli masz już sklonowane repozytorium, w jego katalogu wykonaj:

```sh
git pull --ff-only
cargo install --path . --force --locked
```

`git clone` i `git pull` pobierają kod. `cargo install` kompiluje go w trybie
release i instaluje oba polecenia w katalogu Cargo. Samo `cargo build` nie
aktualizuje programów dostępnych w `PATH`. Konfiguracja aplikacji i klucze API
pozostają w dotychczasowych lokalizacjach.

Sprawdź, skąd system uruchamia programy:

```powershell
# Windows / PowerShell
Get-Command aichat, aiq | Select-Object Name, Source
```

```bash
# Linux
command -v aichat aiq
```

Jeśli na Linuxie wcześniej skopiowano programy do `/usr/local/bin`, odśwież także
te kopie poleceniem `sudo cp ~/.cargo/bin/aichat ~/.cargo/bin/aiq /usr/local/bin/`.
Na Windows starsze pliki `.exe` w katalogu bieżącym lub wcześniejszym wpisie
`PATH` również mogą przesłaniać nową instalację w `%USERPROFILE%\.cargo\bin`.

### Pre-built Binaries

Download pre-built binaries for macOS, Linux, and Windows from [GitHub Releases](https://github.com/sigoden/aichat/releases), extract them, and add the `aichat` binary to your `$PATH`.

### Temperatura i wybór providera w `aiq`

`aiq` uruchamia `aichat` z rolą `quick`. Temperatura z konfiguracji, roli lub sesji
(np. `0.3`) jest automatycznie pomijana w żądaniach do modeli GPT-5 i nowszych,
gdy model korzysta z rozumowania. Tak samo pomijany jest `top_p`. Dotyczy to
zarówno OpenAI, jak i OpenRoutera, również przy nadpisaniach `patch`.
Nie trzeba zmieniać temperatury roli `quick` dla pozostałych modeli.

Nie wszystkie nowsze modele wymagają temperatury `1`: modele obsługujące
`reasoning_effort: none` mogą przyjmować własną temperaturę przy jawnym wyłączeniu
rozumowania. Aplikacja zachowuje te parametry dla GPT-5.1 i nowszych przy `none`;
dla pierwotnego GPT-5 (także mini i nano) zawsze je pomija.
Zobacz [zgodność parametrów w dokumentacji OpenAI](https://developers.openai.com/api/docs/guides/latest-model).

Aby sprawdzić model domyślny i ustawienia roli `quick`, uruchom w PowerShell:

```powershell
aichat --info
aiq --info
```

Pole `model` ma postać `nazwa_klienta:nazwa_modelu`. Jeśli `aiq --info` nie pokazuje
pola `model`, rola dziedziczy model domyślny. Przy standardowej konfiguracji:

| Wartość `model` | Droga żądania |
| --- | --- |
| `openai:gpt-6-sol` | Klient OpenAI, domyślnie `https://api.openai.com/v1` |
| `openrouter:openai/gpt-6-sol` | Klient OpenRouter, domyślnie `https://openrouter.ai/api/v1` |

Liczy się nazwa klienta przed dwukropkiem. `openai/` po dwukropku to część nazwy
modelu w OpenRouterze. Rola `quick`, sesja, prelude lub flaga `-m` mogą zmienić wybór modelu.
Możesz go wymusić dla pojedynczego polecenia:

```powershell
aiq -m openai:gpt-6-sol "Podaj stolicę Francji"
aiq -m openrouter:openai/gpt-6-sol "Podaj stolicę Francji"
```

Nazwy klientów są konfigurowalne. Sprawdź również `api_base` właściwego wpisu
w `clients` w pliku `config.yaml` oraz zmienne `OPENAI_API_BASE` i
`OPENROUTER_API_BASE`, które mają pierwszeństwo przed plikiem. Lokalizację
konfiguracji pokaże `aichat --info` (pole `config_file`); na Windows domyślnie jest
to `%APPDATA%\aichat\config.yaml`.

Aby potwierdzić faktyczny adres żądania, włącz na jedno wywołanie log diagnostyczny:

```powershell
$env:AICHAT_LOG_LEVEL = "debug"
$env:AICHAT_LOG_PATH = "$PWD\aiq-debug.log"
aiq "Odpowiedz jednym słowem: OK"
Select-String -Path .\aiq-debug.log -Pattern 'Request https?://'
Remove-Item Env:AICHAT_LOG_LEVEL, Env:AICHAT_LOG_PATH
```

Wiersz `Request https://.../chat/completions` pokazuje adres, pod który aplikacja
wysłała żądanie. Log zawiera także treść rozmowy, więc nie udostępniaj go w całości.

## Features

### Multi-Providers

Integrate seamlessly with over 20 leading LLM providers through a unified interface. Supported providers include OpenAI, Claude, Gemini (Google AI Studio), Ollama, Groq, Azure-OpenAI, VertexAI, Bedrock, Github Models, Mistral, Deepseek, AI21, XAI Grok, Cohere, Perplexity, Cloudflare, OpenRouter, Ernie, Qianwen, Moonshot, ZhipuAI, MiniMax, Deepinfra, VoyageAI, any OpenAI-Compatible API provider.

### CMD Mode

Explore powerful command-line functionalities with AIChat's CMD mode.

![aichat-cmd](https://github.com/user-attachments/assets/6c58c549-1564-43cf-b772-e1c9fe91d19c)

### REPL Mode

Experience an interactive Chat-REPL with features like tab autocompletion, multi-line input support, history search, configurable keybindings, and custom REPL prompts.

![aichat-repl](https://github.com/user-attachments/assets/218fab08-cdae-4c3b-bcf8-39b6651f1362)

### Shell Assistant

Elevate your command-line efficiency. Describe your tasks in natural language, and let AIChat transform them into precise shell commands. AIChat intelligently adjusts to your OS and shell environment.

![aichat-execute](https://github.com/user-attachments/assets/0c77e901-0da2-4151-aefc-a2af96bbb004)

### Multi-Form Input

Accept diverse input forms such as stdin, local files and directories, and remote URLs, allowing flexibility in data handling.

| Input             | CMD                                  | REPL                             |
| ----------------- | ------------------------------------ | -------------------------------- |
| CMD               | `aichat hello`                       |                                  |
| STDIN             | `cat data.txt \| aichat`             |                                  |
| Last Reply        |                                      | `.file %%`                       |
| Local files       | `aichat -f image.png -f data.txt`    | `.file image.png data.txt`       |
| Local directories | `aichat -f dir/`                     | `.file dir/`                     |
| Remote URLs       | `aichat -f https://example.com`      | `.file https://example.com`      |
| External commands | ```aichat -f '`git diff`'```         | ```.file `git diff` ```          |
| Combine Inputs    | `aichat -f dir/ -f data.txt explain` | `.file dir/ data.txt -- explain` |

### Role

Customize roles to tailor LLM behavior, enhancing interaction efficiency and boosting productivity.

![aichat-role](https://github.com/user-attachments/assets/023df6d2-409c-40bd-ac93-4174fd72f030)

> The role consists of a prompt and model configuration.

### Session

Maintain context-aware conversations through sessions, ensuring continuity in interactions.

![aichat-session](https://github.com/user-attachments/assets/56583566-0f43-435f-95b3-730ae55df031)

> The left side uses a session, while the right side does not use a session.

### Macro

Streamline repetitive tasks by combining a series of REPL commands into a custom macro.

![aichat-macro](https://github.com/user-attachments/assets/23c2a08f-5bd7-4bf3-817c-c484aa74a651)

### RAG

Integrate external documents into your LLM conversations for more accurate and contextually relevant responses.

![aichat-rag](https://github.com/user-attachments/assets/359f0cb8-ee37-432f-a89f-96a2ebab01f6)

### Function Calling

Function calling supercharges LLMs by connecting them to external tools and data sources. This unlocks a world of possibilities, enabling LLMs to go beyond their core capabilities and tackle a wider range of tasks.

We have created a new repository [https://github.com/sigoden/llm-functions](https://github.com/sigoden/llm-functions) to help you make the most of this feature.

#### AI Tools & MCP

Integrate external tools to automate tasks, retrieve information, and perform actions directly within your workflow.

![aichat-tool](https://github.com/user-attachments/assets/7459a111-7258-4ef0-a2dd-624d0f1b4f92)

#### AI Agents (CLI version of OpenAI GPTs)

AI Agent = Instructions (Prompt) + Tools (Function Callings) + Documents (RAG).

![aichat-agent](https://github.com/user-attachments/assets/0b7e687d-e642-4e8a-b1c1-d2d9b2da2b6b)

### Local Server Capabilities

AIChat includes a lightweight built-in HTTP server for easy deployment.

```
$ aichat --serve
Chat Completions API: http://127.0.0.1:8000/v1/chat/completions
Embeddings API:       http://127.0.0.1:8000/v1/embeddings
Rerank API:           http://127.0.0.1:8000/v1/rerank
LLM Playground:       http://127.0.0.1:8000/playground
LLM Arena:            http://127.0.0.1:8000/arena?num=2
```

#### Proxy LLM APIs

The LLM Arena is a web-based platform where you can compare different LLMs side-by-side. 

Test with curl:

```sh
curl -X POST -H "Content-Type: application/json" -d '{
  "model":"claude:claude-3-5-sonnet-20240620",
  "messages":[{"role":"user","content":"hello"}], 
  "stream":true
}' http://127.0.0.1:8000/v1/chat/completions
```

#### LLM Playground

A web application to interact with supported LLMs directly from your browser.

![aichat-llm-playground](https://github.com/user-attachments/assets/aab1e124-1274-4452-b703-ef15cda55439)

#### LLM Arena

A web platform to compare different LLMs side-by-side.

![aichat-llm-arena](https://github.com/user-attachments/assets/edabba53-a1ef-4817-9153-38542ffbfec6)

## Custom Themes

AIChat supports custom dark and light themes, which highlight response text and code blocks.

![aichat-themes](https://github.com/sigoden/aichat/assets/4012553/29fa8b79-031e-405d-9caa-70d24fa0acf8)

## Documentation

- [Chat-REPL Guide](https://github.com/sigoden/aichat/wiki/Chat-REPL-Guide)
- [Command-Line Guide](https://github.com/sigoden/aichat/wiki/Command-Line-Guide)
- [Role Guide](https://github.com/sigoden/aichat/wiki/Role-Guide)
- [Macro Guide](https://github.com/sigoden/aichat/wiki/Macro-Guide)
- [RAG Guide](https://github.com/sigoden/aichat/wiki/RAG-Guide)
- [Environment Variables](https://github.com/sigoden/aichat/wiki/Environment-Variables)
- [Configuration Guide](https://github.com/sigoden/aichat/wiki/Configuration-Guide)
- [Custom Theme](https://github.com/sigoden/aichat/wiki/Custom-Theme)
- [Custom REPL Prompt](https://github.com/sigoden/aichat/wiki/Custom-REPL-Prompt)
- [FAQ](https://github.com/sigoden/aichat/wiki/FAQ)

## License

Copyright (c) 2023-2026 aichat-developers.

AIChat is made available under the terms of either the MIT License or the Apache License 2.0, at your option.

See the LICENSE-APACHE and LICENSE-MIT files for license details.
